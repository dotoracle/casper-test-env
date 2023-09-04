#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use casper_types::Gas;
use std::fs::{File, OpenOptions};
use std::io::Write;

use crate::utils::pow;
pub fn write_to(is_deploy: bool, func_name: &str, gas: Gas) {
    let mut output = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("gasStats.txt")
        .unwrap();
    if is_deploy {
        writeln!(
            output,
            "Deploy {}: {}",
            func_name,
            (gas.value().as_u64() as f64) / pow(10, 9).as_u64() as f64
        )
        .unwrap_or_default();
    } else {
        writeln!(
            output,
            "Call {}: {}",
            func_name,
            (gas.value().as_u64() as f64) / pow(10, 9).as_u64() as f64
        )
        .unwrap_or_default();
    }
}
