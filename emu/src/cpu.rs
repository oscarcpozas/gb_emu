use crate::mmu::Mmu;
use crate::instr;

pub struct Cpu {
    a: u8, // accumulator
    f: u8, // flags register
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16
}

impl Cpu {

    pub fn new() -> Self {
        Self {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            pc: 0,
            sp: 0
        }
    }

    /*
    Execute a single instruction.
    Function fetches an instruction code from the memory, decodes it, and updates the CPU/memory state accordingly.
    */
    pub fn fetch_n_execute(&mut self, mmu: &mut Mmu) -> usize {
        let (inst, args) = self.fetch_op_from_mem(mmu);
        let (time, size) = instr::decode(inst, args, self, mmu);

        self.pc = self.get_pc().wrapping_add(args); // Update the value of the program counter.

        time
    }

    /*
    Fetch an instruction from the memory.
    Program counter define the address of the instruction in the memory is incremented by the size of the instruction.
    */
    fn fetch_op_from_mem(&self, mmu: &mut Mmu) -> (u16, u16) {
        let code = mmu.get8(self.get_pc());
        if code == 0xcb {
            let sb = mmu.get8(self.get_pc() + 1);
            (0xcb00 | sb as u16, 2)
        } else {
            (code as u16, 1)
        }
    }

    /// Switch the CPU state to halting.
    pub fn halt(&mut self) {
        // TODO: self.halt = true;
    }

    /// Disable interrupts to this CPU.
    pub fn disable_interrupt(&mut self) {
        // TODO: self.ime = false;
    }

    /// Enable interrupts to this CPU.
    pub fn enable_interrupt(&mut self) {
        //TODO: self.ime = true;
    }

    /// Stop the CPU.
    pub fn stop(&self) {
        // TODO: Stop.
    }

    /// Gets the value of `z` flag in the flag register.
    pub fn get_zf(&self) -> bool { self.f & 0x80 == 0x80 }

    /// Updates the value of `z` flag in the flag register.
    pub fn set_zf(&mut self, v: bool) {
        if v {
            self.f = self.f | 0x80
        } else {
            self.f = self.f & !0x80
        }
    }

    /// Gets the value of `n` flag in the flag register.
    pub fn get_nf(&self) -> bool { self.f & 0x40 == 0x40 }

    /// Updates the value of `n` flag in the flag register.
    pub fn set_nf(&mut self, v: bool) {
        if v {
            self.f = self.f | 0x40
        } else {
            self.f = self.f & !0x40
        }
    }

    /// Gets the value of `h` flag in the flag register.
    pub fn get_hf(&self) -> bool {
        self.f & 0x20 == 0x20
    }

    /// Updates the value of `h` flag in the flag register.
    pub fn set_hf(&mut self, v: bool) {
        if v {
            self.f = self.f | 0x20
        } else {
            self.f = self.f & !0x20
        }
    }

    /// Gets the value of `c` flag in the flag register.
    pub fn get_cf(&self) -> bool {
        self.f & 0x10 == 0x10
    }

    /// Updates the value of `c` flag in the flag register.
    pub fn set_cf(&mut self, v: bool) {
        if v {
            self.f = self.f | 0x10
        } else {
            self.f = self.f & !0x10
        }
    }

    /// Updates the value of `a` register.
    pub fn set_a(&mut self, v: u8) {
        self.a = v
    }

    /// Updates the value of `b` register.
    pub fn set_b(&mut self, v: u8) {
        self.b = v
    }

    /// Updates the value of `c` register.
    pub fn set_c(&mut self, v: u8) {
        self.c = v
    }

    /// Updates the value of `d` register.
    pub fn set_d(&mut self, v: u8) {
        self.d = v
    }

    /// Updates the value of `e` register.
    pub fn set_e(&mut self, v: u8) {
        self.e = v
    }

    /// Updates the value of `h` register.
    pub fn set_h(&mut self, v: u8) {
        self.h = v
    }

    /// Updates the value of `l` register.
    pub fn set_l(&mut self, v: u8) {
        self.l = v
    }

