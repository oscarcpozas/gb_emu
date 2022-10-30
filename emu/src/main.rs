mod window;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use crate::window::GUI;

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

fn main() {
    env_logger::init();

    let args: Opt = Opt::from_args();
    println!("Reading cartridge from {:?}", &args.rom);

    let rom = load_rom_buffer(&args.rom);
    println!("ROM size: {} Bytes", rom.len());

    println!("Starting GUI and hardware communication...");
    let gui = GUI::new();
    gui.run();
}