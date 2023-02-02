use std::fmt;
use log::info;
use crate::io::mbc::mbc::MbcType;

pub struct Cartridge {
    title: String,
    cgb: Cgb,
    sgb: bool,
    mbc: MbcType,
    rom_size: u8,
    ram_size: u8,
}

#[derive(Debug)]
pub enum Cgb {
    Cgb,
    CgbOnly,
    Unknown,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            title: parse_title(&rom[0x134..0x144]),
            cgb: parse_cgb(&rom[0x143]),
            sgb: rom[0x146] == 0x03,
            mbc: MbcType::new(rom[0x147], rom.clone()),
            rom_size: rom[0x148],
            ram_size: rom[0x149],
        }
    }
}

impl fmt::Display for Cartridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rom_size = match self.rom_size {
            0x00 => "32KByte (no ROM banking)",
            0x01 => "64KByte (4 banks)",
            0x02 => "128KByte (8 banks)",
            0x03 => "256KByte (16 banks)",
            0x04 => "512KByte (32 banks)",
            0x05 => "1MByte (64 banks)  - only 63 banks used by Mbc1",
            0x06 => "2MByte (128 banks) - only 125 banks used by Mbc1",
            0x07 => "4MByte (256 banks)",
            0x52 => "1.1MByte (72 banks)",
            0x53 => "1.2MByte (80 banks)",
            0x54 => "1.5MByte (96 banks)",
            _ => "Unknown",
        };
        let ram_size = match self.ram_size {
            0x00 => "None",
            0x01 => "2 KBytes",
            0x02 => "8 Kbytes",
            0x03 => "32 KBytes (4 banks of 8KBytes each)",
            _ => "Unknown",
        };

        write!(f,
               "\n ROM Title: {} \n CGB: {:?} \n SGB: {} \n Cartridge type: {:?} \n ROM size: {} \n RAM size: {}",
               self.title,
               self.cgb,
               self.sgb,
               self.mbc,
               rom_size,
               ram_size
        )
    }
}

fn parse_title(vec: &[u8]) -> String {
    String::from_utf8_lossy(&vec).to_string()
}

fn parse_cgb(cgb_flag: &u8) -> Cgb {
    match cgb_flag {
        0x80 => Cgb::Cgb,
        0xC0 => Cgb::CgbOnly,
        _ => Cgb::Unknown
    }
}