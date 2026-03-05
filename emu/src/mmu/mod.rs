pub mod handler;

pub use crate::mmu::handler::RefCellMemHandler;

/// The variants to control memory read access from the CPU.
pub enum MemRead {
    /// Replaces the value passed from the memory to the CPU.
    Replace(u8),
    /// Shows the actual value passed from the memory to the CPU.
    PassThrough,
}

/// The variants to control memory write access from the CPU.
pub enum MemWrite {
    /// Replaces the value to be written by the CPU to the memory.
    Replace(u8),
    /// Allows to write the original value from the CPU to the memory.
    PassThrough,
    /// Discard the write access from the CPU.
    Block,
}

pub trait MemHandler {
    /// The function is called when the CPU attempts to read from the memory.
    fn on_read(&self, addr: u16) -> MemRead;

    /// The function is called when the CPU attempts to write to the memory.
    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite;

    /// A version of on_write that can be called on a shared reference.
    /// This is used by the MMU to call on_write on handlers stored in an Rc.
    /// The default implementation panics, but RefCellMemHandler overrides it.
    fn on_write_shared(&self, _addr: u16, _value: u8) -> MemWrite {
        panic!("on_write_shared not implemented for this handler");
    }
}

use std::collections::HashMap;
use std::rc::Rc;

pub struct Mmu {
    ram: Vec<u8>,
    handlers: HashMap<u16, Vec<Rc<dyn MemHandler>>>,
}

impl Mmu {
    pub fn new() -> Self {
        Self {
            ram: vec![0u8; 0x10000],
            handlers: HashMap::new(),
        }
    }

    pub fn add_handler<T>(&mut self, range: (u16, u16), handler: Rc<T>)
    where
        T: MemHandler + 'static,
    {
        for i in range.0..=range.1 {
            if self.handlers.contains_key(&i) {
                self.handlers.get_mut(&i).unwrap().push(handler.clone());
            } else {
                self.handlers.insert(i, vec![handler.clone()]);
            }
        }
    }

    pub fn get8(&self, mut addr: u16) -> u8 {
        if let Some(handlers) = self.handlers.get(&addr) {
            for handler in handlers {
                match handler.on_read(addr) {
                    MemRead::Replace(value) => return value,
                    MemRead::PassThrough => {}
                }
            }
        }

        if self.is_echo_ram(addr) {
            addr -= 0x2000; // Echo RAM sector, it's same content that C000-DDFF sector
        }
        self.ram[addr as usize]
    }

    pub fn set8(&mut self, mut addr: u16, mut v: u8) {
        if let Some(handlers) = self.handlers.get(&addr) {
            for handler in handlers.iter() {
                match handler.on_write_shared(addr, v) {
                    MemWrite::Replace(value) => {
                        v = value;
                    }
                    MemWrite::PassThrough => {}
                    MemWrite::Block => {
                        return;
                    }
                }
            }
        }

        if self.is_echo_ram(addr) {
            addr -= 0x2000;
        }
        self.ram[addr as usize] = v;
    }

    pub fn get16(&self, addr: u16) -> u16 {
        let l = self.get8(addr) as u16;
        let h = self.get8(addr.wrapping_add(1)) as u16;
        h << 8 | l
    }

    pub fn set16(&mut self, addr: u16, v: u16) {
        self.set8(addr, v as u8);
        self.set8(addr.wrapping_add(1), (v >> 8) as u8);
    }

    // Echo ram sector contains the same data as C000-DDFF sector
    fn is_echo_ram(&self, addr: u16) -> bool {
        addr >= 0xe000 && addr <= 0xfdff
    }
}
