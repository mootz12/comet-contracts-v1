//! Declaration of the Storage Keys
use soroban_sdk::{contracttype, Address};

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;

pub(crate) const SHARED_BUMP_AMOUNT: u32 = 31 * DAY_IN_LEDGERS;
pub(crate) const SHARED_LIFETIME_THRESHOLD: u32 = SHARED_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 120 * DAY_IN_LEDGERS;
pub(crate) const BALANCE_LIFETIME_THRESHOLD: u32 = BALANCE_BUMP_AMOUNT - 20 * DAY_IN_LEDGERS;

// Token Details Struct
#[contracttype]
#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct Record {
    pub balance: i128,
    pub weight: i128,
    pub scalar: i128,
    pub index: u32,
}

// Data Keys for Pool' Storage Data
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Factory,       // Address of the Factory Contract
    Controller,    // Address of the Controller Account
    SwapFee,       // i128
    AllTokenVec,   // Vec<Address>
    AllRecordData, // Map<Address, Record>
    TokenShare,    // Address
    TotalShares,   // i128
    PublicSwap,    // bool
    Finalize,      // bool
    Freeze,        // bool
}

// Data Keys for the LP Token
#[derive(Clone)]
#[contracttype]
pub enum DataKeyToken {
    Allowance(AllowanceDataKey),
    Balance(Address),
    Nonce(Address),
    State(Address),
    Admin,
}

#[derive(Clone)]
#[contracttype]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[contracttype]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}
