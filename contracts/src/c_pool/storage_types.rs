//! Declaration of the Storage Keys
use soroban_sdk::{contracttype, Address, Map, Vec};

pub(crate) const INSTANCE_BUMP_THRESHOLD: u32 = 34560; // 2 days
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 69120; // 4 days
pub(crate) const BALANCE_BUMP_THRESHOLD: u32 = 518400; // 30 days
pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 535670; // 45 days

// Token Details Struct
#[contracttype]
#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct Record {
    pub bound: bool,
    pub index: u32,
    pub denorm: i128,
    pub balance: i128,
}

// Data Keys for Pool' Storage Data
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Factory,       // Address of the Factory Contract
    Controller,    // Address of the Controller Account
    SwapFee,       // i128
    TotalWeight,   // i128
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
