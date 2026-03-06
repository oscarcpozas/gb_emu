use crate::mmu::{MemRead, MemWrite};
use std::fmt;

pub trait Mbc {
    fn get_name(&self) -> &str;
    fn on_read(&self, addr: u16) -> MemRead;
    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite;
}

pub fn new(code: u8, rom: Vec<u8>) -> Box<dyn Mbc> {
    match code {
        0x00 => Box::new(MbcNone::new(rom)),
        0x01 | 0x02 | 0x03 => Box::new(Mbc1::new(rom)),
        0x0f | 0x10 | 0x11 | 0x12 | 0x13 => Box::new(Mbc3::new(rom)),
        0x19 | 0x1a | 0x1b | 0x1c | 0x1d | 0x1e => Box::new(Mbc5::new(rom)),
        0x05 | 0x06 => unimplemented!("MBC2: {:02x}", code),
        0x08 | 0x09 => unimplemented!("ROM+RAM: {:02x}", code),
        0x0b | 0x0c | 0x0d => unimplemented!("MMM01: {:02x}", code),
        0xfc => unimplemented!("POCKET CAMERA"),
        0xfd => unimplemented!("BANDAI TAMA5"),
        0xfe => unimplemented!("HuC3"),
        0xff => unimplemented!("HuC1"),
        _ => unreachable!("Invalid cartridge type: {:02x}", code),
    }
}

impl fmt::Debug for dyn Mbc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

struct MbcNone {
    rom: Vec<u8>,
}

impl MbcNone {
    fn new(rom: Vec<u8>) -> Self {
        Self { rom }
    }
}

impl Mbc for MbcNone {
    fn get_name(&self) -> &str {
        "ROM Only"
    }

    fn on_read(&self, addr: u16) -> MemRead {
        if addr <= 0x7FFF {
            MemRead::Replace(self.rom[addr as usize])
        } else {
            MemRead::PassThrough
        }
    }

    fn on_write(&mut self, _addr: u16, _value: u8) -> MemWrite {
        // Writes to ROM space are silently ignored (no MBC to handle bank switching)
        MemWrite::Block
    }
}

// ---------------------------------------------------------------------------
// MBC1
// ---------------------------------------------------------------------------
// Supports up to 2MB ROM (128 banks of 16KB) and 32KB RAM (4 banks of 8KB).
//
// Memory map:
//   0x0000-0x3FFF  ROM bank 0 (fixed)
//   0x4000-0x7FFF  Switchable ROM bank
//   0xA000-0xBFFF  Switchable RAM bank (if enabled)
//
// Write registers:
//   0x0000-0x1FFF  RAM enable  (0x0A = enable, anything else = disable)
//   0x2000-0x3FFF  ROM bank low 5 bits  (0 treated as 1)
//   0x4000-0x5FFF  Upper 2 bits — RAM bank OR upper ROM bank bits
//   0x6000-0x7FFF  Banking mode  (0 = ROM mode, 1 = RAM mode)
struct Mbc1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank_lo: u8, // lower 5 bits of ROM bank
    bank_hi: u8,     // upper 2 bits (RAM bank or upper ROM bank)
    ram_enabled: bool,
    ram_mode: bool, // false = ROM banking mode, true = RAM banking mode
}

impl Mbc1 {
    fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: vec![0; 0x8000], // 32KB max
            rom_bank_lo: 1,
            bank_hi: 0,
            ram_enabled: false,
            ram_mode: false,
        }
    }

    /// Effective ROM bank for the 0x4000-0x7FFF window.
    fn rom_bank(&self) -> usize {
        let bank = if self.ram_mode {
            self.rom_bank_lo as usize
        } else {
            ((self.bank_hi as usize) << 5) | (self.rom_bank_lo as usize)
        };
        // Banks 0x00, 0x20, 0x40, 0x60 are remapped to the next bank
        match bank {
            0x00 | 0x20 | 0x40 | 0x60 => bank + 1,
            b => b,
        }
    }

    /// Effective RAM bank.
    fn ram_bank(&self) -> usize {
        if self.ram_mode {
            self.bank_hi as usize
        } else {
            0
        }
    }

    fn rom_read(&self, addr: usize) -> u8 {
        self.rom.get(addr).copied().unwrap_or(0xFF)
    }

    fn ram_read(&self, addr: usize) -> u8 {
        self.ram.get(addr).copied().unwrap_or(0xFF)
    }
}

impl Mbc for Mbc1 {
    fn get_name(&self) -> &str {
        "MBC1"
    }

    fn on_read(&self, addr: u16) -> MemRead {
        match addr {
            0x0000..=0x3FFF => MemRead::Replace(self.rom_read(addr as usize)),
            0x4000..=0x7FFF => {
                let offset = self.rom_bank() * 0x4000 + (addr as usize - 0x4000);
                MemRead::Replace(self.rom_read(offset))
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                let offset = self.ram_bank() * 0x2000 + (addr as usize - 0xA000);
                MemRead::Replace(self.ram_read(offset))
            }
            0xA000..=0xBFFF => MemRead::Replace(0xFF),
            _ => MemRead::PassThrough,
        }
    }

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x3FFF => {
                let lo = value & 0x1F;
                self.rom_bank_lo = if lo == 0 { 1 } else { lo };
            }
            0x4000..=0x5FFF => {
                self.bank_hi = value & 0x03;
            }
            0x6000..=0x7FFF => {
                self.ram_mode = value & 0x01 != 0;
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                let offset = self.ram_bank() * 0x2000 + (addr as usize - 0xA000);
                if offset < self.ram.len() {
                    self.ram[offset] = value;
                }
            }
            _ => {}
        }
        MemWrite::Block
    }
}

