use std::fmt;

pub enum MbcType {
    None(MbcNone),
}

impl fmt::Debug for MbcType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ctype = match self {
            MbcType::None(_) => "ROM Only",
        };
        write!(f, "{}", ctype)
    }
}

struct MbcNone {
    rom: Vec<u8>,
}

impl MbcNone {
    fn new(rom: Vec<u8>) -> Self {
        Self {
            rom
        }
    }

}

impl MbcType {
    pub fn new(code: u8, rom: Vec<u8>) -> Self {
        match code {
            0x00 => MbcType::None(MbcNone::new(rom)),  // ROM Only
            0x01 | 0x02 | 0x03 => unimplemented!("MBC1: {:02x}", code),  // MBC1 | MBC1+RAM | MBC1+RAM+BATTERY
            0x05 | 0x06 => unimplemented!("MBC2: {:02x}", code),  // MBC2 | MBC2+BATTERY
            0x08 | 0x09 => unimplemented!("ROM+RAM: {:02x}", code),  // ROM+RAM | ROM+RAM+BATTERY
            0x0b | 0x0c | 0x0d => unimplemented!("MMM01: {:02x}", code),  // MMM01 | MMM01+RAM | MMM01+RAM+BATTERY
            0x0f | 0x10 | 0x11 | 0x12 | 0x13 => unimplemented!("MBC3: {:02x}", code),  // MBC3+TIMER+BATTERY | MBC3+TIMER+RAM+BATTERY | MBC3 | MBC3+RAM | MBC3+RAM+BATTERY
            0x19 | 0x1a | 0x1b | 0x1c | 0x1d | 0x1e => unimplemented!("MBC5: {:02x}", code),  // MBC5 | MBC5+RAM | MBC5+RAM+BATTERY | MBC5+RUMBLE | MBC5+RUMBLE+RAM | MBC5+RUMBLE+RAM+BATTERY
            0xfc => unimplemented!("POCKET CAMERA"), // POCKET CAMERA
            0xfd => unimplemented!("BANDAI TAMA5"),  // BANDAI TAMA5
            0xfe => unimplemented!("HuC3"),  // HuC3
            0xff => unimplemented!("HuC1"),  // HuC1+RAM+BATTERY
            _ => unreachable!("Invalid cartridge type: {:02x}", code),
        }
    }
}