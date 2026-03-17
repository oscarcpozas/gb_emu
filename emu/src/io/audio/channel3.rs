/// Channel 3 — Wave channel (NR30–NR34 + wave RAM at 0xFF30–0xFF3F).
pub struct Channel3 {
    pub enabled: bool,
    dac_enabled: bool,

    // NR31: length_load (max 256)
    length_load: u8,

    // NR32: output_level [6:5]  0=mute,1=100%,2=50%,3=25%
    output_level: u8,

    // NR33/NR34: frequency + trigger + length_enable
    frequency: u16,
    length_enable: bool,

    // Wave RAM: 16 bytes holding 32 4-bit samples
    pub wave_ram: [u8; 16],

    // Internal state (timer in T-cycles)
    wave_pos: usize,
    freq_timer: i32,
    length_counter: u16,
}

impl Channel3 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            length_load: 0,
            output_level: 0,
            frequency: 0,
            length_enable: false,
            wave_ram: [0u8; 16],
            wave_pos: 0,
            freq_timer: 0,
            length_counter: 0,
        }
    }

    pub fn read_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF1A => ((self.dac_enabled as u8) << 7) | 0x7F,
            0xFF1B => 0xFF,
            0xFF1C => (self.output_level << 5) | 0x9F,
            0xFF1D => 0xFF,
            0xFF1E => ((self.length_enable as u8) << 6) | 0xBF,
            0xFF30..=0xFF3F => self.wave_ram[(addr - 0xFF30) as usize],
            _ => 0xFF,
        }
    }

    pub fn write_reg(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF1A => {
                self.dac_enabled = (val & 0x80) != 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xFF1B => {
                self.length_load = val;
                self.length_counter = 256 - val as u16;
            }
            0xFF1C => {
                self.output_level = (val >> 5) & 0x03;
            }
            0xFF1D => {
                self.frequency = (self.frequency & 0x700) | val as u16;
            }
            0xFF1E => {
                self.frequency =
                    (self.frequency & 0x00FF) | (((val & 0x07) as u16) << 8);
                self.length_enable = (val & 0x40) != 0;
                if val & 0x80 != 0 {
                    self.trigger();
                }
            }
            0xFF30..=0xFF3F => {
                self.wave_ram[(addr - 0xFF30) as usize] = val;
            }
            _ => {}
        }
    }

    fn trigger(&mut self) {
        if self.dac_enabled {
            self.enabled = true;
        }
        if self.length_counter == 0 {
            self.length_counter = 256;
        }
        self.freq_timer = (2048 - self.frequency as i32) * 2;
        self.wave_pos = 0;
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

    /// Advance by `t_cycles` (T-cycles). Returns current output sample (0–15).
    pub fn tick(&mut self, t_cycles: u32) -> u8 {
        if !self.enabled {
            return 0;
        }
        self.freq_timer -= t_cycles as i32;
        while self.freq_timer <= 0 {
            self.freq_timer += (2048 - self.frequency as i32) * 2;
            self.wave_pos = (self.wave_pos + 1) & 31;
        }

        let byte = self.wave_ram[self.wave_pos / 2];
        let nibble = if self.wave_pos & 1 == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        };

        match self.output_level {
            0 => 0,
            1 => nibble,
            2 => nibble >> 1,
            3 => nibble >> 2,
            _ => 0,
        }
    }
}
