//! Shared fixture and property checks for the fuzz targets.

use crate::c_consts::{MIN_FEE, STROOP_SCALAR};
use crate::c_pool::comet::{CometPoolContract, CometPoolContractClient};
use crate::c_pool::storage_types::Record;
use num_bigint::BigInt;
use num_traits::Zero;
use soroban_sdk::testutils::arbitrary::arbitrary::{self, Arbitrary, Unstructured};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::xdr::{ScErrorCode, ScErrorType};
use soroban_sdk::{vec, Address, Env, Error, InvokeError};

// ---------------------------------------------------------------------------------------------
// Fixture constants
// ---------------------------------------------------------------------------------------------

pub const WEIGHT_1: i128 = 0_8000000; // 80%
pub const WEIGHT_2: i128 = 0_2000000; // 20%
pub const SWAP_FEE: i128 = MIN_FEE; // 0.0001%

/// Total supply of each token, in stroops: 1 token .. i64::MAX / 2 (Stellar Asset Contract cap).
pub const MIN_SUPPLY: i128 = 10i128.pow(7);
pub const MAX_SUPPLY: i128 = (i64::MAX / 2) as i128;

/// Supply split, as percentages of each token's total supply.
pub const POOL_PCT: i128 = 20;
pub const USER_A_PCT: i128 = 30;
pub const USER_B_PCT: i128 = 50;

/// Largest single fuzzed amount (tokens or LP), in stroops.
pub const MAX_AMOUNT: i128 = MAX_SUPPLY;

// ---------------------------------------------------------------------------------------------
// Fuzz input primitives
// ---------------------------------------------------------------------------------------------

/// Log-uniform integer in `lo ..= hi`: pick a decade, then uniform within it. Uniform sampling
/// over 18 orders of magnitude would spend nearly every case at the top.
pub fn log_uniform(u: &mut Unstructured<'_>, lo: i128, hi: i128) -> arbitrary::Result<i128> {
    let lo_exp = lo.ilog10();
    let hi_exp = hi.ilog10();
    let exp = u.int_in_range(lo_exp..=hi_exp)?;
    let d_lo = 10i128.pow(exp).max(lo);
    let d_hi = 10i128.pow(exp + 1).saturating_sub(1).min(hi);
    u.int_in_range(d_lo..=d_hi)
}

/// A positive amount in `1 ..= MAX_AMOUNT`, log-uniform.
#[derive(Debug, Clone, Copy)]
pub struct Amount(pub i128);

impl<'a> Arbitrary<'a> for Amount {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Amount(log_uniform(u, 1, MAX_AMOUNT)?))
    }
}

/// A token supply in `MIN_SUPPLY ..= MAX_SUPPLY`, log-uniform.
#[derive(Debug, Clone, Copy)]
pub struct Supply(pub i128);

impl<'a> Arbitrary<'a> for Supply {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Supply(log_uniform(u, MIN_SUPPLY, MAX_SUPPLY)?))
    }
}

#[derive(Arbitrary, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    One,
    Two,
}

impl Token {
    pub fn other(self) -> Token {
        match self {
            Token::One => Token::Two,
            Token::Two => Token::One,
        }
    }
}

/// `true` when `FUZZ_TRACE` is set in the environment.
pub fn tracing() -> bool {
    std::env::var_os("FUZZ_TRACE").is_some()
}

// ---------------------------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------------------------

/// An 80/20 pool at the minimum swap fee over two Stellar Asset Contracts.
///
/// Each token's supply is split `POOL_PCT` / `USER_A_PCT` / `USER_B_PCT`. `init` mints the fixed
/// `INIT_POOL_SUPPLY`, so the pool's share of supply sets the tokens-per-LP scale.
pub struct Fixture<'a> {
    pub env: Env,
    pub admin: Address,
    pub user_a: Address,
    pub user_b: Address,
    pub token_1: Address,
    pub token_2: Address,
    pub pool: CometPoolContractClient<'a>,
}

