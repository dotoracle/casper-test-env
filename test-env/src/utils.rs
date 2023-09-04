use casper_types::{U128, U256};

pub fn pow(x: u64, y: u64) -> U128 {
    U128::from(x).pow(U128::from(y))
}

pub fn pow_u256(x: u64, y: u64) -> U256 {
    U256::from(x).pow(U256::from(y))
}
