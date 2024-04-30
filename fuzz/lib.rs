//! Common code for fuzzing test suites.
#![allow(unused)]
#![no_main]

use soroban_fixed_point_math::FixedPoint;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::arbitrary::{arbitrary::{self, Arbitrary, Unstructured}, fuzz_catch_panic}, token};
use soroban_sdk::{testutils::Address as _, vec, Address, token::TokenClient};

mod fixture;
pub use fixture::TestFixture;

#[derive(Arbitrary, Debug)]
pub struct NatI128(
    #[arbitrary(with = |u: &mut Unstructured| u.int_in_range(0..=(u32::MAX as i128)))] pub i128,
);

#[derive(Arbitrary, Debug)]
pub struct LargeI128(
    #[arbitrary(with = |u: &mut Unstructured| u.int_in_range(100000000..=(i64::MAX as i128)))] pub i128,
);

type ContractResult<T> = Result<T, Result<soroban_sdk::Error, soroban_sdk::InvokeError>>;

/// Panic if a contract call result might have been the result of an unexpected panic.
///
/// Calls that return an error with type `ScErrorType::WasmVm` and code `ScErrorCode::InvalidAction`
/// are assumed to be unintended errors. These are the codes that result from plain `panic!` invocations,
/// thus contracts should never simply call `panic!`, but instead use `panic_with_error!`.
///
/// Other rare types of internal exception can return `InvalidAction`.
#[track_caller]
pub fn verify_contract_result<T>(env: &soroban_sdk::Env, r: &ContractResult<T>) {
    use soroban_sdk::testutils::Events;
    use soroban_sdk::xdr::{ScErrorCode, ScErrorType};
    use soroban_sdk::{ConversionError, Error};
    match r {
        Err(Ok(e)) => {
            if e.is_type(ScErrorType::WasmVm) && e.is_code(ScErrorCode::InvalidAction) {
                let msg = "contract failed with InvalidAction - unexpected panic?";
                eprintln!("{msg}");
                eprintln!("recent events (10):");
                for (i, event) in env.events().all().iter().rev().take(10).enumerate() {
                    eprintln!("{i}: {event:?}");
                }
                panic!("{msg}");
            }
        }
        _ => {}
    }
}

/// Asset that `b` is within `percentage` of `a` where `percentage` is a decimal
pub fn assert_approx_eq_rel(a: f64, b: f64, percentage: f64) {
    let rel_delta = b * percentage;

    assert_approx_eq_abs(a, b, rel_delta);
}

/// Asset that `b` is within `abs` of `a`
pub fn assert_approx_eq_abs(a: f64, b: f64, abs: f64) {
    assert!(
        a > b - abs && a < b + abs,
        "assertion failed: `(left != right)` \
         (left: `{:?}`, right: `{:?}`, epsilon: `{:?}`)",
        a,
        b,
        abs
    );
}

/// The set of tokens that the pool supports.
#[derive(Arbitrary, Debug, Clone, Copy)]
pub enum PoolToken {
    ONE,
    TWO,
}

#[derive(Arbitrary, Debug)]
pub struct SwapExactIn {
    pub amount: NatI128,
    pub min_out: NatI128,
    pub token_in: PoolToken,
    // pub token_out: PoolToken,
}

/// Withdraw `amount` of `token` out of the pool for `user`.
#[derive(Arbitrary, Debug)]
pub struct SwapExactOut {
    pub amount: NatI128,
    pub max_in: NatI128,
    // pub token_in: PoolToken,
    pub token_out: PoolToken,
}

impl SwapExactIn {
    pub fn run(&self, fixture: &TestFixture, user_index: usize) {
        let user = match user_index {
            0 => &fixture.frodo,
            1 => &fixture.samwise,
            _ => panic!("invalid user index"),
        };
        let token_in = match self.token_in {
            PoolToken::ONE => &fixture.token_1,
            PoolToken::TWO => &fixture.token_2,
        };
        let token_out = match self.token_in {
            PoolToken::ONE => &fixture.token_2,
            PoolToken::TWO => &fixture.token_1,
        };
        let r = fixture.pool.try_swap_exact_amount_in(&token_in.address, &self.amount.0, &token_out.address, &self.min_out.0, &i128::MAX, &user);
        verify_contract_result(&fixture.env, &r);
    }
}

impl SwapExactOut {
    pub fn run(&self, fixture: &TestFixture, user_index: usize) {
        let user = match user_index {
            0 => &fixture.frodo,
            1 => &fixture.samwise,
            _ => panic!("invalid user index"),
        };
        let token_in = match self.token_out {
            PoolToken::ONE => &fixture.token_2,
            PoolToken::TWO => &fixture.token_1,
        };
        let token_out = match self.token_out {
            PoolToken::ONE => &fixture.token_1,
            PoolToken::TWO => &fixture.token_2,
        };
        let r = fixture.pool.try_swap_exact_amount_out(&token_in.address, &self.max_in.0, &token_out.address, &self.amount.0, &i128::MAX, &user);
        verify_contract_result(&fixture.env, &r);
    }
}

