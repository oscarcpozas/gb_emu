use std::collections::HashMap;
use tera::{to_value, Value};
use tera::try_get_value;

fn is_num(s: &str) -> bool {
    match s.trim().parse::<usize>() {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn eval_getter(s: &str, b: usize) -> String {
    if s == "nz" {
        format!("!cpu.get_zf()")
    } else if s == "nc" {
        format!("!cpu.get_cf()")
    } else if s == "z" {
        format!("cpu.get_zf()")
    } else if s == "cf" {
        format!("cpu.get_cf()")
    } else if s == "d8" || s == "a8" || s == "r8" {
        format!("mmu.get8(cpu.get_pc().wrapping_add(arg))")
    } else if s == "d16" || s == "a16" {
        format!("mmu.get16(cpu.get_pc().wrapping_add(arg))")
    } else if s.starts_with("0x") {
        let mut expr = s.split("+");
        let offset = expr.next().expect("No offset");
        let arg = expr.next().expect("No arg");
        format!("{}+{} as u16", offset, eval_getter(&arg, b))
    } else if is_num(s) {
        format!("{}", s)
    } else if s.starts_with("(") {
        format!("mmu.get{}({})", b, eval_getter(&s[1..s.len() - 1], b))
    } else {
        format!("cpu.get_{}()", s)
    }
}

pub fn getter(value: &Value, map: &HashMap<String, Value>) -> tera::Result<Value> {
    let value = try_get_value!("arg", "value", String, value);
    let bits = try_get_value!("arg", "bits", usize, map.get("bits").unwrap());
    Ok(to_value(&eval_getter(&value, bits)).unwrap())
}

fn eval_setter(s: &str, b: usize) -> String {
    if s.starts_with("(") {
        format!("mmu.set{}({}, ", b, eval_getter(&s[1..s.len() - 1], b))
    } else {
        format!("cpu.set_{}(", s)
    }
}

pub fn setter(value: &Value, map: &HashMap<String, Value>) -> tera::Result<Value> {
    let value = try_get_value!("setter", "value", String, value);
    let bits = try_get_value!("setter", "bits", usize, map.get("bits").unwrap());
    Ok(to_value(&eval_setter(&value, bits)).unwrap())
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