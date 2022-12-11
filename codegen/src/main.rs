extern crate env_logger;
extern crate structopt;
extern crate serde_json;

mod generator;
mod models;
mod filters;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Generate {
    #[structopt(name = "OPCODES", parse(from_os_str))]
    opcodes: PathBuf,
    #[structopt(name = "OUTPUT", parse(from_os_str))]
    output: PathBuf,
}

#[derive(Debug)]
pub struct Error(String);

pub type Result<T> = std::result::Result<T, Error>;

fn main() {
    env_logger::init();

    let args = Generate::from_args();
    match generator::run(&args) {
        Ok(_) => {
            println!("File generated successfully!");
        }
        Err(e) => {
            println!("Error: {:#?}", e);
            std::process::exit(1);
        }
    };
}

/*
    Command to run code generation from root directory:
    cargo run --bin codegen ./codegen/res/LR35902_opcodes.patched.json ./emu/src/instr.rs
 */
