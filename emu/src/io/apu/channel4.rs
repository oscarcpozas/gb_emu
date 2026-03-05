/// Noise frequency divisor table (in T-cycles).
const DIVISOR_TABLE: [u32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

/// Channel 4 — Noise/LFSR (NR41–NR44).
pub struct Channel4 {
    pub enabled: bool,

    // NR41: length_load[5:0]
    length_load: u8,

    // NR42: initial_volume[7:4], env_dir[3], env_period[2:0]
    initial_volume: u8,
    env_add: bool,
    env_period: u8,

    // NR43: clock_shift[7:4], width_mode[3], divisor_code[2:0]
    clock_shift: u8,
    width_mode: bool, // true = 7-bit LFSR, false = 15-bit
    divisor_code: u8,

    // NR44: trigger, length_enable
    length_enable: bool,

    // Internal state (timer in T-cycles)
    lfsr: u16,
    freq_timer: i32,
    volume: u8,
    env_timer: u8,
    length_counter: u16,
}

impl Channel4 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_load: 0,
            initial_volume: 0,
            env_add: false,
            env_period: 0,
            clock_shift: 0,
            width_mode: false,
            divisor_code: 0,
            length_enable: false,
            lfsr: 0x7FFF,
            freq_timer: 0,
            volume: 0,
            env_timer: 0,
            length_counter: 0,
        }
    }

    pub fn read_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF20 => 0xFF,
            0xFF21 => {
                (self.initial_volume << 4)
                    | ((self.env_add as u8) << 3)
                    | self.env_period
            }
            0xFF22 => {
                (self.clock_shift << 4)
                    | ((self.width_mode as u8) << 3)
                    | self.divisor_code
            }
            0xFF23 => ((self.length_enable as u8) << 6) | 0xBF,
            _ => 0xFF,
        }
    }

    pub fn write_reg(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF20 => {
                self.length_load = val & 0x3F;
                self.length_counter = 64 - self.length_load as u16;
            }
            0xFF21 => {
                self.initial_volume = val >> 4;
                self.env_add = (val & 0x08) != 0;
                self.env_period = val & 0x07;
                if val & 0xF8 == 0 {
                    self.enabled = false;
                }
            }
            0xFF22 => {
                self.clock_shift = val >> 4;
                self.width_mode = (val & 0x08) != 0;
                self.divisor_code = val & 0x07;
            }
            0xFF23 => {
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
        self.lfsr = 0x7FFF;
        self.freq_timer =
            (DIVISOR_TABLE[self.divisor_code as usize] << self.clock_shift) as i32;
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

    /// Advance LFSR by `t_cycles` (T-cycles). Returns current output (0 or volume).
    pub fn tick(&mut self, t_cycles: u32) -> u8 {
        if !self.enabled {
            return 0;
        }
        self.freq_timer -= t_cycles as i32;
        while self.freq_timer <= 0 {
            let period =
                (DIVISOR_TABLE[self.divisor_code as usize] << self.clock_shift) as i32;
            self.freq_timer += period;

            // Clock the LFSR: XOR bits 0 and 1, shift right, place result in bit 14
            let xor_bit = (self.lfsr & 1) ^ ((self.lfsr >> 1) & 1);
            self.lfsr = (self.lfsr >> 1) | (xor_bit << 14);
            if self.width_mode {
                // Also place XOR result in bit 6 for 7-bit mode
                self.lfsr = (self.lfsr & !(1 << 6)) | (xor_bit << 6);
            }
        }

        // Output: bit 0 inverted (0 = high output)
        if self.lfsr & 1 == 0 {
            self.volume
        } else {
            0
        }
    }
}