impl Fixture<'_> {
    pub fn create<'a>(supply_1: i128, supply_2: i128) -> Fixture<'a> {
        let env = Env::default();
        env.mock_all_auths();
        env.cost_estimate().budget().reset_unlimited();

        let admin = Address::generate(&env);
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);

        let token_1 = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let token_2 = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();

        let pct = |supply: i128, pct: i128| supply * pct / 100;
        for (token, supply) in [(&token_1, supply_1), (&token_2, supply_2)] {
            let sac = StellarAssetClient::new(&env, token);
            sac.mint(&admin, &pct(supply, POOL_PCT));
            sac.mint(&user_a, &pct(supply, USER_A_PCT));
            sac.mint(&user_b, &pct(supply, USER_B_PCT));
        }

        let pool_id = env.register(CometPoolContract, ());
        let pool = CometPoolContractClient::new(&env, &pool_id);
        pool.init(
            &admin,
            &vec![&env, token_1.clone(), token_2.clone()],
            &vec![&env, WEIGHT_1, WEIGHT_2],
            &vec![&env, pct(supply_1, POOL_PCT), pct(supply_2, POOL_PCT)],
            &SWAP_FEE,
        );

        Fixture {
            env,
            admin,
            user_a,
            user_b,
            token_1,
            token_2,
            pool,
        }
    }

    pub fn token(&self, t: Token) -> &Address {
        match t {
            Token::One => &self.token_1,
            Token::Two => &self.token_2,
        }
    }

    pub fn token_balance(&self, token: &Address, who: &Address) -> i128 {
        TokenClient::new(&self.env, token).balance(who)
    }

    pub fn lp(&self, who: &Address) -> i128 {
        self.pool.balance(who)
    }

    /// The pool's stored record for a token, as the math functions expect it. Both tokens are
    /// 7-decimal Stellar assets, so the scalar is `STROOP_SCALAR`.
    pub fn record(&self, t: Token) -> Record {
        Record {
            balance: self.pool.get_balance(self.token(t)),
            weight: self.pool.get_normalized_weight(self.token(t)),
            scalar: STROOP_SCALAR,
            index: match t {
                Token::One => 0,
                Token::Two => 1,
            },
        }
    }

    pub fn state(&self) -> PoolState {
        PoolState {
            b1: self.pool.get_balance(&self.token_1),
            b2: self.pool.get_balance(&self.token_2),
            supply: self.pool.get_total_supply(),
        }
    }

    /// Property: recorded balances match token balances; LP supply matches holder balances; the
    /// pool holds none of its own LP between operations.
    pub fn assert_accounting(&self, ctx: &str) {
        for token in [&self.token_1, &self.token_2] {
            let recorded = self.pool.get_balance(token);
            let actual = self.token_balance(token, &self.pool.address);
            assert_eq!(
                recorded, actual,
                "{ctx}: recorded balance {recorded} != token balance {actual}"
            );
        }
        let supply = self.pool.get_total_supply();
        let held_by_pool = self.lp(&self.pool.address);
        let held =
            self.lp(&self.admin) + self.lp(&self.user_a) + self.lp(&self.user_b) + held_by_pool;
        assert_eq!(
            supply, held,
            "{ctx}: LP total supply {supply} != sum of holder balances {held}"
        );
        assert_eq!(
            held_by_pool, 0,
            "{ctx}: pool holds {held_by_pool} of its own LP tokens between operations"
        );
    }

    /// Has user A join for `lp` LP so the LP supply scale is fuzzed too. Rejected (and harmless)
    /// when it exceeds A's balance.
    pub fn user_a_join(&self, lp: i128) -> bool {
        let r = self
            .pool
            .try_join_pool(&lp, &vec![&self.env, i128::MAX, i128::MAX], &self.user_a);
        outcome("user A join_pool", r)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PoolState {
    pub b1: i128,
    pub b2: i128,
    pub supply: i128,
}

impl PoolState {
    /// Residual allowance for one operation. Every rounding *direction* in the pool math is
    /// exact; `c_pow` is pool-favoring only to within one unit (1e-18) of the ratio
    /// (MATH_FIXES.md §4.4), worth `quantity × 1e-18` stroops of a result.
    pub fn tolerance(&self) -> i128 {
        self.b1.max(self.b2).max(self.supply) / 10i128.pow(18) + 1
    }
}

/// Property: value per LP share did not decrease. With normalized weights 0.8 / 0.2 the invariant
/// per share `b1^0.8 · b2^0.2 / S` is compared exactly by raising both sides to the fifth power:
/// `(b1' + t)^4 · (b2' + t) · S^5  >=  b1^4 · b2 · (S' - t)^5`, `t` = tolerance.
pub fn assert_value_per_share_non_decreasing(before: &PoolState, after: &PoolState, ctx: &str) {
    let t = before.tolerance().max(after.tolerance());
    let b1 = BigInt::from(after.b1 + t);
    let b2 = BigInt::from(after.b2 + t);
    let s_after = BigInt::from(after.supply - t);
    assert!(
        s_after > BigInt::zero(),
        "{ctx}: LP supply {} collapsed below tolerance {t}",
        after.supply
    );
    let lhs = b1.pow(4) * b2 * BigInt::from(before.supply).pow(5);
    let rhs = BigInt::from(before.b1).pow(4) * BigInt::from(before.b2) * s_after.pow(5);
    assert!(
        lhs >= rhs,
        "{ctx}: value per LP share decreased (tolerance {t})\n  before: {before:?}\n  after:  {after:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Contract call outcome classification
// ---------------------------------------------------------------------------------------------

/// `true` if the call succeeded; `false` if the contract rejected it with a typed error.
/// Panics on a raw contract panic (`WasmVm / InvalidAction`) or any other unexpected failure.
#[track_caller]
pub fn outcome<T, E: core::fmt::Debug>(
    ctx: &str,
    r: Result<Result<T, E>, Result<Error, InvokeError>>,
) -> bool {
    match r {
        Ok(Ok(_)) => true,
        Ok(Err(e)) => panic!("{ctx}: return value conversion failed: {e:?}"),
        Err(Ok(e)) => {
            // A raw `panic!` / failed `unwrap` inside the contract surfaces as `InvalidAction`:
            // `Context` in native test execution, `WasmVm` when running compiled WASM.
            if e.is_code(ScErrorCode::InvalidAction)
                && (e.is_type(ScErrorType::Context) || e.is_type(ScErrorType::WasmVm))
            {
                panic!("{ctx}: contract panicked ({e:?}) — raw panic or unwrap?");
            }
            assert!(
                e.is_type(ScErrorType::Contract),
                "{ctx}: unexpected non-contract error {e:?}"
            );
            if tracing() {
                eprintln!("reject {ctx} -> {e:?}");
            }
            false
        }
        Err(Err(e)) => panic!("{ctx}: invoke error {e:?}"),
    }
}

/// Runs the two pool-level property checks around one operation and returns whether it succeeded.
/// `state` is advanced and `tol` accumulated on success.
pub struct Stepper<'f, 'a> {
    pub f: &'f Fixture<'a>,
    pub state: PoolState,
    pub ops: i128,
    pub tol: i128,
}

impl<'f, 'a> Stepper<'f, 'a> {
    pub fn new(f: &'f Fixture<'a>) -> Self {
        f.assert_accounting("init");
        Stepper {
            f,
            state: f.state(),
            ops: 0,
            tol: 0,
        }
    }

    pub fn step(&mut self, ctx: &str, ok: bool) -> bool {
        self.f.assert_accounting(ctx);
        if ok {
            let next = self.f.state();
            assert_value_per_share_non_decreasing(&self.state, &next, ctx);
            self.tol += self.state.tolerance().max(next.tolerance());
            self.state = next;
            self.ops += 1;
        }
        ok
    }
}
