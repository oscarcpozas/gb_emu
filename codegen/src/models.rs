use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Flags {
    pub Z: String,
    pub N: String,
    pub H: String,
    pub C: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Operand {
    pub name: String,
    pub immediate: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Cycle {
    One(usize),
    Two(usize, usize),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Instruction {
    pub code: String,
    pub mnemonic: String,
    pub bits: u8,
    pub bytes: u16,
    pub cycles: Vec<u16>,
    pub operands: Vec<Operand>,
    pub immediate: bool,
    pub flags: Flags
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Instructions {
    pub unprefixed: Vec<Instruction>,
    pub cbprefixed: Vec<Instruction>,
}