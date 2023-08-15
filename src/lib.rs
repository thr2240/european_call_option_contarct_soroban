//! This contract implements European Call option

#![no_std]

mod option;
mod storage_types;
mod oracle {
    soroban_sdk::contractimport!(
        file = "../oracle_soroban/target/wasm32-unknown-unknown/release/oracle.wasm"
    );
}

use crate::option::{
    check_time_bound, deposite_escrow, get_buyer, is_buyer_entered, is_initialized, load_option,
    save_option, set_buyer, set_init_time,
};
use crate::storage_types::{DataKey, OptionInfo};
use soroban_sdk::{contract, contractimpl, token, unwrap::UnwrapOptimized, Address, Env, log};

#[contract]
pub struct EuropeanCallOption;

#[contractimpl]
impl EuropeanCallOption {
    pub fn init_option(
        e: Env,
        seller: Address,
        strike_price: u32,
        expiration_date: u64,
        premium: u32,
        escrow_token: Address,
        escrow_amount: u32,
        underlying_token: Address,
        oracle_contract_id: Address,
    ) {
        if is_initialized(&e) {
            panic!("Option was already initialized");
        }
        if strike_price == 0 || escrow_amount == 0 {
            panic!("Zero value is not allowed");
        }
        save_option(
            &e,
            &OptionInfo {
                seller,
                escrow_token,
                escrow_amount,
                underlying_token,
                strike_price,
                expiration_date,
                premium,
                oracle_contract_id,
            },
        );
        deposite_escrow(&e);
        set_init_time(&e, &e.ledger().timestamp());
    }
    pub fn buy_option(e: Env, buyer: Address) {
        if !is_initialized(&e) {
            panic!("Option was not initialized");
        }
        let option = load_option(&e);

        let premium_token_client = token::Client::new(&e, &option.escrow_token);
        let underlying_token_client = token::Client::new(&e, &option.underlying_token);
        let contract = e.current_contract_address();

        buyer.require_auth();

        // Send premium to the seller
        premium_token_client.transfer(&buyer, &option.seller, &(option.premium as i128));

        log!(&e, "option.premium {}");
        // Depositing escrowAmount * strikePrice
        let deposit_amount = option
            .escrow_amount
            .checked_mul(option.strike_price)
            .unwrap_optimized() as i128;

        underlying_token_client.transfer(&buyer, &contract, &deposit_amount);

        // Set buyer of this option
        set_buyer(&e, &buyer);
    }

    pub fn exercise_option(e: Env) {
        if !check_time_bound(&e) {
            panic!("Expiration Date is not fulfilled");
        }

        if !is_initialized(&e) || !is_buyer_entered(&e) {
            panic!("Option has not been initialized");
        }
        let option = load_option(&e);
        let contract = e.current_contract_address();
        let underlying_token_client = token::Client::new(&e, &option.underlying_token);
        let escrow_token_client = token::Client::new(&e, &option.escrow_token);
        let deposited_amount = option
            .escrow_amount
            .checked_mul(option.strike_price)
            .unwrap_optimized() as i128;
        let buyer = get_buyer(&e);

        let oracle_client = oracle::Client::new(&e, &option.oracle_contract_id);
        let asset = oracle::Asset::Stellar(escrow_token_client.address.clone());
        let current_price = oracle_client.lastprice(&asset);

        if let Some(price_data) = current_price {
            if price_data.price < option.strike_price as i128 {
                // Send strikePrice * escrowAmount to buyer
                buyer.require_auth();
                underlying_token_client.transfer(&contract, &buyer, &deposited_amount);

                // Send escrowAmount to seller
                escrow_token_client.transfer(
                    &contract,
                    &option.seller,
                    &(option.escrow_amount as i128),
                );
            } else {
                option.seller.require_auth();
                // Send strikePrice * escrowAmount to seller
                underlying_token_client.transfer(&contract, &option.seller, &deposited_amount);

                // Send escrowAmount to buyer
                escrow_token_client.transfer(&contract, &buyer, &(option.escrow_amount as i128));
            }
            e.storage().instance().remove(&DataKey::OptionInfo);
            e.storage().instance().remove(&DataKey::Buyer);
        } else {
            panic!("Failed to fetch price from Oracle");
        }
    }

    pub fn withdraw(e: Env) {
        let option = load_option(&e);
        if is_initialized(&e) && is_buyer_entered(&e) {
            panic!("Seller can't withdraw funds");
        }

        if !is_initialized(&e) {
            panic!("Option wasn't initialized yet");
        }

        option.seller.require_auth();
        token::Client::new(&e, &option.escrow_token).transfer(
            &e.current_contract_address(),
            &option.seller,
            &(option.escrow_amount as i128),
        );
        e.storage().instance().remove(&DataKey::OptionInfo);
    }
}

mod test;
