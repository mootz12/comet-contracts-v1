//! Fuzzing support for the Comet pool contract.
//!
//! The contract sources are compiled directly into this crate with `#[path]` instead of depending
//! on the `contracts` package. The contract is a `cdylib`, and cargo-fuzz's coverage
//! instrumentation cannot link a standalone dylib on macOS (initializer pointers are dead-stripped
//! and the coverage hooks only exist in the final fuzzer binary). Mounting the modules keeps the
//! contract manifest untouched and gives native speed with real panic messages.
//!
//! `crate::` paths inside the contract sources resolve against this crate root, which is why the
//! module names below must match `contracts/src/lib.rs` exactly.

#![allow(dead_code, unused_imports)]

#[path = "../../contracts/src/c_consts.rs"]
pub mod c_consts;
#[path = "../../contracts/src/c_math.rs"]
pub mod c_math;
#[path = "../../contracts/src/c_num.rs"]
pub mod c_num;
#[path = "../../contracts/src/c_pool/mod.rs"]
pub mod c_pool;

pub mod common;
