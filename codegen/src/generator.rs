use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use tera::{Tera, Context};
use crate::{Error, filters, Generate, Result};
use crate::models::{Instruction, Instructions};
use crate::serde_json;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("codegen/templates/**/*") {
            Ok(tera) => {
                let names: Vec<_> = tera.get_template_names().collect();
                println!("Templates found on this location: {}", names.join(", "));
                tera
            },
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };

        tera.register_filter("getter", filters::getter);
        tera.register_filter("setter", filters::setter);
        tera.register_filter("setflag", filters::setflag);
        tera
    };
}

pub fn run(opt: &Generate) -> Result<()> {
    let file = File::open(&opt.opcodes).expect("Opcodes file not found");
    let instructions = get_instructions(&file);

    println!("--------");
    println!("Second instruction on file:\n{:#?}", &instructions[1]);
    println!("--------");

    let mut context = Context::new();
    context.insert("instructions", &instructions);
    let output = match TEMPLATES.render("root.rs", &context) {
        Ok(output) => output,
        Err(e) => return Err(Error(e.to_string()))
    };

    let formatted: String = apply_fmt(&output);

    let mut file = File::create(&opt.output).expect("Output path not found");
    file.write_all(formatted.as_bytes()).expect("Couldn't write to output");

    Ok(())
}

fn get_instructions(file: &File) -> Vec<Instruction> {
    let instructions: Instructions = serde_json::from_reader(file).expect("serde JSON failed");

    let unprefixed_inst: Vec<Instruction> = instructions.unprefixed.into_iter()
        .map(|mut inst| { inst.code.insert_str(2, "00"); inst }).collect();
    let cb_inst: Vec<Instruction> = instructions.cbprefixed.into_iter()
        .map(|mut inst| { inst.code.insert_str(2, "cb"); inst }).collect();

    [unprefixed_inst, cb_inst].concat()
}

fn apply_fmt(input: &String) -> String {
    let process = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Couldn't spawn rustfmt");

    process.stdin.unwrap()
        .write_all(input.as_bytes())
        .expect("Couldn't write to rustfmt");

    let mut formatted = String::new();

    process.stdout.unwrap()
        .read_to_string(&mut formatted)
        .expect("Couldn't read rustfmt");

    formatted
}