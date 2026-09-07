//! Deposit / withdraw fuzz target for an 80/20 Comet pool at the minimum swap fee.
//!
//! # Scenario
//!
//! 1. **Token supplies.** Each of the two tokens has a fuzzed total supply, log-uniform up to
//!    `i64::MAX / 2` (the tokens are Stellar Asset Contracts). The supply is split 20% to the
//!    pool, 30% to user A, 50% to user B.
//! 2. **Pool init.** Weights 0.8 / 0.2, minimum swap fee, the pool's 20% share of each token.
//!    `init` mints a fixed `INIT_POOL_SUPPLY`, so this fixes the tokens-per-LP scale.
//! 3. **User A joins** proportionally for a fuzzed LP amount, scaling the LP supply relative to
//!    the token balances (rejected if it exceeds A's balance; the pool then stays at init scale).
//! 4. **User B takes three fuzzed pool actions** — any of the six join / exit / single-sided
//!    functions with fuzzed amounts — then exits any LP still held so they end flat.
//! 5. **Close-out swap.** If B's token-2 balance moved, a swap returns it to where it started, so
//!    the whole sequence nets to "token 1 in, token 1 out" whatever mix of LP paths was used.
//!
//! # Properties, checked after every successful operation
//!
//! 1. **Accounting is exact** (`Fixture::assert_accounting`).
//! 2. **Value per LP share never decreases** (`assert_value_per_share_non_decreasing`).
//! 3. **User B cannot net tokens.** Once B holds no LP:
//!    - if every action used the same value form (all proportional, or all single-sided in one
//!      token) B holds no more of either token than they started with, before the close-out;
//!    - if the close-out brought token 2 back to its starting balance (within tolerance), B holds
//!      no more token 1 than they started with;
//!    - always, B cannot have gained *both* tokens.
//!
//! Tolerance is `PoolState::tolerance` per successful operation. Contract errors returned through
//! `try_*` are rejected operations; a raw panic (`WasmVm / InvalidAction`) fails the case. Dust
//! close-out sells whose output rounds to zero are skipped (see `common::expected_swap_out`).

#![no_main]

use comet_fuzz::common::{
    expected_swap_out, outcome, tracing, Amount, Fixture, Stepper, Supply, Token,
};
use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::arbitrary::arbitrary::{self, Arbitrary};
use soroban_sdk::vec;

const USER_B_ACTIONS: usize = 3;

/// One pool action by user B.
#[derive(Arbitrary, Debug, Clone, Copy)]
enum Action {
    /// `join_pool`: proportional deposit for an exact LP amount out.
    Join { lp_out: Amount },
    /// `exit_pool`: proportional withdrawal of an exact LP amount in (capped at B's balance).
    Exit { lp_in: Amount },
    /// `dep_tokn_amt_in_get_lp_tokns_out`: single-sided, exact token in.
    DepositExactIn { token: Token, amount_in: Amount },
    /// `dep_lp_tokn_amt_out_get_tokn_in`: single-sided, exact LP out.
    DepositExactLpOut { token: Token, lp_out: Amount },
    /// `wdr_tokn_amt_in_get_lp_tokns_out`: single-sided, exact LP in (capped at B's balance).
    WithdrawExactLpIn { token: Token, lp_in: Amount },
    /// `wdr_tokn_amt_out_get_lp_tokns_in`: single-sided, exact token out (max LP = B's balance).
    WithdrawExactOut { token: Token, amount_out: Amount },
}

/// The form in which value moves in an action; decides whether the per-token check applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Proportional,
    Single(Token),
}

impl Action {
    fn form(&self) -> Form {
        match self {
            Action::Join { .. } | Action::Exit { .. } => Form::Proportional,
            Action::DepositExactIn { token, .. }
            | Action::DepositExactLpOut { token, .. }
            | Action::WithdrawExactLpIn { token, .. }
            | Action::WithdrawExactOut { token, .. } => Form::Single(*token),
        }
    }
}

#[derive(Arbitrary, Debug)]
struct Input {
    supply_1: Supply,
    supply_2: Supply,
    user_a_join_lp: Amount,
    actions: [Action; USER_B_ACTIONS],
}

