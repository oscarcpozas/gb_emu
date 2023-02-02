pub struct Mmu {
    ram: Vec<u8>
}

impl Mmu {
    pub fn new() -> Self {
        Self {
            ram: vec![0u8; 0x10000]
        }
    }

    pub fn get8(&self, mut addr: u16) -> u8 {
        if self.is_echo_ram(addr) {
            addr -= 0x2000; // Echo RAM sector, it's same content that C000-DDFF sector
        }
        self.ram[addr as usize]
    }

    pub fn set8(&mut self, mut addr: u16, v: u8) {
        if self.is_echo_ram(addr) {
            addr -= 0x2000; // Echo RAM sector, it's same content that C000-DDFF sector
        }
        self.ram[addr as usize] = v;
    }

    pub fn get16(&self, mut addr: u16) -> u16 {
        let l = self.get8(addr) as u16;
        let h = self.get8(addr + 1) as u16;
        h << 8 | l
    }

    // TODO: Review that part, I guess the memory persists inverted
    pub fn set16(&mut self, mut addr: u16, v: u16) {
        self.set8(addr, v as u8);
        self.set8(addr + 1, (v >> 8) as u8);
    }

    // Echo ram sector contains the same data as C000-DDFF sector
    fn is_echo_ram(&self, addr: u16) -> bool {
        addr >= 0xe000 && addr <= 0x1fff
    }
}