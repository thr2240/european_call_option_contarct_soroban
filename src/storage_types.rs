use soroban_sdk::{ contracttype, Address, String, Env};

pub(crate) const TEMPORARY_BUMP_AMOUNT: u32 = 17280; // 1 day
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 34560; // 2 days
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518400; // 30 days

#[derive(Clone)]
#[contracttype]
pub struct OptionInfo {
    // Owner of this option
    pub seller: Address,
    // Stoking token in escrow
    pub escrow_token: Address,
    // Underlying token in escrow
    pub underlying_token: Address,
    // Stoking amount in escrow
    pub escrow_amount: u32,
    // Strike price in this option
    pub strike_price: u32,
    // Timestamp of expiration Date
    pub expiration_date: u64,
    // Option Fee
    pub premium: u32,
    // Oracle contract id
    pub oracle_contract_id: Address
}


#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    OptionInfo,
    Buyer,
    InitTime,
}