use soroban_sdk::{ contracttype, Address};

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
    pub premium: u32
}


#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    OptionInfo,
    Buyer,
    InitTime,
}