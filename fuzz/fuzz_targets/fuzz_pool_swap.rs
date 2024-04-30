#![allow(unused)]
#![no_main]
use fuzz_common::{
    verify_contract_result, assert_approx_eq_rel, NatI128, LargeI128, SwapExactIn, SwapExactOut, TestFixture
};
use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::arbitrary::{fuzz_catch_panic, arbitrary::{self, Arbitrary, Unstructured}};
use soroban_sdk::{testutils::Address as _, vec, Address, token::TokenClient};

#[derive(Arbitrary, Debug)]
struct Input {
    samwise_1_balance: NatI128,
    samwise_2_balance: NatI128,
    frodo_1_balance: NatI128,
    frodo_2_balance: NatI128,
    init_1_balance: LargeI128,
    init_2_balance: LargeI128,
    commands: [Command; 10],
}

#[derive(Arbitrary, Debug)]
enum Command {
    // Sam (1) Pool Commands
    SamSwapExactIn(SwapExactIn),
    SamSwapExactOut(SwapExactOut),

    // Frodo (2) Pool Commands
    FrodoSwapExactIn(SwapExactIn),
    FrodoSwapExactOut(SwapExactOut),
}

fuzz_target!(|input: Input| {
    let mut fixture = TestFixture::create(0_8000000, 0_2000000, input.init_1_balance.0, input.init_2_balance.0, 0_0030000);

    // Create two new users
    fixture.token_1.mint(&fixture.samwise, &input.samwise_1_balance.0);
    fixture.token_2.mint(&fixture.samwise, &input.samwise_2_balance.0);
    fixture.token_1.mint(&fixture.frodo, &input.frodo_1_balance.0);
    fixture.token_2.mint(&fixture.frodo, &input.frodo_2_balance.0);

    let invariant = fixture.calc_invariant();
    let mut last_invariant = invariant.clone();
    for command in &input.commands {
        command.run(&fixture);
        let new_invariant = fixture.calc_invariant();
        assert_approx_eq_rel(last_invariant, new_invariant, 0.0035);
        last_invariant = new_invariant;
    }
    assert_approx_eq_rel(invariant, last_invariant, 0.01);
    assert!(last_invariant >= invariant);
});

impl Command {
    fn run(&self, fixture: &TestFixture) {
        use Command::*;
        match self {
            SamSwapExactIn(cmd) => cmd.run(fixture, 0),
            SamSwapExactOut(cmd) => cmd.run(fixture, 0),
            FrodoSwapExactIn(cmd) => cmd.run(fixture, 1),
            FrodoSwapExactOut(cmd) => cmd.run(fixture, 1),
        }
    }
}
