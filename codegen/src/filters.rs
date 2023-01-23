use std::collections::HashMap;
use tera::{to_value, Value};
use tera::try_get_value;
use crate::models::Operand;

pub fn getter(value: &Value, map: &HashMap<String, Value>) -> tera::Result<Value> {
    let operand = try_get_value!("arg", "value", Operand, value);
    let bits = try_get_value!("arg", "bits", usize, map.get("bits").unwrap());
    Ok(to_value(&eval_getter(&operand.name.to_lowercase(), bits, operand.immediate)).unwrap())
}

fn eval_getter(operand: &str, bits: usize, immediate: bool) -> String {
    if !immediate {
        format!("mmu.get{}({})", bits, eval_getter(operand, bits, true))
    } else if operand == "nz" {
        format!("!cpu.get_zf()")
    } else if operand == "nc" {
        format!("!cpu.get_cf()")
    } else if operand == "z" {
        format!("cpu.get_zf()")
    } else if operand == "cf" {
        format!("cpu.get_cf()")
    } else if operand == "d8" || operand == "a8" || operand == "r8" {
        format!("mmu.get8(cpu.get_pc().wrapping_add(arg))")
    } else if operand == "d16" || operand == "a16" {
        format!("mmu.get16(cpu.get_pc().wrapping_add(arg))")
    } else if operand.starts_with("0x") {
        let mut expr = operand.split("+");
        let offset = expr.next().expect("No offset");
        let arg = expr.next().expect("No arg");
        format!("{}+{} as u16", offset, eval_getter(&arg, bits, true))
    } else if is_num(operand) {
        format!("{}", operand)
    } else {
        format!("cpu.get_{}()", operand)
    }
}

fn is_num(s: &str) -> bool {
    match s.trim().parse::<usize>() {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn setter(value: &Value, map: &HashMap<String, Value>) -> tera::Result<Value> {
    let operand = try_get_value!("setter", "value", Operand, value);
    let bits = try_get_value!("setter", "bits", usize, map.get("bits").unwrap());
    Ok(to_value(&eval_setter(&operand.name.to_lowercase(), bits, operand.immediate)).unwrap())
}

fn eval_setter(operand: &str, bits: usize, immediate: bool) -> String {
    if !immediate {
        format!("mmu.set{}({}, ", bits, eval_getter(&operand, bits, true))
    } else {
        format!("cpu.set_{}(", &operand)
    }
}

pub fn setflag(value: &Value, map: &HashMap<String, Value>) -> tera::Result<Value> {
    let value = try_get_value!("setflag", "value", String, value);
    let flag = try_get_value!("setflag", "flg", String, map.get("flg").unwrap());
    if value == "-" {
        Ok(to_value("").unwrap())
    } else if value == "0" {
        Ok(to_value(format!("cpu.set_{}f(false);", flag)).unwrap())
    } else if value == "1" {
        Ok(to_value(format!("cpu.set_{}f(true);", flag)).unwrap())
    } else {
        Ok(to_value(format!("cpu.set_{}f({});", flag, value.to_lowercase())).unwrap())
    }
}