use crate::cpu::Cpu;
use crate::gui::hardware::Hardware;
use crate::io::boot::BootRom;
use crate::io::gpu::ppu::Ppu;
use crate::io::mbc::cartridge::Cartridge;
use crate::mmu::{Mmu, RefCellMemHandler};
use log::info;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread::sleep;
use std::time::{Duration, Instant};

// Game Boy CPU clock speed: 4.194304 MHz
const CPU_CLOCK_HZ: u64 = 4_194_304;
// Target frame rate: 60 FPS
const TARGET_FPS: u64 = 60;
// CPU cycles per frame
const CYCLES_PER_FRAME: usize = (CPU_CLOCK_HZ / TARGET_FPS) as usize;

pub struct Emu {
    cpu: Cpu,
    mmu: Mmu,
    cartridge: Rc<RefCell<Cartridge>>,
    boot_rom: Rc<RefCell<BootRom>>,
    ppu: Rc<RefCell<Ppu>>,
    cycles: usize,
}

impl Emu {
    pub fn run(rom: &[u8], hardware: Hardware) {
        let mut emu = Emu::new(rom, hardware.get_vram());
        emu.cartridge.borrow().show_info();

        info!("Starting emulation loop");

        // Main emulation loop
        while hardware.get_gui_is_alive() {
            let frame_start = Instant::now();

            // Run one frame worth of CPU cycles
            emu.run_frame();

            // Calculate how long to sleep to maintain target frame rate
            let frame_time = frame_start.elapsed();
            let target_frame_time = Duration::from_micros(1_000_000 / TARGET_FPS);

            if frame_time < target_frame_time {
                sleep(target_frame_time - frame_time);
            }
        }
    }

    fn new(rom: &[u8], vram_buffer: std::sync::Arc<std::sync::Mutex<Vec<u32>>>) -> Self {
        let cpu = Cpu::new();
        let mut mmu = Mmu::new();

        // Create the cartridge
        let cartridge = Rc::new(RefCell::new(Cartridge::new(rom.to_vec())));

        // Create the boot ROM
        let boot_rom = Rc::new(RefCell::new(BootRom::new()));

        // Create the PPU
        let ppu = Rc::new(RefCell::new(Ppu::new(vram_buffer)));

        // Create memory handlers
        let cartridge_handler = Rc::new(RefCellMemHandler::new(cartridge.clone()));
        let boot_rom_handler = Rc::new(RefCellMemHandler::new(boot_rom.clone()));
        let ppu_handler = Rc::new(RefCellMemHandler::new(ppu.clone()));

        // Add memory handlers
        mmu.add_handler((0x0000, 0x7FFF), cartridge_handler.clone());
        mmu.add_handler((0x0000, 0x00FF), boot_rom_handler.clone());
        mmu.add_handler((0xFF50, 0xFF50), boot_rom_handler.clone());
        mmu.add_handler((0x8000, 0x9FFF), ppu_handler.clone());
        mmu.add_handler((0xFE00, 0xFE9F), ppu_handler.clone());
        mmu.add_handler((0xFF40, 0xFF4B), ppu_handler.clone());

        Self {
            cpu,
            mmu,
            cartridge,
            boot_rom,
            ppu,
            cycles: 0,
        }
    }

    fn run_frame(&mut self) {
        let mut cycles_this_frame = 0;

        // Run CPU cycles until we've reached the target for this frame
        while cycles_this_frame < CYCLES_PER_FRAME {
            // Execute one CPU instruction
            let cycles = self.step();
            cycles_this_frame += cycles;
            self.cycles += cycles;

            // Update PPU
            self.ppu.borrow_mut().update(cycles);
        }
    }

    fn step(&mut self) -> usize {
        // Execute one CPU instruction and return the number of cycles it took
        self.cpu.fetch_n_execute(&mut self.mmu)
    }
}
