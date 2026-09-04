# Comet fuzz suite

Property-based fuzzing of the pool contract with [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz),
following the [Stellar fuzzing guide](https://developers.stellar.org/docs/build/smart-contracts/example-contracts/fuzzing).

## Setup

```sh
cargo install cargo-fuzz
rustup install nightly
```

## Run

From the repository root:

```sh
make fuzz-deposit-withdraw                 # run until interrupted
make fuzz-swap FUZZ_TIME=300               # run for 300 seconds
make fuzz                                  # alias for fuzz-deposit-withdraw
```

or directly:

```sh
cd fuzz
cargo +nightly fuzz run -s none fuzz_deposit_withdraw -- -max_total_time=60 -max_len=128 -len_control=0
```

`-max_len=128 -len_control=0` pins the input length from the start. The fuzz input decodes to
about 60 bytes; libFuzzer otherwise begins with a few bytes and grows slowly, and a truncated
input decodes every enum to its first variant, so early runs are almost all proportional joins.

`-s none` disables the address sanitizer. It is required on macOS, where ASAN's module
constructors conflict with dead-stripping when `soroban-env-host` is linked; ASAN adds little for
pure Rust in any case. On Linux it can be omitted.

Set `FUZZ_TRACE=1` to print one line per run showing how many operations succeeded and in which
forms (useful when tuning input ranges).

Failing inputs are written to `fuzz/artifacts/<target>/`. Reproduce one with:

```sh
cd fuzz
cargo +nightly fuzz run -s none fuzz_deposit_withdraw artifacts/fuzz_deposit_withdraw/<file>
```

## Layout

- `src/lib.rs` mounts the contract sources (see below).
- `src/common.rs` holds the shared fixture (`Fixture`), the property checks
  (`assert_accounting`, `assert_value_per_share_non_decreasing`), the `Stepper` that runs them
  around every operation, input primitives (`Amount`, `Supply`, `Token`), and `outcome`, which
  classifies a `try_*` result as success, typed rejection, or raw panic.
- `fuzz_targets/*.rs` are the targets; each documents its scenario and properties at the top.

## How the contract is linked

The fuzz crate does not depend on the `contracts` package. It compiles the contract sources
directly via `#[path]` in `src/lib.rs`. The contract is a `cdylib`, and cargo-fuzz's coverage
instrumentation cannot link a standalone dylib on macOS. Mounting the modules keeps the contract
manifest untouched and gives native speed with real panic messages. The `soroban-sdk`,
`soroban-token-sdk`, and `soroban-fixed-point-math` versions in `fuzz/Cargo.toml` must match
`contracts/Cargo.toml`.

## Targets

| Target | Scenario | Properties |
|---|---|---|
| `fuzz_deposit_withdraw` | User B takes three fuzzed join / exit / single-sided actions, then exits flat (dust swept through the single-sided path). | Accounting exact; value per LP share non-decreasing; once B holds no LP, if every action used the same value form B holds no more of either token than they started with, otherwise B cannot have gained both. |
| `fuzz_swap` | User B makes three fuzzed swaps (exact-in or exact-out, either direction), then a close-out swap returns token 2 to its starting balance. | Accounting exact; invariant `b1^0.8 · b2^0.2` non-decreasing with LP supply constant; if the close-out succeeded B holds no more token 1 than they started with; B cannot have gained both. Dust `swap_exact_amount_in` calls whose output rounds to zero are skipped and counted (`dust_skip`) because the contract hits a raw divide-by-zero there — see REVIEW.md 1.9. |

Both share the fixture: an 80/20 pool at the minimum swap fee over two Stellar Asset Contracts
with fuzzed supplies up to `i64::MAX / 2`, split 20% pool / 30% user A / 50% user B, with user A
joining for a fuzzed LP amount first. Tolerance is one unit per 1e18 of the largest pool quantity
per operation, for the sub-unit `c_pow` residual documented in `MATH_FIXES.md` §4.4.
