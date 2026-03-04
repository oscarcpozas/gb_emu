mod alu;
mod cpu;
mod emu;
mod gui;
mod instr;
mod io;
mod mmu;

use crate::emu::Emu;
use crate::gui::hardware::Hardware;
use crate::gui::window::GUI;
use log::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(name = "ROM", parse(from_os_str))]
    rom: PathBuf,
}

fn load_rom_buffer<P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut f = File::open(path).expect("Couldn't open file");
    let mut buf = Vec::new();

    f.read_to_end(&mut buf).expect("Couldn't read file");

    buf
}

/*
   Rom loaded on a vector of u8 looks like that:
   [195, 12, 2, 0, 0, 0, 0, 0, 195, 12, 2, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 135,
    225, 95, 22, 0, 25, 94, 35, 86, 213, 225, 233, 255, 255, 255, 255, 255, ...]
*/

fn main() {
    env_logger::init();

    let args: Opt = Opt::from_args();
    debug!("Reading cartridge from {:?}", &args.rom);

    let rom: Vec<u8> = load_rom_buffer(&args.rom);
    debug!("ROM size: {} Bytes", &rom.len());

    debug!("Initializing GUI...");
    let gui = GUI::new();
    let gui_arc = Hardware::new(&gui);

    std::thread::spawn(move || {
        debug!("Starting emulator on separated thread");
        Emu::run(rom, gui_arc);
    });

    gui.run();
}
