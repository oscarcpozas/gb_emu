/// Integration tests for the emulator components.
///
/// Each test builds only the components it needs (no GUI, no window).
use crate::cpu::Cpu;
use crate::io::boot::BootRom;
use crate::io::graphics::ppu::Ppu;
use crate::io::interrupt::{Interrupt, INT_TIMER, INT_VBLANK};
use crate::io::timer::Timer;
use crate::mmu::{Mmu, RefCellMemHandler};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal MMU with just the boot ROM mapped (no cartridge).
fn mmu_with_boot_rom() -> (Mmu, Rc<RefCell<BootRom>>) {
    let mut mmu = Mmu::new();
    let boot = Rc::new(RefCell::new(BootRom::new()));
    let handler = Rc::new(RefCellMemHandler::new(boot.clone()));
    mmu.add_handler((0x0000, 0x00FF), handler.clone());
    mmu.add_handler((0xFF50, 0xFF50), handler);
    (mmu, boot)
}

/// Build a Ppu backed by an in-process frame buffer (no GUI needed).
fn headless_ppu() -> (Ppu, Arc<Mutex<Vec<u32>>>) {
    let fb = Arc::new(Mutex::new(vec![0u32; 160 * 144]));
    (Ppu::new(fb.clone()), fb)
}

// ---------------------------------------------------------------------------
// CPU tests
// ---------------------------------------------------------------------------

#[test]
fn test_boot_rom_first_instruction_ld_sp() {
    // Boot ROM byte 0: 0x31 0xFE 0xFF  →  LD SP, 0xFFFE
    let (mut mmu, _boot) = mmu_with_boot_rom();
    let mut cpu = Cpu::new();

    let cycles = cpu.fetch_n_execute(&mut mmu);

    assert_eq!(cpu.get_sp(), 0xFFFE, "SP must be 0xFFFE after LD SP,0xFFFE");
    assert_eq!(cpu.get_pc(), 3, "PC must advance past the 3-byte instruction");
    assert_eq!(cycles, 12, "LD SP,nn takes 12 cycles");
}

#[test]
fn test_boot_rom_xor_a_clears_a_and_sets_zero_flag() {
    // Second boot ROM instruction at 0x0003: 0xAF → XOR A  (A ^= A → 0)
    let (mut mmu, _boot) = mmu_with_boot_rom();
    let mut cpu = Cpu::new();

    cpu.set_a(0x42);
    cpu.set_pc(0x0003); // skip LD SP instruction
    cpu.fetch_n_execute(&mut mmu);

    assert_eq!(cpu.get_a(), 0x00, "XOR A must zero register A");
    assert!(cpu.get_zf(), "Zero flag must be set after XOR A");
    assert!(!cpu.get_nf(), "Subtract flag must be clear after XOR A");
    assert!(!cpu.get_hf(), "Half-carry flag must be clear after XOR A");
    assert!(!cpu.get_cf(), "Carry flag must be clear after XOR A");
    assert_eq!(cpu.get_pc(), 0x0004);
}

#[test]
fn test_boot_rom_shadowed_by_cartridge_after_disable() {
    // With boot ROM active, 0x0000 returns boot ROM byte (0x31).
    // After writing to 0xFF50, 0x0000 falls through to cartridge/RAM (0x00).
    let (mut mmu, boot) = mmu_with_boot_rom();

    assert_eq!(mmu.get8(0x0000), 0x31, "Boot ROM byte 0 must be 0x31 when active");

    // Disable the boot ROM by writing a non-zero value to 0xFF50.
    mmu.set8(0xFF50, 0x01);

    assert_eq!(
        mmu.get8(0x0000),
        0x00,
        "Address 0x0000 must fall through to RAM (0x00) once boot ROM is disabled"
    );
}

