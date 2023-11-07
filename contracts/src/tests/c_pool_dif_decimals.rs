#![cfg(test)]

use std::println;
extern crate std;
use crate::c_consts::BONE;
use crate::c_pool::comet::CometPoolContract;
use crate::c_pool::comet::CometPoolContractClient;
use crate::c_pool::error::Error;
use soroban_sdk::String;
use soroban_sdk::token;
use soroban_sdk::xdr::AccountId;
// use soroban_sdk::xdr::ScStatusType;
use soroban_sdk::Bytes;
use soroban_sdk::{testutils::Address as _, Address, IntoVal};
use soroban_sdk::{vec, BytesN, Env, Symbol};

mod test_token {
    soroban_sdk::contractimport!(
        file = "../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

fn create_token_contract<'a>(e: &'a Env, admin: &'a soroban_sdk::Address) -> token::Client<'a> {
    token::Client::new(&e, &e.register_stellar_asset_contract(admin.clone()))
}

fn create_and_init_token_contract<'a>(
    env: &'a Env,
    admin_id: &'a Address,
    decimals: &'a u32,
    name: &'a str,
    symbol: &'a str,
) -> test_token::Client<'a> {
    let token_id = env.register_contract_wasm(None, test_token::WASM);
    let client = test_token::Client::new(&env, &token_id);
    client.initialize(
        &admin_id,
        decimals,
        &String::from_slice(&env, name),
        &String::from_slice(&env, symbol),
    );
    client
}

// fn install_token_wasm(e: &Env) -> BytesN<32> {
//     soroban_sdk::contractimport!(
//         file = "../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
//     );
//     e.install_contract_wasm(WASM)
// }

fn to_stroop<T: Into<f64>>(a: T) -> i128 {
    (a.into() * 1e7) as i128
}
fn to_six_dec<T: Into<f64>>(a: T) -> i128 {
    (a.into() * 1e6) as i128
}

#[test]
fn test_pool_functions_different_decimals() {
    let env: Env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let admin = soroban_sdk::Address::random(&env);
    let user1 = soroban_sdk::Address::random(&env);
    let user2 = soroban_sdk::Address::random(&env);
    let contract_id = env.register_contract(None, CometPoolContract);
    let client = CometPoolContractClient::new(&env, &contract_id);
    let factory = admin.clone();
    let controller_arg = factory.clone();
    client.init(&factory, &controller_arg);
    env.budget().reset_unlimited();

    // Create Admin
    let mut admin1 = soroban_sdk::Address::random(&env);

    // Create 4 tokens
    let mut token1: test_token::Client<'_> = create_and_init_token_contract(&env, &admin1, &5, "NebulaCoin", "NBC");
    let mut token2: test_token::Client<'_> = create_and_init_token_contract(&env, &admin1, &7, "StroopCoin", "STRP");

    // let mut token1 = create_token_contract(&env, &admin1);
    // let mut token2 = create_token_contract(&env, &admin1);

    // Create 2 users
    let mut user1 = soroban_sdk::Address::random(&env);
    let mut user2 = soroban_sdk::Address::random(&env);

    token1.mint(&admin1, &to_six_dec(50));
    token2.mint(&admin1, &to_stroop(20));

    token1.mint(&admin, &to_six_dec(50));
    token2.mint(&admin, &to_stroop(20));

    println!("Token Balance of User1 before = {}", token2.balance(&user1));
    token1.mint(&user1, &to_six_dec(25));
    token2.mint(&user1, &to_stroop(4));
    println!(
        "Token Balance of User1 After minting = {}",
        token2.balance(&user1)
    );

    token1.mint(&user2, &to_six_dec(12));
    token2.mint(&user2, &to_stroop(5));

    let controller = client.get_controller();
    assert_eq!(controller, admin);
    let num_tokens = client.get_num_tokens();
    assert_eq!(num_tokens, 0);

    let contract_address = contract_id;
    // token1.approve(&admin, &contract_address, &i128::MAX, &200);
    // token2.approve(&admin, &contract_address, &i128::MAX, &200);

    // client.bind(&token1.address, to_six_dec(50), &to_stroop(5), &admin);
    // client.bind(&token2.address, &to_stroop(20), &to_stroop(5), &admin);
    
    client.bundle_bind(&vec![&env, token1.address.clone() ,token2.address.clone() ], &vec![&env, to_six_dec(50), to_stroop(20)], &vec![&env, to_stroop(5), to_stroop(5)]);
    // let token_vec = vec![client, ]
    client.set_swap_fee(&to_stroop(0.003), &controller);
    let swap_fee = client.get_swap_fee();
    assert_eq!(swap_fee, to_stroop(0.003));
    client.finalize();

    token1.approve(&user1, &contract_address, &i128::MAX, &200);
    token2.approve(&user1, &contract_address, &i128::MAX, &200);

    token1.approve(&user2, &contract_address, &i128::MAX, &200);
    token2.approve(&user2, &contract_address, &i128::MAX, &200);

    println!("Token Balance of User1 before = {}", token1.balance(&user2));

    env.budget().reset_unlimited();

    client.join_pool(&to_stroop(10), &vec![&env, i128::MAX, i128::MAX], &user2);

    client.join_pool(&to_stroop(10), &vec![&env, i128::MAX, i128::MAX], &user1);

    client.exit_pool(&to_stroop(10), &vec![&env, 0, 0], &user1);

    env.budget().reset_unlimited();

    client.join_pool(&to_stroop(10), &vec![&env, i128::MAX, i128::MAX], &user1);
    client.exit_pool(&to_stroop(10), &vec![&env, 0, 0], &user1);

    client.join_pool(&to_stroop(10), &vec![&env, i128::MAX, i128::MAX], &user1);
    client.exit_pool(&to_stroop(10), &vec![&env, 0, 0], &user1);
    env.budget().reset_unlimited();

    client.join_pool(&to_stroop(10), &vec![&env, i128::MAX, i128::MAX], &user1);

    client.exit_pool(&to_stroop(10), &vec![&env, 0, 0], &user1);

    client.join_pool(&to_stroop(10), &vec![&env, i128::MAX, i128::MAX], &user1);
    client.exit_pool(&to_stroop(10), &vec![&env, 0, 0], &user1);

    client.exit_pool(&to_stroop(10), &vec![&env, 0, 0], &user2);

    client.join_pool(&to_stroop(10), &vec![&env, i128::MAX, i128::MAX], &user2);

    client.exit_pool(&to_stroop(10), &vec![&env, 0, 0], &user2);

    env.budget().reset_unlimited();

    client.join_pool(&to_stroop(10), &vec![&env, i128::MAX, i128::MAX], &user2);

    client.exit_pool(&to_stroop(10), &vec![&env, 0, 0], &user2);

    // The balances prove that there is no problem when a user continuously
    // joins and exits pool to gain surplus amounts due to rounding errors.
    println!("Token Balance of User2 Final = {}", token2.balance(&user2));
    println!("Token Balance of User1 Final = {}", token2.balance(&user1));

    assert_eq!(client.balance(&user2), 0);
    assert_eq!(client.balance(&user1), 0);
}
