//! Swap fuzz target for an 80/20 Comet pool at the minimum swap fee.
//!
//! # Scenario
//!
//! Same fixture as `fuzz_deposit_withdraw`: fuzzed token supplies up to `i64::MAX / 2`, split
//! 20% pool / 30% user A / 50% user B; user A joins for a fuzzed LP amount. Then **user B makes
//! three fuzzed swaps** — `swap_exact_amount_in` or `swap_exact_amount_out`, either direction —
//! and finally a **close-out swap** that returns B's token-2 balance to where it started, so the
//! whole sequence nets to "token 1 in, token 1 out".
//!
//! # Properties, checked after every successful operation
//!
//! 1. **Accounting is exact** (`Fixture::assert_accounting`).
//! 2. **Value per LP share never decreases** (`assert_value_per_share_non_decreasing`). Swaps do
//!    not change the LP supply, so this is the Balancer invariant `b1^0.8 · b2^0.2` itself, and
//!    the supply is additionally asserted constant.
//! 3. **User B cannot net tokens.** If the close-out brought token 2 back to its starting
//!    balance (within tolerance), B holds no more token 1 than they started with. Always: B
//!    cannot have gained *both* tokens.
//!
//! # Known issue skipped, not asserted
//!
//! `swap_exact_amount_in` divides `amount_in / amount_out` for its price sanity check without a
//! `token_amount_out > 0` guard, so a dust swap whose output rounds to zero hits a raw
//! divide-by-zero panic (`WasmVm / InvalidAction`) instead of a typed error. Balancer's `bdiv`
//! reverts typed there. The target computes the expected output with the contract's own math
//! first and records such cases as `dust_skip` instead of calling in, so that any *other* raw
//! panic still fails the case. See REVIEW.md.

#![no_main]

use comet_fuzz::c_consts::{MAX_IN_RATIO, STROOP};
use comet_fuzz::c_math::calc_token_out_given_token_in;
use comet_fuzz::common::{outcome, tracing, Amount, Fixture, Stepper, Supply, Token, SWAP_FEE};
use libfuzzer_sys::fuzz_target;
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::testutils::arbitrary::arbitrary::{self, Arbitrary};

const USER_B_SWAPS: usize = 3;

#[derive(Arbitrary, Debug, Clone, Copy)]
enum Swap {
    /// `swap_exact_amount_in`: sell `amount_in` of `token_in` for whatever comes out.
    ExactIn { token_in: Token, amount_in: Amount },
    /// `swap_exact_amount_out`: buy `amount_out` of `token_out` for whatever it costs.
    ExactOut {
        token_out: Token,
        amount_out: Amount,
    },
}

#[derive(Arbitrary, Debug)]
struct Input {
    supply_1: Supply,
    supply_2: Supply,
    user_a_join_lp: Amount,
    swaps: [Swap; USER_B_SWAPS],
}

/// Expected output of `swap_exact_amount_in`, computed with the contract's own math, or `None`
/// when the contract would reject the amount before reaching the math (`MAX_IN_RATIO`).
fn expected_out(f: &Fixture, token_in: Token, amount_in: i128) -> Option<i128> {
    let in_rec = f.record(token_in);
    let out_rec = f.record(token_in.other());
    let max_in = in_rec.balance.fixed_mul_floor(MAX_IN_RATIO, STROOP)?;
    if amount_in > max_in {
        return None;
    }
    Some(calc_token_out_given_token_in(
        &f.env, &in_rec, &out_rec, amount_in, SWAP_FEE,
    ))
}

fuzz_target!(|input: Input| {
    let f = Fixture::create(input.supply_1.0, input.supply_2.0);
    let mut s = Stepper::new(&f);

    let ok = f.user_a_join(input.user_a_join_lp.0);
    s.step("user A join_pool", ok);
    let supply_before_swaps = s.state.supply;

    let b = &f.user_b;
    let start_1 = f.token_balance(&f.token_1, b);
    let start_2 = f.token_balance(&f.token_2, b);
    let mut swaps_ok = 0;
    let mut dust_skip = 0;

    // A swap, with the known dust divide-by-zero routed around. Returns whether it succeeded.
    let mut do_swap = |ctx: &str, swap: Swap| -> bool {
        match swap {
            Swap::ExactIn {
                token_in,
                amount_in,
            } => {
                if expected_out(&f, token_in, amount_in.0) == Some(0) {
                    dust_skip += 1;
                    if tracing() {
                        eprintln!("dust_skip {ctx}");
                    }
                    return false;
                }
                let r = f.pool.try_swap_exact_amount_in(
                    f.token(token_in),
                    &amount_in.0,
                    f.token(token_in.other()),
                    &0,
                    &i128::MAX,
                    b,
                );
                s.step(ctx, outcome(ctx, r))
            }
            Swap::ExactOut {
                token_out,
                amount_out,
            } => {
                let token_in = token_out.other();
                let max_in = f.token_balance(f.token(token_in), b);
                let r = f.pool.try_swap_exact_amount_out(
                    f.token(token_in),
                    &max_in,
                    f.token(token_out),
                    &amount_out.0,
                    &i128::MAX,
                    b,
                );
                s.step(ctx, outcome(ctx, r))
            }
        }
    };

    for (i, swap) in input.swaps.iter().enumerate() {
        let ctx = format!("user B swap {i}: {swap:?}");
        if do_swap(&ctx, *swap) {
            swaps_ok += 1;
        }
    }

    // --- close-out: bring token 2 back to its starting balance --------------------------------
    let mid_2 = f.token_balance(&f.token_2, b);
    let mut closed = true;
    if mid_2 > start_2 {
        // sell the token-2 gain back for token 1
        closed = do_swap(
            "close-out sell token 2",
            Swap::ExactIn {
                token_in: Token::Two,
                amount_in: Amount(mid_2 - start_2),
            },
        );
    } else if mid_2 < start_2 {
        // buy back the token-2 shortfall with token 1
        closed = do_swap(
            "close-out buy token 2",
            Swap::ExactOut {
                token_out: Token::Two,
                amount_out: Amount(start_2 - mid_2),
            },
        );
    }

    assert_eq!(
        s.state.supply, supply_before_swaps,
        "LP supply changed across swaps"
    );

    if tracing() {
        eprintln!(
            "trace swaps_ok={swaps_ok} dust_skip={dust_skip} closed={closed} supply=({}, {}) pool={:?}",
            input.supply_1.0, input.supply_2.0, s.state
        );
    }

    // --- Property 3 --------------------------------------------------------------------------
    let end_1 = f.token_balance(&f.token_1, b);
    let end_2 = f.token_balance(&f.token_2, b);
    let tol = s.tol;
    if closed && (end_2 - start_2).abs() <= tol {
        assert!(
            end_1 <= start_1 + tol,
            "user B netted token 1 after round trip: start {start_1} end {end_1} (tol {tol})\n  {input:#?}"
        );
    }
    assert!(
        !(end_1 > start_1 + tol && end_2 > start_2 + tol),
        "user B netted both tokens: start ({start_1}, {start_2}) end ({end_1}, {end_2}) (tol {tol})\n  {input:#?}"
    );
});
