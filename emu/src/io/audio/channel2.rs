/// Duty cycle waveform table (8 steps each).
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 1, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

/// Channel 2 — Square wave (NR21–NR24).
pub struct Channel2 {
    pub enabled: bool,

    // NR21: duty[7:6], length_load[5:0]
    duty: u8,
    length_load: u8,

    // NR22: initial_volume[7:4], env_dir[3], env_period[2:0]
    initial_volume: u8,
    env_add: bool,
    env_period: u8,

    // NR23/NR24: frequency + trigger + length_enable
    frequency: u16,
    length_enable: bool,

    // Internal state (timers in T-cycles)
    duty_pos: usize,
    freq_timer: i32,
    volume: u8,
    env_timer: u8,
    length_counter: u16,
}

impl Channel2 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            duty: 0,
            length_load: 0,
            initial_volume: 0,
            env_add: false,
            env_period: 0,
            frequency: 0,
            length_enable: false,
            duty_pos: 0,
            freq_timer: 0,
            volume: 0,
            env_timer: 0,
            length_counter: 0,
        }
    }

    pub fn read_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF16 => (self.duty << 6) | 0x3F,
            0xFF17 => {
                (self.initial_volume << 4)
                    | ((self.env_add as u8) << 3)
                    | self.env_period
            }
            0xFF18 => 0xFF,
            0xFF19 => ((self.length_enable as u8) << 6) | 0xBF,
            _ => 0xFF,
        }
    }

    pub fn write_reg(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF16 => {
                self.duty = (val >> 6) & 0x03;
                self.length_load = val & 0x3F;
                self.length_counter = 64 - self.length_load as u16;
            }
            0xFF17 => {
                self.initial_volume = val >> 4;
                self.env_add = (val & 0x08) != 0;
                self.env_period = val & 0x07;
                if val & 0xF8 == 0 {
                    self.enabled = false;
                }
            }
            0xFF18 => {
                self.frequency = (self.frequency & 0x700) | val as u16;
            }
            0xFF19 => {
                self.frequency =
                    (self.frequency & 0x00FF) | (((val & 0x07) as u16) << 8);
                self.length_enable = (val & 0x40) != 0;
                if val & 0x80 != 0 {
                    self.trigger();
                }
            }
            _ => {}
        }
    }

    fn trigger(&mut self) {
        self.enabled = true;
        if self.length_counter == 0 {
            self.length_counter = 64;
        }
        self.freq_timer = (2048 - self.frequency as i32) * 4;
        self.volume = self.initial_volume;
        self.env_timer = self.env_period;
    }

    /// Called at every length step (256 Hz) by the frame sequencer.
    pub fn clock_length(&mut self) {
        if self.length_enable && self.length_counter > 0 {
            self.length_counter -= 1;
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    /// Called at every envelope step (64 Hz) by the frame sequencer.
    pub fn clock_envelope(&mut self) {
        if self.env_period == 0 {
            return;
        }
        if self.env_timer > 0 {
            self.env_timer -= 1;
        }
        if self.env_timer == 0 {
            self.env_timer = self.env_period;
            if self.env_add && self.volume < 15 {
                self.volume += 1;
            } else if !self.env_add && self.volume > 0 {
                self.volume -= 1;
            }
        }
    }

    /// Advance frequency timer by `t_cycles` (T-cycles). Returns current output (0 or volume).
    pub fn tick(&mut self, t_cycles: u32) -> u8 {
        if !self.enabled {
            return 0;
        }
        self.freq_timer -= t_cycles as i32;
        while self.freq_timer <= 0 {
            self.freq_timer += (2048 - self.frequency as i32) * 4;
            self.duty_pos = (self.duty_pos + 1) & 7;
        }
        if DUTY_TABLE[self.duty as usize][self.duty_pos] != 0 {
            self.volume
        } else {
            0
        }
    }
}