#[test]
fn test_pc_advances_correctly_for_multibyte_instructions() {
    // LD HL, 0x9FFF is 3 bytes at address 0x0004 in the boot ROM.
    // After execution PC must be 0x0004 + 3 = 0x0007.
    let (mut mmu, _boot) = mmu_with_boot_rom();
    let mut cpu = Cpu::new();
    cpu.set_pc(0x0004);

    cpu.fetch_n_execute(&mut mmu);

    assert_eq!(cpu.get_hl(), 0x9FFF, "HL must be loaded with 0x9FFF");
    assert_eq!(cpu.get_pc(), 0x0007, "PC must advance 3 bytes for LD HL,nn");
}

#[test]
fn test_call_and_ret() {
    // Manually place a tiny routine in RAM and verify CALL + RET.
    // RAM starts at 0xC000. We'll place: RET (0xC9) there.
    // Then a CALL 0xC000 instruction at 0xD000.
    let mut mmu = Mmu::new();
    let mut cpu = Cpu::new();

    // CALL 0xC000 at address 0xD000  →  bytes 0xCD 0x00 0xC0
    mmu.set8(0xD000, 0xCD);
    mmu.set8(0xD001, 0x00);
    mmu.set8(0xD002, 0xC0);
    // RET at 0xC000
    mmu.set8(0xC000, 0xC9);

    cpu.set_pc(0xD000);
    cpu.set_sp(0xFFFE);

    // Execute CALL
    cpu.fetch_n_execute(&mut mmu);
    assert_eq!(cpu.get_pc(), 0xC000, "After CALL, PC must point to subroutine");
    assert_eq!(cpu.get_sp(), 0xFFFC, "CALL must push 2 bytes, moving SP down");
    assert_eq!(mmu.get16(0xFFFC), 0xD003, "Return address must be instruction after CALL");

    // Execute RET
    cpu.fetch_n_execute(&mut mmu);
    assert_eq!(cpu.get_pc(), 0xD003, "After RET, PC must return to caller");
    assert_eq!(cpu.get_sp(), 0xFFFE, "RET must restore SP");
}

#[test]
fn test_halt_idles_until_interrupt_cleared() {
    // CPU in halt burns 4 cycles per step, not advancing PC.
    let mut mmu = Mmu::new();
    let mut cpu = Cpu::new();

    cpu.set_pc(0xC000);
    mmu.set8(0xC000, 0x76); // HALT opcode

    // Execute HALT itself — this should set halted = true.
    cpu.fetch_n_execute(&mut mmu);
    assert!(cpu.halted, "CPU must be halted after HALT instruction");
    assert_eq!(cpu.get_pc(), 0xC001);

    // While halted, fetch_n_execute returns 4 cycles without moving PC.
    let cycles = cpu.fetch_n_execute(&mut mmu);
    assert_eq!(cycles, 4, "Halted CPU must burn exactly 4 cycles per step");
    assert_eq!(cpu.get_pc(), 0xC001, "Halted PC must not advance");

    // Un-halt the CPU (interrupt handler would do this).
    cpu.halted = false;
    cpu.set_pc(0xC001);
    mmu.set8(0xC001, 0x00); // NOP
    let cycles = cpu.fetch_n_execute(&mut mmu);
    assert_eq!(cycles, 4, "NOP takes 4 cycles");
    assert_eq!(cpu.get_pc(), 0xC002);
}

// ---------------------------------------------------------------------------
// Interrupt tests
// ---------------------------------------------------------------------------

#[test]
fn test_interrupt_dispatch_jumps_to_vblank_vector() {
    let mut mmu = Mmu::new();
    let mut cpu = Cpu::new();
    let mut irq = Interrupt::new();

    cpu.set_pc(0xC100);
    cpu.set_sp(0xFFFE);
    cpu.ime = true;

    irq.ie_reg = INT_VBLANK;
    irq.if_reg = INT_VBLANK;

    let extra = irq.dispatch(&mut cpu, &mut mmu);

    assert_eq!(cpu.get_pc(), 0x0040, "VBlank vector is 0x0040");
    assert!(!cpu.ime, "IME must be cleared when servicing interrupt");
    assert_eq!(extra, 20, "Interrupt dispatch costs 20 cycles");
    assert_eq!(mmu.get16(0xFFFC), 0xC100, "Return address must be pushed");
}