fuzz_target!(|input: Input| {
    let f = Fixture::create(input.supply_1.0, input.supply_2.0);
    let env = &f.env;
    let mut s = Stepper::new(&f);

    // --- 3. user A joins to set the LP scale ------------------------------------------------
    let ok = f.user_a_join(input.user_a_join_lp.0);
    s.step("user A join_pool", ok);

    // --- 4. user B: three fuzzed actions -----------------------------------------------------
    let b = &f.user_b;
    let start_1 = f.token_balance(&f.token_1, b);
    let start_2 = f.token_balance(&f.token_2, b);
    let mut forms: Vec<Form> = Vec::new();

    for (i, action) in input.actions.iter().enumerate() {
        let ctx = format!("user B action {i}: {action:?}");
        let ok = match *action {
            Action::Join { lp_out } => {
                let r = f
                    .pool
                    .try_join_pool(&lp_out.0, &vec![env, i128::MAX, i128::MAX], b);
                s.step(&ctx, outcome(&ctx, r))
            }
            Action::Exit { lp_in } => {
                let lp_in = lp_in.0.min(f.lp(b));
                let r = f.pool.try_exit_pool(&lp_in, &vec![env, 0, 0], b);
                s.step(&ctx, outcome(&ctx, r))
            }
            Action::DepositExactIn { token, amount_in } => {
                let r = f.pool.try_dep_tokn_amt_in_get_lp_tokns_out(
                    f.token(token),
                    &amount_in.0,
                    &0,
                    b,
                );
                s.step(&ctx, outcome(&ctx, r))
            }
            Action::DepositExactLpOut { token, lp_out } => {
                let r = f.pool.try_dep_lp_tokn_amt_out_get_tokn_in(
                    f.token(token),
                    &lp_out.0,
                    &i128::MAX,
                    b,
                );
                s.step(&ctx, outcome(&ctx, r))
            }
            Action::WithdrawExactLpIn { token, lp_in } => {
                let lp_in = lp_in.0.min(f.lp(b));
                let r = f
                    .pool
                    .try_wdr_tokn_amt_in_get_lp_tokns_out(f.token(token), &lp_in, &0, b);
                s.step(&ctx, outcome(&ctx, r))
            }
            Action::WithdrawExactOut { token, amount_out } => {
                let max_lp = f.lp(b);
                let r = f.pool.try_wdr_tokn_amt_out_get_lp_tokns_in(
                    f.token(token),
                    &amount_out.0,
                    &max_lp,
                    b,
                );
                s.step(&ctx, outcome(&ctx, r))
            }
        };
        if ok {
            forms.push(action.form());
        }
    }

    // --- user B exits whatever LP is left so the token check is meaningful --------------------
    if f.lp(b) > 0 {
        let lp = f.lp(b);
        let r = f.pool.try_exit_pool(&lp, &vec![env, 0, 0], b);
        if s.step(
            "user B final exit_pool",
            outcome("user B final exit_pool", r),
        ) {
            forms.push(Form::Proportional);
        }
    }
    // A proportional exit rejects dust LP that would pay out zero of some token. Sweep it through
    // the single-sided path instead, which has no zero-output guard. A sweep that pays nothing
    // moves no value and does not count as a form; one that pays out is a single-sided withdrawal.
    for token in [Token::One, Token::Two] {
        let lp = f.lp(b);
        if lp == 0 {
            break;
        }
        let before = f.token_balance(f.token(token), b);
        let ctx = format!("user B dust sweep {token:?}");
        let r = f
            .pool
            .try_wdr_tokn_amt_in_get_lp_tokns_out(f.token(token), &lp, &0, b);
        if s.step(&ctx, outcome(&ctx, r)) && f.token_balance(f.token(token), b) > before {
            forms.push(Form::Single(token));
        }
    }

    let flat = f.lp(b) == 0 && s.ops > 0;
    let lp_ops = s.ops;

    // --- Property 3a: same-form sequences, before the close-out --------------------------------
    let mid_1 = f.token_balance(&f.token_1, b);
    let mid_2 = f.token_balance(&f.token_2, b);
    let same_form = forms.windows(2).all(|w| w[0] == w[1]);
    if flat && same_form {
        let tol = s.tol;
        assert!(
            mid_1 <= start_1 + tol,
            "user B netted token 1: start {start_1} end {mid_1} (tol {tol})\n  {input:#?}"
        );
        assert!(
            mid_2 <= start_2 + tol,
            "user B netted token 2: start {start_2} end {mid_2} (tol {tol})\n  {input:#?}"
        );
    }

    // --- 5. close-out: bring token 2 back to its starting balance ------------------------------
    // A mixed sequence (e.g. single-sided in with token 1, out with token 2) is economically a
    // swap. Swapping the token-2 change back turns it into "token 1 in, token 1 out" so the
    // token-1 balance alone shows whether B extracted value.
    let mut closed = flat;
    if flat && mid_2 > start_2 {
        let ctx = "user B close-out sell token 2";
        if expected_swap_out(&f, Token::Two, mid_2 - start_2) == Some(0) {
            if tracing() {
                eprintln!("dust_skip {ctx}");
            }
            closed = false;
        } else {
            let r = f.pool.try_swap_exact_amount_in(
                &f.token_2,
                &(mid_2 - start_2),
                &f.token_1,
                &0,
                &i128::MAX,
                b,
            );
            closed = s.step(ctx, outcome(ctx, r));
        }
    } else if flat && mid_2 < start_2 {
        let ctx = "user B close-out buy token 2";
        let r = f.pool.try_swap_exact_amount_out(
            &f.token_1,
            &mid_1,
            &f.token_2,
            &(start_2 - mid_2),
            &i128::MAX,
            b,
        );
        closed = s.step(ctx, outcome(ctx, r));
    }

    // Set FUZZ_TRACE=1 to see, per run, how many operations succeeded and in which forms.
    if tracing() {
        eprintln!(
            "trace lp_ops={lp_ops} forms={forms:?} closed={closed} supply=({}, {}) pool={:?}",
            input.supply_1.0, input.supply_2.0, s.state
        );
    }

    // --- Property 3b: round trip and both-tokens checks ----------------------------------------
    if flat {
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
    }
});
