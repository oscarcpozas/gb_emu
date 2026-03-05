use crate::cpu::Cpu;
use crate::gui::hardware::Hardware;
use crate::io::boot::BootRom;
use crate::io::gpu::ppu::Ppu;
use crate::io::interrupt::{Interrupt, INT_JOYPAD};
use crate::io::joypad::Joypad;
use crate::io::mbc::cartridge::Cartridge;
use crate::io::timer::Timer;
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
    interrupt: Rc<RefCell<Interrupt>>,
    joypad: Rc<RefCell<Joypad>>,
    timer: Rc<RefCell<Timer>>,
    cycles: usize,
}

impl Emu {
    pub fn run(rom: Vec<u8>, hardware: Hardware) {
        let mut emu = Emu::new(rom, hardware.get_vram(), hardware.get_keys_states());
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

    fn new(
        rom: Vec<u8>,
        vram_buffer: std::sync::Arc<std::sync::Mutex<Vec<u32>>>,
        keys: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<crate::gui::window::GameBoyKey, bool>>>,
    ) -> Self {
        let cpu = Cpu::new();
        let mut mmu = Mmu::new();

        let cartridge = Rc::new(RefCell::new(Cartridge::new(rom)));
        let boot_rom = Rc::new(RefCell::new(BootRom::new()));
        let ppu = Rc::new(RefCell::new(Ppu::new(vram_buffer)));
        let interrupt = Rc::new(RefCell::new(Interrupt::new()));
        let joypad = Rc::new(RefCell::new(Joypad::new(keys)));
        let timer = Rc::new(RefCell::new(Timer::new()));

        let cartridge_handler = Rc::new(RefCellMemHandler::new(cartridge.clone()));
        let boot_rom_handler = Rc::new(RefCellMemHandler::new(boot_rom.clone()));
        let ppu_handler = Rc::new(RefCellMemHandler::new(ppu.clone()));
        let interrupt_handler = Rc::new(RefCellMemHandler::new(interrupt.clone()));
        let joypad_handler = Rc::new(RefCellMemHandler::new(joypad.clone()));
        let timer_handler = Rc::new(RefCellMemHandler::new(timer.clone()));

        // Boot ROM must be registered BEFORE cartridge so it takes priority for 0x0000-0x00FF.
        // When boot ROM is inactive it returns PassThrough, falling through to cartridge.
        mmu.add_handler((0x0000, 0x00FF), boot_rom_handler.clone());
        mmu.add_handler((0xFF50, 0xFF50), boot_rom_handler.clone());
        mmu.add_handler((0x0000, 0x7FFF), cartridge_handler.clone());
        mmu.add_handler((0x8000, 0x9FFF), ppu_handler.clone());
        mmu.add_handler((0xFE00, 0xFE9F), ppu_handler.clone());
        mmu.add_handler((0xFF40, 0xFF4B), ppu_handler.clone());
        mmu.add_handler((0xFF46, 0xFF46), ppu_handler.clone());
        mmu.add_handler((0xFF0F, 0xFF0F), interrupt_handler.clone());
        mmu.add_handler((0xFFFF, 0xFFFF), interrupt_handler.clone());
        mmu.add_handler((0xFF00, 0xFF00), joypad_handler.clone());
        mmu.add_handler((0xFF04, 0xFF07), timer_handler.clone());

        Self {
            cpu,
            mmu,
            cartridge,
            boot_rom,
            ppu,
            interrupt,
            joypad,
            timer,
            cycles: 0,
        }
    }

    fn run_frame(&mut self) {
        let mut cycles_this_frame = 0;

        while cycles_this_frame < CYCLES_PER_FRAME {
            let cycles = self.step();
            cycles_this_frame += cycles;
            self.cycles += cycles;
        }
    }

    fn step(&mut self) -> usize {
        // Debug: log first 60 instructions
        if self.cycles < 60 * 24 {
            log::trace!(
                "PC={:04X} SP={:04X} A={:02X} F={:02X} BC={:04X} DE={:04X} HL={:04X} | op={:02X}",
                self.cpu.get_pc(), self.cpu.get_sp(),
                self.cpu.get_a(), self.cpu.get_af() as u8,
                self.cpu.get_bc(), self.cpu.get_de(), self.cpu.get_hl(),
                self.mmu.get8(self.cpu.get_pc()),
            );
        }

        // Execute pending OAM DMA if requested (triggered by write to 0xFF46).
        let pending_dma = self.ppu.borrow_mut().pending_dma.take();
        if let Some(page) = pending_dma {
            let src_addr = (page as u16) << 8;
            let src: Vec<u8> = (0..0xA0u16).map(|i| self.mmu.get8(src_addr + i)).collect();
            self.ppu.borrow_mut().execute_dma(&src);
        }

        // Execute one CPU instruction
        let cycles = self.cpu.fetch_n_execute(&mut self.mmu);

        // Update PPU
        let ppu_interrupts = self.ppu.borrow_mut().update(cycles);
        if ppu_interrupts != 0 {
            self.interrupt.borrow_mut().request(ppu_interrupts);
        }

        // Update timer
        let timer_interrupt = self.timer.borrow_mut().update(cycles);
        if timer_interrupt != 0 {
            self.interrupt.borrow_mut().request(timer_interrupt);
        }

        // Poll joypad for newly-pressed keys
        if self.joypad.borrow_mut().poll_interrupt() {
            self.interrupt.borrow_mut().request(INT_JOYPAD);
        }

        // Dispatch pending interrupts to the CPU
        let int_cycles = self.interrupt.borrow_mut().dispatch(&mut self.cpu, &mut self.mmu);

        cycles + int_cycles
    }
}
