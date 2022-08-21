use crate::cpu::Cpu;
use crate::mmu::Mmu;
use crate::alu;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::*;

pub fn decode(code: u16, arg: u16, cpu: &mut Cpu, mmu: &mut Mmu) -> (usize, usize) {
    match code {
        {% for i in instructions -%}
        {{i.code}} => op_{{i.code}}(arg, cpu, mmu),
        {% endfor -%}
        _ => panic!("Invalid opcode: {:04x}: {:04x}", cpu.get_pc(), code),
    }
}