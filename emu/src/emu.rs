use crate::cpu::Cpu;
use crate::gui::hardware::Hardware;
use crate::io::audio::Apu;
use crate::io::boot::BootRom;
use crate::io::graphics::ppu::Ppu;
use crate::io::interrupt::{INT_JOYPAD, Interrupt};
use crate::io::joypad::Joypad;
use crate::io::mbc::cartridge::Cartridge;
use crate::io::timer::Timer;
use crate::mmu::{Mmu, RefCellMemHandler};
use log::{info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread::sleep;
use std::time::{Duration, Instant};
use crate::gui::window::GameBoyKey;

// Game Boy CPU clock speed: 4.194304 MHz
const CPU_CLOCK_HZ: u64 = 4_194_304;
const TARGET_FPS: u64 = 60;
const CYCLES_PER_FRAME: usize = (CPU_CLOCK_HZ / TARGET_FPS) as usize;

pub struct Emu {
    cpu: Cpu,
    mmu: Mmu,
    cartridge: Rc<RefCell<Cartridge>>,
    ppu: Rc<RefCell<Ppu>>,
    interrupt: Rc<RefCell<Interrupt>>,
    joypad: Rc<RefCell<Joypad>>,
    timer: Rc<RefCell<Timer>>,
    apu: Rc<RefCell<Apu>>,
}

impl Emu {
    pub fn run(rom: Vec<u8>, hardware: Hardware) {
        let mut emu = Emu::new(
            rom,
            hardware.get_vram(),
            hardware.get_keys_states(),
            hardware.get_muted(),
        );
        emu.cartridge.borrow().show_info();

        info!("Starting emulation loop");

        while hardware.get_gui_is_alive() {
            let frame_start = Instant::now();

            emu.process_frame();

            // Calculate how long to sleep to maintain target frame rate
            let frame_time = frame_start.elapsed();
            let target_frame_time = Duration::from_micros(1_000_000 / TARGET_FPS);

            if frame_time < target_frame_time {
                sleep(target_frame_time - frame_time);
            } else {
                warn!("Frame took longer than target frame time");
            }
        }
    }

    fn new(
        rom: Vec<u8>,
        vram_buffer: Arc<Mutex<Vec<u32>>>,
        keys: Arc<Mutex<HashMap<GameBoyKey, bool>>>,
        muted: Arc<AtomicBool>,
    ) -> Self {
        let cpu = Cpu::new();
        let mut mmu = Mmu::new();

        let cartridge = Rc::new(RefCell::new(Cartridge::new(rom)));
        let boot_rom = Rc::new(RefCell::new(BootRom::new()));
        let ppu = Rc::new(RefCell::new(Ppu::new(vram_buffer)));
        let interrupt = Rc::new(RefCell::new(Interrupt::new()));
        let joypad = Rc::new(RefCell::new(Joypad::new(keys)));
        let timer = Rc::new(RefCell::new(Timer::new()));
        let apu = Rc::new(RefCell::new(Apu::new(muted)));

        // Boot ROM must be registered BEFORE cartridge so it takes priority for 0x0000-0x00FF.
        // When boot ROM is inactive it returns PassThrough, falling through to cartridge.
        let boot_rom_handler = Rc::new(RefCellMemHandler::new(boot_rom.clone()));
        mmu.add_handler((0x0000, 0x00FF), boot_rom_handler.clone());
        mmu.add_handler((0xFF50, 0xFF50), boot_rom_handler.clone());

        let cartridge_handler = Rc::new(RefCellMemHandler::new(cartridge.clone()));
        mmu.add_handler((0x0000, 0x7FFF), cartridge_handler.clone());

        let ppu_handler = Rc::new(RefCellMemHandler::new(ppu.clone()));
        mmu.add_handler((0x8000, 0x9FFF), ppu_handler.clone()); // VRAM
        mmu.add_handler((0xFE00, 0xFE9F), ppu_handler.clone()); // OAM
        mmu.add_handler((0xFF40, 0xFF4B), ppu_handler.clone()); // LCD Registers
        mmu.add_handler((0xFF46, 0xFF46), ppu_handler.clone()); // OAM DMA

        let interrupt_handler = Rc::new(RefCellMemHandler::new(interrupt.clone()));
        mmu.add_handler((0xFF0F, 0xFF0F), interrupt_handler.clone()); // IF: Interrupt flag
        mmu.add_handler((0xFFFF, 0xFFFF), interrupt_handler.clone()); // IE: Interrupt enable

        let joypad_handler = Rc::new(RefCellMemHandler::new(joypad.clone()));
        mmu.add_handler((0xFF00, 0xFF00), joypad_handler.clone());

        let timer_handler = Rc::new(RefCellMemHandler::new(timer.clone()));
        mmu.add_handler((0xFF04, 0xFF07), timer_handler.clone());

        let apu_handler = Rc::new(RefCellMemHandler::new(apu.clone()));
        mmu.add_handler((0xFF10, 0xFF3F), apu_handler.clone());

        Self {
            cpu,
            mmu,
            cartridge,
            ppu,
            interrupt,
            joypad,
            timer,
            apu,
        }
    }

    fn process_frame(&mut self) {
        let mut cycles_this_frame = 0;

        while cycles_this_frame < CYCLES_PER_FRAME {
            let cycles = self.step();
            cycles_this_frame += cycles;
        }
    }

    fn step(&mut self) -> usize {
        // Execute pending OAM DMA if requested (triggered by write to 0xFF46).
        let pending_dma = self.ppu.borrow_mut().pending_dma.take();
        if let Some(page) = pending_dma {
            let src_addr = (page as u16) << 8;
            // 0xA0 = 160 bytes, size of DMA (Direct Memory Access) OAM buffer
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

        // Update APU
        self.apu.borrow_mut().update(cycles);

        // Poll joypad for newly-pressed keys
        if self.joypad.borrow_mut().poll_interrupt() {
            self.interrupt.borrow_mut().request(INT_JOYPAD);
        }

        // Dispatch pending interrupts to the CPU
        let int_cycles = self
            .interrupt
            .borrow_mut()
            .dispatch(&mut self.cpu, &mut self.mmu);

        cycles + int_cycles
    }
}