    /// Updates the value of `a` and `f` register as a single 16-bit register.
    pub fn set_af(&mut self, v: u16) {
        self.a = (v >> 8) as u8;
        self.f = (v & 0xf0) as u8;
    }

    /// Updates the value of `b` and `c` register as a single 16-bit register.
    pub fn set_bc(&mut self, v: u16) {
        self.b = (v >> 8) as u8;
        self.c = v as u8;
    }

    /// Updates the value of `d` and `e` register as a single 16-bit register
    pub fn set_de(&mut self, v: u16) {
        self.d = (v >> 8) as u8;
        self.e = v as u8;
    }

    /// Updates the value of `h` and `l` register as a single 16-bit register.
    pub fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = v as u8;
    }

    /// Gets the value of `a` register.
    pub fn get_a(&self) -> u8 { self.a }

    /// Gets the value of `b` register.
    pub fn get_b(&self) -> u8 { self.b }

    /// Gets the value of `c` register.
    pub fn get_c(&self) -> u8 { self.c }

    /// Gets the value of `d` register.
    pub fn get_d(&self) -> u8 { self.d }

    /// Gets the value of `e` register.
    pub fn get_e(&self) -> u8 { self.e }

    /// Gets the value of `h` register.
    pub fn get_h(&self) -> u8 { self.h }

    /// Gets the value of `l` register.
    pub fn get_l(&self) -> u8 { self.l }

    /// Gets the value of `a` and `f` register as a single 16-bit register.
    pub fn get_af(&self) -> u16 { (self.a as u16) << 8 | self.f as u16 }

    /// Gets the value of `b` and `c` register as a single 16-bit register.
    pub fn get_bc(&self) -> u16 { (self.b as u16) << 8 | self.c as u16 }

    /// Gets the value of `d` and `e` register as a single 16-bit register.
    pub fn get_de(&self) -> u16 { (self.d as u16) << 8 | self.e as u16 }

    /// Gets the value of `h` and `l` register as a single 16-bit register.
    pub fn get_hl(&self) -> u16 { (self.h as u16) << 8 | self.l as u16 }

    /// Gets the value of the program counter.
    pub fn get_pc(&self) -> u16 { self.pc }

    /// Updates the value of the program counter.
    pub fn set_pc(&mut self, v: u16) { self.pc = v }

    /// Gets the value of the stack pointer register.
    pub fn get_sp(&self) -> u16 { self.sp }

    /// Updates the value of the stack pointer register.
    pub fn set_sp(&mut self, v: u16) {
        self.sp = v
    }

    /// Pushes a 16-bit value to the stack, updating the stack pointer register.
    pub fn push(&mut self, mmu: &mut Mmu, v: u16) {
        let pointer = self.get_sp().wrapping_sub(2);
        self.set_sp(pointer);
        mmu.set16(pointer, v)
    }

    /// Pops a 16-bit value from the stack, updating the stack pointer register.
    pub fn pop(&mut self, mmu: &mut Mmu) -> u16 {
        let pointer = self.get_sp();
        self.set_sp(self.get_sp().wrapping_add(2));
        mmu.get16(pointer)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fetch_first_op_from_mem() {
        let mut mmu = Mmu::new();
        let mut cpu = Cpu::new();

        let (inst, args) = cpu.fetch_op_from_mem(&mut mmu);

        assert_eq!(inst, 0x0000);
        assert_eq!(args, 1);
    }

    #[test]
    fn test_op_0x0000() {
        let mut mmu = Mmu::new();
        let mut cpu = Cpu::new();

        let time = cpu.fetch_n_execute(&mut mmu);

        assert_eq!(time, 4);
        assert_eq!(cpu.get_pc(), 1);
    }

    #[test]
    fn test_op_0x0001() {
        let mut mmu = Mmu::new();
        let mut cpu = Cpu::new();

        cpu.set_bc(0x1234);
        let time = cpu.fetch_n_execute(&mut mmu);

        assert_eq!(time, 12);
        assert_eq!(cpu.get_pc(), 1);
    }
}