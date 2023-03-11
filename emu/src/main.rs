mod emu;
mod cpu;
mod mmu;
mod instr;
mod alu;
mod gui;
mod io;

use log::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use crate::emu::Emu;
use crate::gui::window::GUI;
use crate::gui::hardware::Hardware;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(name = "ROM", parse(from_os_str))]
    rom: PathBuf
}

fn load_rom_buffer<P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut f = File::open(path).expect("Couldn't open file");
    let mut buf = Vec::new();

    f.read_to_end(&mut buf).expect("Couldn't read file");

    buf
}

/*
    Rom loaded on a vector of u8 looks like that:
    [195, 12, 2, 0, 0, 0, 0, 0, 195, 12, 2, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 135,
    225, 95, 22, 0, 25, 94, 35, 86, 213, 225, 233, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 195, 126, 1, 255, 255, 255, 255, 255, 195, 190, 38, 255, 255, 255, 255, 255, 195,
    190, 38, 255, 255, 255, 255, 255, 195, 91, 0, 245, 229, 213, 197, 205, 107, 0, 62, 1, 224, 204, 193, 209, 225, 241, 217, 240, 205, 239, 120, 0, 159, 0, 164, 0, 186, 0, 234, 39, 240,
    225, 254, 7, 40, 8, 254, 6, 200, 62, 6, 224, 225, 201, 240, 1, 254, 85, 32, 8, 62, 41, 224, 203, 62, 1, 24, 8, 254, 41, 192, 62, 85, 224, 203, 175, 224, 2, 201, 240, 1, 224, 208, 201, 240, 1,
    224, 208, 240, 203, 254, 41, 200, 240, 207, 224, 1, 62, 255, 224, 207, 62, 128, 224, 2, 201, 240, 1, 224, 208, ...]
 */

fn main() {
    env_logger::init();

    let args: Opt = Opt::from_args();
    debug!("Reading cartridge from {:?}", &args.rom);

    let rom = load_rom_buffer(&args.rom);
    debug!("ROM size: {} Bytes", &rom.len());

    debug!("Initializing GUI...");
    let gui = GUI::new();
    let gui_arc = Hardware::new(&gui);

    std::thread::spawn(move || {
        debug!("Starting emulator on a new thread...");
        Emu::run(&rom, gui_arc);
    });

    gui.run(); // This blocks contains the main loop
}