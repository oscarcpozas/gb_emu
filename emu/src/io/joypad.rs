use crate::gui::window::GameBoyKey;
use crate::mmu::{MemHandler, MemRead, MemWrite};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Joypad register (0xFF00).
///
/// Writing selects which group to read:
///   bit 5 = 0 → select action  buttons (A, B, Select, Start)
///   bit 4 = 0 → select direction buttons (Right, Left, Up, Down)
///
/// Reading returns the state of the selected group (active-low: 0 = pressed).
///   bits 3-0: Down/Start, Up/Select, Left/B, Right/A
pub struct Joypad {
    /// Which group is selected (bits 5-4 of last write)
    select: u8,
    /// Shared key state from the GUI
    keys: Arc<Mutex<HashMap<GameBoyKey, bool>>>,
    /// Previous combined key byte — used to detect new presses for interrupt
    prev_keys: u8,
}

impl Joypad {
    pub fn new(keys: Arc<Mutex<HashMap<GameBoyKey, bool>>>) -> Self {
        Self {
            select: 0x30, // both groups de-selected by default
            keys,
            prev_keys: 0x0F,
        }
    }

    /// Returns the current low nibble (bits 3-0) for the selected group.
    /// Bits are active-low: 0 = pressed.
    fn read_keys(&self) -> u8 {
        let keys = self.keys.lock().unwrap();
        let mut nibble = 0x0F; // all released

        if self.select & 0x20 == 0 {
            // Action buttons selected
            if *keys.get(&GameBoyKey::A).unwrap_or(&false) {
                nibble &= !0x01;
            }
            if *keys.get(&GameBoyKey::B).unwrap_or(&false) {
                nibble &= !0x02;
            }
            if *keys.get(&GameBoyKey::Select).unwrap_or(&false) {
                nibble &= !0x04;
            }
            if *keys.get(&GameBoyKey::Start).unwrap_or(&false) {
                nibble &= !0x08;
            }
        }

        if self.select & 0x10 == 0 {
            // Direction buttons selected
            if *keys.get(&GameBoyKey::Right).unwrap_or(&false) {
                nibble &= !0x01;
            }
            if *keys.get(&GameBoyKey::Left).unwrap_or(&false) {
                nibble &= !0x02;
            }
            if *keys.get(&GameBoyKey::Up).unwrap_or(&false) {
                nibble &= !0x04;
            }
            if *keys.get(&GameBoyKey::Down).unwrap_or(&false) {
                nibble &= !0x08;
            }
        }

        nibble
    }

    /// Check whether any newly-pressed key should trigger a joypad interrupt.
    /// Returns true if an interrupt should be requested.
    pub fn poll_interrupt(&mut self) -> bool {
        let current = self.read_keys();
        // A falling edge on any bit (1→0 means key pressed) triggers the interrupt
        let newly_pressed = self.prev_keys & !current;
        self.prev_keys = current;
        newly_pressed != 0
    }
}

impl MemHandler for Joypad {
    fn on_read(&self, addr: u16) -> MemRead {
        if addr == 0xFF00 {
            // Upper bits: select bits (return what was written), lower nibble: key state
            let value = (self.select & 0x30) | self.read_keys() | 0xC0;
            MemRead::Replace(value)
        } else {
            MemRead::PassThrough
        }
    }

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite {
        if addr == 0xFF00 {
            self.select = value & 0x30;
            MemWrite::Block
        } else {
            MemWrite::PassThrough
        }
    }
}
