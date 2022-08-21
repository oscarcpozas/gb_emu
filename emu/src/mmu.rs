pub struct Mmu {
    ram: Vec<u8>
}

impl Mmu {

    // Echo ram sector contains the same data as C000-DDFF sector
    fn is_echo_ram(addr: u16) -> bool {
        addr >= 0x0000 && addr <= 0x1fff
    }

    pub fn get8(&self, mut addr: u16) -> u8 {
        if is_echo_ram(addr) {
            addr -= 0x2000;
        }
        self.ram[addr as usize]
    }

    pub fn set8(&self, mut addr: u16, v: u8) {
        // Echo RAM sector, it's same content that C000-DDFF sector
        if is_echo_ram(addr) {
            addr -= 0x2000;
        }
        self.ram[addr as usize] = v;
    }

    pub fn get16(&self, mut addr: u16) -> u16 {
        let f = self.get8(addr) as u16;
        let s = self.get8(addr + 1);
        f << 8 | s
    }

    // TODO: Review that part, I guess the memory persists inverted
    pub fn set16(&self, mut addr: u16, v: u16) {
        self.set8(addr, v as u8);
        self.set8(addr + 1, (v >> 8) as u8);
    }
}