#[test]
fn test_interrupt_not_dispatched_when_ime_clear() {
    let mut mmu = Mmu::new();
    let mut cpu = Cpu::new();
    let mut irq = Interrupt::new();

    cpu.set_pc(0xC200);
    cpu.ime = false; // interrupts globally disabled

    irq.ie_reg = INT_VBLANK;
    irq.if_reg = INT_VBLANK;

    let extra = irq.dispatch(&mut cpu, &mut mmu);

    assert_eq!(cpu.get_pc(), 0xC200, "PC must not change when IME=0");
    assert_eq!(extra, 0);
}

#[test]
fn test_halt_wakes_on_interrupt_even_with_ime_clear() {
    // The CPU exits halt when an interrupt is pending regardless of IME.
    let mut mmu = Mmu::new();
    let mut cpu = Cpu::new();
    let mut irq = Interrupt::new();

    cpu.halted = true;
    cpu.ime = false;

    irq.ie_reg = INT_VBLANK;
    irq.if_reg = INT_VBLANK;

    irq.dispatch(&mut cpu, &mut mmu);

    assert!(!cpu.halted, "Halt must clear when interrupt is pending, even with IME=0");
}

// ---------------------------------------------------------------------------
// Timer tests
// ---------------------------------------------------------------------------

#[test]
fn test_div_increments_at_correct_rate() {
    let mut timer = Timer::new();
    let mut mmu = Mmu::new();

    // DIV is the upper byte of a 16-bit counter that increments each cycle.
    // After 256 cycles the upper byte (DIV) should be 1.
    timer.update(255);
    assert_eq!(mmu.get8(0xFF04), 0x00); // read via struct directly
    timer.update(1); // total 256 cycles → DIV = 1

    // We read DIV directly from the struct since no handler is wired in this test.
    // The counter is 256 after 256 cycles, upper byte = 1.
    // Verify by reading through the MemHandler trait.
    use crate::mmu::MemHandler;
    match timer.on_read(0xFF04) {
        crate::mmu::MemRead::Replace(v) => assert_eq!(v, 1, "DIV must be 1 after 256 cycles"),
        _ => panic!("Expected Replace"),
    }
}

#[test]
fn test_timer_overflow_requests_interrupt() {
    let mut timer = Timer::new();

    // Enable timer at slowest rate (1024 cycles/tick), set TIMA to 0xFF.
    use crate::mmu::MemHandler;
    timer.on_write(0xFF07, 0x04); // TAC: timer on, 1024 cycles/tick
    timer.on_write(0xFF05, 0xFF); // TIMA = 0xFF (one tick away from overflow)
    timer.on_write(0xFF06, 0x10); // TMA = 0x10 (reload value)

    let irq = timer.update(1024); // one tick → overflow
    assert_eq!(irq, INT_TIMER, "Timer overflow must produce INT_TIMER");

    // TIMA must reload from TMA.
    match timer.on_read(0xFF05) {
        crate::mmu::MemRead::Replace(v) => assert_eq!(v, 0x10, "TIMA must reload from TMA"),
        _ => panic!("Expected Replace"),
    }
}

#[test]
fn test_div_write_resets_counter() {
    let mut timer = Timer::new();
    use crate::mmu::MemHandler;

    timer.update(512); // advance counter
    timer.on_write(0xFF04, 0xFF); // any write resets DIV

    match timer.on_read(0xFF04) {
        crate::mmu::MemRead::Replace(v) => assert_eq!(v, 0, "Writing DIV must reset it to 0"),
        _ => panic!("Expected Replace"),
    }
}

// ---------------------------------------------------------------------------
// PPU tests
// ---------------------------------------------------------------------------

