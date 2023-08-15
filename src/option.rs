
use soroban_sdk::{ Env, Address, token};
use crate::storage_types::{ DataKey, OptionInfo, INSTANCE_BUMP_AMOUNT};

pub fn check_time_bound(e: &Env) -> bool {
    let option = load_option(e);

    let current_timestamp = e.ledger().timestamp();
    let init_timestamp = get_init_time(&e);
    current_timestamp >= (option.expiration_date + init_timestamp)
}

pub fn save_option(e: &Env, option: &OptionInfo) {
    e.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
    e.storage().instance().set(&DataKey::OptionInfo, option);
}

pub fn load_option(e: &Env) -> OptionInfo {
    e.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
    e.storage().instance().get(&DataKey::OptionInfo).unwrap()
}

pub fn set_buyer(e: &Env, buyer: &Address) {
    e.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
    e.storage().instance().set(&DataKey::Buyer, buyer);
}

pub fn get_buyer(e: &Env) -> Address {
    e.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
    e.storage().instance().get(&DataKey::Buyer).unwrap()
}

pub fn set_init_time(e: &Env, time: &u64) {
    e.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
    e.storage().instance().set(&DataKey::InitTime, time);
}

pub fn get_init_time(e: &Env) -> u64 {
    e.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
    e.storage().instance().get(&DataKey::InitTime).unwrap()
}


pub fn deposite_escrow(e: &Env) {
    let option = load_option(e);
    let seller = option.seller;
    let contract = e.current_contract_address();
    let escrow_token_client = token::Client::new(e, &option.escrow_token);
    seller.require_auth();
    escrow_token_client.transfer(&seller, &contract, &(option.escrow_amount as i128));
}

pub fn is_initialized(e: &Env) -> bool {
    e.storage().instance().has(&DataKey::OptionInfo)
}

pub fn is_buyer_entered(e: &Env) -> bool {
    e.storage().instance().has(&DataKey::Buyer)
}
