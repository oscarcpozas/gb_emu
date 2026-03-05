use crate::cpu::Cpu;
use crate::mmu::{MemHandler, MemRead, MemWrite, Mmu};

/// Interrupt bit masks for IF/IE registers
pub const INT_VBLANK: u8 = 0x01;
pub const INT_LCD_STAT: u8 = 0x02;
pub const INT_TIMER: u8 = 0x04;
pub const INT_SERIAL: u8 = 0x08;
pub const INT_JOYPAD: u8 = 0x10;

/// Interrupt vector addresses
const VEC_VBLANK: u16 = 0x0040;
const VEC_LCD_STAT: u16 = 0x0048;
const VEC_TIMER: u16 = 0x0050;
const VEC_SERIAL: u16 = 0x0058;
const VEC_JOYPAD: u16 = 0x0060;

/// Interrupt controller — handles IF (0xFF0F) and IE (0xFFFF) registers.
pub struct Interrupt {
    /// Interrupt Flag — which interrupts are pending (0xFF0F)
    pub if_reg: u8,
    /// Interrupt Enable — which interrupts are enabled (0xFFFF)
    pub ie_reg: u8,
}

impl Interrupt {
    pub fn new() -> Self {
        Self {
            if_reg: 0x00,
            ie_reg: 0x00,
        }
    }

    /// Signal an interrupt by setting its bit in IF.
    pub fn request(&mut self, mask: u8) {
        self.if_reg |= mask;
    }

    /// Check for pending interrupts and dispatch to the CPU if IME is set.
    /// Returns the number of extra cycles consumed (20 cycles when an interrupt fires).
    pub fn dispatch(&mut self, cpu: &mut Cpu, mmu: &mut Mmu) -> usize {
        let pending = self.if_reg & self.ie_reg & 0x1F;
        if pending == 0 {
            return 0;
        }

        // An interrupt wakes the CPU from halt regardless of IME
        if cpu.halted {
            cpu.halted = false;
        }

        if !cpu.ime {
            return 0;
        }

        // Find the highest-priority interrupt (lowest bit)
        let mask = pending & pending.wrapping_neg(); // isolate lowest set bit

        // Clear the interrupt flag
        self.if_reg &= !mask;

        // Disable further interrupts while handling this one
        cpu.ime = false;

        // Push PC and jump to vector
        let pc = cpu.get_pc();
        cpu.push(mmu, pc);
        cpu.set_pc(Self::vector(mask));

        20
    }

    fn vector(mask: u8) -> u16 {
        match mask {
            INT_VBLANK => VEC_VBLANK,
            INT_LCD_STAT => VEC_LCD_STAT,
            INT_TIMER => VEC_TIMER,
            INT_SERIAL => VEC_SERIAL,
            INT_JOYPAD => VEC_JOYPAD,
            _ => unreachable!("Invalid interrupt mask: {:02x}", mask),
        }
    }
}

impl MemHandler for Interrupt {
    fn on_read(&self, addr: u16) -> MemRead {
        match addr {
            0xFF0F => MemRead::Replace(self.if_reg | 0xE0), // top 3 bits always read as 1
            0xFFFF => MemRead::Replace(self.ie_reg),
            _ => MemRead::PassThrough,
        }
    }

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite {
        match addr {
            0xFF0F => {
                self.if_reg = value & 0x1F;
                MemWrite::Block
            }
            0xFFFF => {
                self.ie_reg = value;
                MemWrite::Block
            }
            _ => MemWrite::PassThrough,
        }
    }
}
