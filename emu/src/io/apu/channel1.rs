/// Duty cycle waveform table (8 steps each).
/// Patterns: 12.5%, 25%, 50%, 75%.
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 1, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

/// Channel 1 — Square wave with frequency sweep (NR10–NR14).
pub struct Channel1 {
    pub enabled: bool,

    // NR10: sweep
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,

    // NR11: duty[7:6], length_load[5:0]
    duty: u8,
    length_load: u8,

    // NR12: initial_volume[7:4], env_dir[3], env_period[2:0]
    initial_volume: u8,
    env_add: bool,
    env_period: u8,

    // NR13/NR14: frequency + trigger + length_enable
    frequency: u16,
    length_enable: bool,

    // Internal state (all timers in T-cycles)
    duty_pos: usize,
    freq_timer: i32,
    volume: u8,
    env_timer: u8,
    length_counter: u16,
    shadow_freq: u16,
    sweep_timer: u8,
    sweep_enabled: bool,
}

impl Channel1 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
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
            shadow_freq: 0,
            sweep_timer: 0,
            sweep_enabled: false,
        }
    }

    pub fn read_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF10 => {
                ((self.sweep_period & 0x07) << 4)
                    | ((self.sweep_negate as u8) << 3)
                    | (self.sweep_shift & 0x07)
                    | 0x80
            }
            0xFF11 => (self.duty << 6) | 0x3F,
            0xFF12 => {
                (self.initial_volume << 4)
                    | ((self.env_add as u8) << 3)
                    | self.env_period
            }
            0xFF13 => 0xFF,
            0xFF14 => ((self.length_enable as u8) << 6) | 0xBF,
            _ => 0xFF,
        }
    }

    pub fn write_reg(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF10 => {
                self.sweep_period = (val >> 4) & 0x07;
                self.sweep_negate = (val & 0x08) != 0;
                self.sweep_shift = val & 0x07;
            }
            0xFF11 => {
                self.duty = (val >> 6) & 0x03;
                self.length_load = val & 0x3F;
                self.length_counter = 64 - self.length_load as u16;
            }
            0xFF12 => {
                self.initial_volume = val >> 4;
                self.env_add = (val & 0x08) != 0;
                self.env_period = val & 0x07;
                if val & 0xF8 == 0 {
                    self.enabled = false;
                }
            }
            0xFF13 => {
                self.frequency = (self.frequency & 0x700) | val as u16;
            }
            0xFF14 => {
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

    fn calculate_sweep_freq(&self) -> u16 {
        let delta = self.shadow_freq >> self.sweep_shift;
        if self.sweep_negate {
            self.shadow_freq.wrapping_sub(delta)
        } else {
            self.shadow_freq.wrapping_add(delta)
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
        self.shadow_freq = self.frequency;
        self.sweep_timer = if self.sweep_period == 0 { 8 } else { self.sweep_period };
        self.sweep_enabled = self.sweep_period != 0 || self.sweep_shift != 0;
        if self.sweep_shift != 0 {
            let new_freq = self.calculate_sweep_freq();
            if new_freq > 2047 {
                self.enabled = false;
            }
        }
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

    /// Called at every sweep step (128 Hz) by the frame sequencer.
    pub fn clock_sweep(&mut self) {
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }
        if self.sweep_timer == 0 {
            self.sweep_timer = if self.sweep_period == 0 { 8 } else { self.sweep_period };
            if self.sweep_enabled && self.sweep_period != 0 {
                let new_freq = self.calculate_sweep_freq();
                if new_freq > 2047 {
                    self.enabled = false;
                } else if self.sweep_shift != 0 {
                    self.frequency = new_freq;
                    self.shadow_freq = new_freq;
                    // Second overflow check
                    let check = self.calculate_sweep_freq();
                    if check > 2047 {
                        self.enabled = false;
                    }
                }
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
