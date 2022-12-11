{% import "ops.rs" as macros %}

use crate::cpu::Cpu;
use crate::mmu::Mmu;
use crate::alu;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::*;

{% for i in instructions %}
#[allow(unused_variables)]
fn op_{{i.code}}(arg: u16, cpu: &mut Cpu, mmu: &mut Mmu) -> (usize, usize) {
    {%- if i.mnemonic == "NOP" -%}

        {{ macros::nop(i=i) }}

    {%- elif i.mnemonic == "INC" -%}

        {%- if i.bits == 8 -%}
        {{ macros::inc8(i=i) }}
        {%- else -%}
        {{ macros::inc16(i=i) }}
        {%- endif -%}

    {%- elif i.mnemonic == "DEC" -%}

        {%- if i.bits == 8 -%}
        {{ macros::dec8(i=i) }}
        {%- else -%}
        {{ macros::dec16(i=i) }}
        {%- endif -%}

    {%- elif i.mnemonic == "LD" -%}

        {{ macros::ld(i=i) }}

    {%- elif i.mnemonic == "LDD" -%}

        {{ macros::ld(i=i) }}
        cpu.set_hl(cpu.get_hl().wrapping_sub(1));

    {%- elif i.mnemonic == "LDI" -%}

        {{ macros::ld(i=i) }}
        cpu.set_hl(cpu.get_hl().wrapping_add(1));

    {%- elif i.mnemonic == "LDHL" -%}

        {{ macros::ldhl(i=i) }}

    {%- elif i.mnemonic == "ADD" -%}

        {%- if i.code == "0xE8" -%}
        {{ macros::addsp(i=i) }}
        {%- else -%}
            {%- if i.bits == 8 -%}
            {{ macros::add8(i=i) }}
            {%- else -%}
            {{ macros::add16(i=i) }}
            {%- endif -%}
        {%- endif -%}

    {%- endif -%}

    ({{i.cycles[0]}}, {{i.bytes}})
}
{% endfor %}

pub fn decode(code: u16, arg: u16, cpu: &mut Cpu, mmu: &mut Mmu) -> (usize, usize) {
    match code {
        {% for i in instructions -%}
        {{i.code}} => op_{{i.code}}(arg, cpu, mmu),
        {% endfor -%}
        _ => panic!("Invalid opcode: {:04x}: {:04x}", cpu.get_pc(), code),
    }
}