#[test]
fn test_ppu_vblank_interrupt_fires_at_scanline_144() {
    let (mut ppu, _fb) = headless_ppu();

    // The PPU update() processes ONE mode transition per call, not a full scanline.
    // A full scanline = 456 cycles across 3 phases: OAM(80) + Transfer(172) + HBlank(204).
    // We tick 4 cycles at a time (simulating NOP instructions) and collect all interrupts.
    // VBlank must fire after exactly 144 * 456 = 65664 cycles.

    let total_cycles = 144 * 456; // cycles to complete scanlines 0-143 and enter VBlank
    let mut vblank_fired = false;
    let mut cycles_run = 0;

    while cycles_run < total_cycles + 456 {
        let irq = ppu.update(4);
        cycles_run += 4;
        if irq & INT_VBLANK != 0 {
            vblank_fired = true;
            break;
        }
    }

    assert!(vblank_fired, "VBlank interrupt must fire after 144 scanlines ({} cycles)", total_cycles);
    // Should have fired at approximately the right time (allow a few cycles of tolerance).
    assert!(
        cycles_run <= total_cycles + 20,
        "VBlank fired too late: {} cycles (expected ~{})",
        cycles_run,
        total_cycles
    );
}

#[test]
fn test_ppu_renders_solid_tile_to_framebuffer() {
    let (mut ppu, fb) = headless_ppu();

    // Write a fully white tile (all pixels = color 0) into VRAM tile 0 at 0x8000.
    // Tile row format: low byte then high byte. All-zero = color index 0 for all pixels.
    // (The default VRAM is already zeroed, so this is a no-op, but let's be explicit.)
    for i in 0..16u16 {
        ppu.set_vram(0x8000 + i, 0x00);
    }

    // Write a fully black tile (all pixels = color 3) into VRAM tile 1 at 0x8010.
    // Color 3: both low and high bits set  → low=0xFF, high=0xFF.
    for i in 0..16u16 {
        ppu.set_vram(0x8010 + i, 0xFF);
    }

    // Set tile map position (0,0) to tile 1 (black tile). Tile map starts at 0x9800.
    ppu.set_vram(0x1800, 0x01); // tile map offset 0x1800 in VRAM = 0x9800 in address space

    // Configure: display on, BG on, unsigned tile data (0x8000), BG map at 0x9800.
    ppu.set_vram(0x0000, 0); // just make sure tile 0 row 0 exists

    // Run scanline 0 through all modes (80 + 172 + 204 = 456 cycles).
    ppu.update(80);  // OAM
    ppu.update(172); // Transfer → renders scanline
    // After Transfer→HBlank the scanline is drawn; check framebuffer row 0.
    let fb_snap = fb.lock().unwrap().clone();

    // The first 8 pixels (tile 0 mapped at (0,0) but we set (0,0) to tile 1) should be black.
    // BGP default = 0xFC = 11 11 11 00 → color 3 = black (0xFF0F380F), color 0 = white.
    let black: u32 = 0xFF0F380F;
    assert_eq!(
        fb_snap[0], black,
        "First pixel must be black (tile 1, color index 3 through BGP=0xFC)"
    );
}

#[test]
fn test_ppu_oam_dma_copies_to_oam() {
    let (mut ppu, _fb) = headless_ppu();

    // Simulate a DMA from page 0xC0 (address 0xC000).
    // Build a fake 160-byte block representing sprite data.
    let mut data = vec![0u8; 0xA0];
    // Sprite 0: Y=0x10 (top of screen), X=0x08, tile=0x42, attrs=0x00
    data[0] = 0x10;
    data[1] = 0x08;
    data[2] = 0x42;
    data[3] = 0x00;

    ppu.execute_dma(&data);

    assert_eq!(ppu.get_oam(0xFE00), 0x10, "OAM byte 0 (Y) must match DMA source");
    assert_eq!(ppu.get_oam(0xFE01), 0x08, "OAM byte 1 (X) must match DMA source");
    assert_eq!(ppu.get_oam(0xFE02), 0x42, "OAM byte 2 (tile) must match DMA source");
}
