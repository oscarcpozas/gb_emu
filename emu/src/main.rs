mod alu;
mod cpu;
mod emu;
mod gui;
mod instr;
mod io;
mod mmu;
#[cfg(test)]
mod tests;

use crate::emu::Emu;
use crate::gui::hardware::Hardware;
use crate::gui::window::GUI;
use log::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(name = "ROM", parse(from_os_str))]
    rom: Option<PathBuf>,
}

fn load_rom_buffer<P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut f = File::open(&path).expect("Couldn't open ROM file");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("Couldn't read ROM file");
    buf
}

fn read_rom_args() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        Some(PathBuf::from(args[1].clone()))
    } else {
        None
    }
}

fn main() {
    env_logger::init();

    let args: Opt = Opt::from_args();

    let gui = GUI::new();
    let hardware = Hardware::new(&gui);

    match args.rom {
        Some(path) => {
            debug!("Reading cartridge from {:?}", path);
            let rom = load_rom_buffer(&path);
            debug!("ROM size: {} bytes", rom.len());

            std::thread::spawn(move || {
                debug!("Starting emulator thread");
                Emu::run(rom, hardware);
            });

            gui.run(true);
        }

        None => {
            debug!("No ROM provided — waiting for drag & drop or file picker");

            // Clone the Arc so the watcher thread can observe it while the
            // main thread is busy running the winit event loop.
            let dropped_file: Arc<Mutex<Option<PathBuf>>> = gui.dropped_file.clone();

            std::thread::spawn(move || {
                loop {
                    let path = dropped_file.lock().unwrap().clone();
                    if let Some(p) = path {
                        debug!("Loading ROM from {:?}", p);
                        let rom = load_rom_buffer(&p);
                        debug!("ROM size: {} bytes", rom.len());
                        Emu::run(rom, hardware);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            });

            // Show splash until the user provides a ROM, then switch to game loop.
            gui.run(false);
        }
    }
}
