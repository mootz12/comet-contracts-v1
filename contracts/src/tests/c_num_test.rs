#![cfg(test)]
extern crate std;
use soroban_sdk::Env;
use soroban_sdk::I256;

use crate::c_consts::BONE;
use crate::c_num::c_pow;

#[test]
#[should_panic = "Error(Contract, #34)"]
fn test_c_pow_low() {
    let env: Env = Env::default();
    c_pow(
        &env,
        &I256::from_i32(&env, 0),
        &I256::from_i32(&env, 2),
        false,
    );
}

#[test]
#[should_panic = "Error(Contract, #35)"]
fn test_c_pow_high() {
    let env: Env = Env::default();
    c_pow(
        &env,
        &I256::from_i128(&env, 2 * BONE),
        &I256::from_i32(&env, 2),
        false,
    );
}

#[test]
fn test_c_pow_integer_rounding_direction() {
    let env = Env::default();
    let base = I256::from_i128(&env, BONE + 1);
    let exp = I256::from_i128(&env, 2 * BONE);

    assert_eq!(c_pow(&env, &base, &exp, false).to_i128().unwrap(), BONE + 2);
    assert_eq!(c_pow(&env, &base, &exp, true).to_i128().unwrap(), BONE + 3);
}
