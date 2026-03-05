use crate::io::interrupt::INT_TIMER;
use crate::mmu::{MemHandler, MemRead, MemWrite};

/// Timer & Divider registers.
///
/// 0xFF04 — DIV:  Divider register. Upper byte of a 16-bit internal counter
///                that increments every CPU cycle. Write resets it to 0.
/// 0xFF05 — TIMA: Timer counter. Increments at the frequency set by TAC.
///                On overflow triggers INT_TIMER and reloads from TMA.
/// 0xFF06 — TMA:  Timer Modulo. Loaded into TIMA on overflow.
/// 0xFF07 — TAC:  Timer Control.
///                  bit 2: timer enable (1 = running)
///                  bits 1-0: clock select
///                    00 → CPU/1024  (4096 Hz)
///                    01 → CPU/16    (262144 Hz)
///                    10 → CPU/64    (65536 Hz)
///                    11 → CPU/256   (16384 Hz)
pub struct Timer {
    /// Internal 16-bit counter. DIV = counter >> 8.
    counter: u16,
    /// TIMA — Timer Counter (0xFF05)
    tima: u8,
    /// TMA — Timer Modulo (0xFF06)
    tma: u8,
    /// TAC — Timer Control (0xFF07)
    tac: u8,
    /// Accumulated sub-cycles for TIMA
    tima_cycles: u32,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            counter: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            tima_cycles: 0,
        }
    }

    /// Advance the timer by `cycles` CPU cycles.
    /// Returns INT_TIMER bitmask if TIMA overflowed, 0 otherwise.
    pub fn update(&mut self, cycles: usize) -> u8 {
        let cycles = cycles as u32;
        let mut interrupt: u8 = 0;

        // DIV increments every CPU cycle (the full 16-bit counter does).
        // It wraps naturally on u16 overflow.
        self.counter = self.counter.wrapping_add(cycles as u16);

        // TIMA only runs when TAC bit 2 is set
        if self.tac & 0x04 != 0 {
            self.tima_cycles += cycles;
            let threshold = self.tima_period();

            while self.tima_cycles >= threshold {
                self.tima_cycles -= threshold;
                let (new_tima, overflowed) = self.tima.overflowing_add(1);
                if overflowed {
                    self.tima = self.tma;
                    interrupt = INT_TIMER;
                } else {
                    self.tima = new_tima;
                }
            }
        }

        interrupt
    }

    /// Returns the number of CPU cycles per TIMA increment based on TAC bits 0-1.
    fn tima_period(&self) -> u32 {
        match self.tac & 0x03 {
            0 => 1024,
            1 => 16,
            2 => 64,
            3 => 256,
            _ => unreachable!(),
        }
    }
}

impl MemHandler for Timer {
    fn on_read(&self, addr: u16) -> MemRead {
        match addr {
            0xFF04 => MemRead::Replace((self.counter >> 8) as u8),
            0xFF05 => MemRead::Replace(self.tima),
            0xFF06 => MemRead::Replace(self.tma),
            0xFF07 => MemRead::Replace(self.tac | 0xF8), // top 5 bits always 1
            _ => MemRead::PassThrough,
        }
    }

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite {
        match addr {
            // Writing any value to DIV resets the internal counter
            0xFF04 => {
                self.counter = 0;
                self.tima_cycles = 0;
                MemWrite::Block
            }
            0xFF05 => {
                self.tima = value;
                MemWrite::Block
            }
            0xFF06 => {
                self.tma = value;
                MemWrite::Block
            }
            0xFF07 => {
                self.tac = value & 0x07;
                MemWrite::Block
            }
            _ => MemWrite::PassThrough,
        }
    }
}