// ---------------------------------------------------------------------------
// MBC3
// ---------------------------------------------------------------------------
// Supports up to 2MB ROM (128 banks) and 32KB RAM (4 banks).
// Has an optional real-time clock (stubbed — reads return 0).
//
// Write registers:
//   0x0000-0x1FFF  RAM/RTC enable
//   0x2000-0x3FFF  ROM bank (7 bits, 0 treated as 1)
//   0x4000-0x5FFF  RAM bank (0-3) or RTC register select (0x08-0x0C)
//   0x6000-0x7FFF  Latch clock data (ignored)
struct Mbc3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: u8,
    ram_bank: u8,
    ram_enabled: bool,
}

impl Mbc3 {
    fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: vec![0; 0x8000],
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
        }
    }

    fn rom_read(&self, addr: usize) -> u8 {
        self.rom.get(addr).copied().unwrap_or(0xFF)
    }
}

impl Mbc for Mbc3 {
    fn get_name(&self) -> &str {
        "MBC3"
    }

    fn on_read(&self, addr: u16) -> MemRead {
        match addr {
            0x0000..=0x3FFF => MemRead::Replace(self.rom_read(addr as usize)),
            0x4000..=0x7FFF => {
                let offset = self.rom_bank as usize * 0x4000 + (addr as usize - 0x4000);
                MemRead::Replace(self.rom_read(offset))
            }
            0xA000..=0xBFFF if self.ram_enabled && self.ram_bank <= 0x03 => {
                let offset = self.ram_bank as usize * 0x2000 + (addr as usize - 0xA000);
                MemRead::Replace(self.ram.get(offset).copied().unwrap_or(0xFF))
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                // RTC registers — stub, return 0
                MemRead::Replace(0x00)
            }
            0xA000..=0xBFFF => MemRead::Replace(0xFF),
            _ => MemRead::PassThrough,
        }
    }

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x3FFF => {
                let bank = value & 0x7F;
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            0x4000..=0x5FFF => {
                self.ram_bank = value;
            }
            0x6000..=0x7FFF => {
                // RTC latch — ignored
            }
            0xA000..=0xBFFF if self.ram_enabled && self.ram_bank <= 0x03 => {
                let offset = self.ram_bank as usize * 0x2000 + (addr as usize - 0xA000);
                if offset < self.ram.len() {
                    self.ram[offset] = value;
                }
            }
            _ => {}
        }
        MemWrite::Block
    }
}

// ---------------------------------------------------------------------------
// MBC5
// ---------------------------------------------------------------------------
// Supports up to 8MB ROM (512 banks of 16KB) and 128KB RAM (16 banks of 8KB).
// Unlike MBC1, bank 0 is accessible in the 0x4000-0x7FFF window.
//
// Write registers:
//   0x0000-0x1FFF  RAM enable
//   0x2000-0x2FFF  ROM bank lower 8 bits
//   0x3000-0x3FFF  ROM bank bit 8 (upper bit)
//   0x4000-0x5FFF  RAM bank (0-0x0F)
struct Mbc5 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: u16, // 9-bit bank number (0-511)
    ram_bank: u8,
    ram_enabled: bool,
}

impl Mbc5 {
    fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: vec![0; 0x20000], // 128KB max
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
        }
    }

    fn rom_read(&self, addr: usize) -> u8 {
        self.rom.get(addr).copied().unwrap_or(0xFF)
    }
}

impl Mbc for Mbc5 {
    fn get_name(&self) -> &str {
        "MBC5"
    }

    fn on_read(&self, addr: u16) -> MemRead {
        match addr {
            0x0000..=0x3FFF => MemRead::Replace(self.rom_read(addr as usize)),
            0x4000..=0x7FFF => {
                let offset = self.rom_bank as usize * 0x4000 + (addr as usize - 0x4000);
                MemRead::Replace(self.rom_read(offset))
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                let offset = self.ram_bank as usize * 0x2000 + (addr as usize - 0xA000);
                MemRead::Replace(self.ram.get(offset).copied().unwrap_or(0xFF))
            }
            0xA000..=0xBFFF => MemRead::Replace(0xFF),
            _ => MemRead::PassThrough,
        }
    }

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x2FFF => {
                self.rom_bank = (self.rom_bank & 0x100) | value as u16;
            }
            0x3000..=0x3FFF => {
                self.rom_bank = (self.rom_bank & 0x0FF) | ((value as u16 & 0x01) << 8);
            }
            0x4000..=0x5FFF => {
                self.ram_bank = value & 0x0F;
            }
            0xA000..=0xBFFF if self.ram_enabled => {
                let offset = self.ram_bank as usize * 0x2000 + (addr as usize - 0xA000);
                if offset < self.ram.len() {
                    self.ram[offset] = value;
                }
            }
            _ => {}
        }
        MemWrite::Block
    }
}
