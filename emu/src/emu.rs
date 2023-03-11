use std::borrow::Borrow;
use std::cell::RefCell;
use std::rc::Rc;
use log::info;
use crate::cpu::Cpu;
use crate::mmu::{MemHandler, Mmu};
use crate::gui::window::GUI;
use crate::gui::hardware::Hardware;
use crate::io::mbc::cartridge::Cartridge;

pub struct Emu {
    cpu: Cpu,
    mmu: Mmu,
    cartridge: Rc<RefCell<Cartridge>>,
}

impl Emu {
    pub fn run(rom: &[u8], hardware: Hardware) {
        let mut emu = Emu::new(rom);
        emu.cartridge.borrow_mut().show_info();a

        // while hardware.get_gui_is_alive() { emu.step(); } // Emu loop is attached to GUI live
    }

    fn new(rom: &[u8]) -> Self {
        let cpu = Cpu::new();
        let mut mmu = Mmu::new();

        let mut cartridge = Rc::new(RefCell::new(Cartridge::new(rom.to_vec())));
        mmu.add_handler((0x0000, 0x7fff), Rc::new(&cartridge));

        Self {
            cpu,
            mmu,
            cartridge,
        }
    }

    fn step(&mut self) {
        let mut time = self.cpu.fetch_n_execute(&mut self.mmu);
    }
}