#![cfg(test)]
#[warn(dead_code)]
extern crate std;

use crate::oracle;
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    token, Address, Env, IntoVal, Symbol,
};

use crate::{EuropeanCallOption, EuropeanCallOptionClient};

fn create_european_call_contract<'a>(e: &Env) -> EuropeanCallOptionClient<'a> {
    let european_call =
        EuropeanCallOptionClient::new(e, &e.register_contract(None, EuropeanCallOption {}));

    european_call
}

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::AdminClient<'a>) {
    let addr = e.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(e, &addr),
        token::AdminClient::new(e, &addr),
    )
}

fn create_option_contract<'a>(
    e: &Env,
    seller: &Address,
    escrow_token: &Address,
    underlying_token: &Address,
    strike_price: u32,
    premium: u32,
    escrow_amount: u32,
    expiration_date: u64,
    oracle_id: &Address,
) -> EuropeanCallOptionClient<'a> {
    let option = create_european_call_contract(&e);

    option.init_option(
        seller,
        &strike_price,
        &expiration_date,
        &premium,
        escrow_token,
        &escrow_amount,
        underlying_token,
        oracle_id,
    );

    // Verify that authorization is required for the seller.
    assert_eq!(
        e.auths(),
        std::vec![(
            seller.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    option.address.clone(),
                    Symbol::new(&e, "init_option"),
                    (
                        seller,
                        strike_price,
                        expiration_date,
                        premium,
                        escrow_token.clone(),
                        escrow_amount,
                        underlying_token.clone(),
                        oracle_id.clone()
                    )
                        .into_val(e)
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        escrow_token.clone(),
                        Symbol::new(&e, "transfer"),
                        (seller.clone(), option.address.clone(), 100_i128,).into_val(e)
                    )),
                    sub_invocations: std::vec![]
                },]
            }
        )]
    );

    option
}

#[test]
fn test_in_case_of_strike_is_high() {
    let e = Env::default();
    e.mock_all_auths();

    e.ledger().with_mut(|li| {
        li.timestamp = 12345;
    });

    let token_admin = Address::random(&e);
    let seller = Address::random(&e);
    let buyer = Address::random(&e);

    let escrow_token = create_token_contract(&e, &token_admin);
    let escrow_token_client = escrow_token.0;
    let escrow_token_admin_client = escrow_token.1;

    let underlying_token = create_token_contract(&e, &token_admin);
    let underlying_token_client = underlying_token.0;
    let underlying_admin_client = underlying_token.1;
    // Mint tokens
    escrow_token_admin_client.mint(&seller, &1000);
    escrow_token_admin_client.mint(&buyer, &100);
    underlying_admin_client.mint(&buyer, &10000);

    std::println!("timeStamp {}", e.ledger().timestamp());

    //Init Oracle contract
    let oracle_id = e.register_contract_wasm(None, oracle::WASM);
    let oracle_client = oracle::Client::new(&e, &oracle_id);
    oracle_client.initialize(
        &Address::random(&e),
        &oracle::Asset::Stellar(Address::random(&e)),
        &18,
        &60,
    );

    let price: i128 = 9;
    oracle_client.add_price(
        &0,
        &oracle::Asset::Stellar(escrow_token_client.address.clone()),
        &price,
    );

    let euro_option = create_option_contract(
        &e,
        &seller,
        &escrow_token_client.address,
        &underlying_token_client.address,
        10,
        10,
        100,
        0,
        &oracle_id,
    );

    // check seller's balance after deposit escrow amount
    assert_eq!(escrow_token_client.balance(&seller), 900);
    std::println!("balance check ok");
    
    // Verify that authorization is required for the buyer.
    euro_option.buy_option(&buyer);

    assert_eq!(
        e.auths(),
        std::vec![(
            buyer.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    euro_option.address.clone(),
                    Symbol::new(&e, "buy_option"),
                    (&buyer,).into_val(&e),
                )),
                sub_invocations: std::vec![
                    AuthorizedInvocation {
                        function: AuthorizedFunction::Contract((
                            escrow_token_client.address.clone(),
                            Symbol::new(&e, "transfer"),
                            (buyer.clone(), seller.clone(), 10_i128,).into_val(&e)
                        )),
                        sub_invocations: std::vec![]
                    },
                    AuthorizedInvocation {
                        function: AuthorizedFunction::Contract((
                            underlying_token_client.address.clone(),
                            Symbol::new(&e, "transfer"),
                            (buyer.clone(), euro_option.address.clone(), 1000_i128,).into_val(&e)
                        )),
                        sub_invocations: std::vec![]
                    },
                ]
            }
        )]
    );

    // check buyer's balance after buying option
    assert_eq!(underlying_token_client.balance(&buyer), 9000);
    assert_eq!(escrow_token_client.balance(&buyer), 90);

    // Check with Buyer
    euro_option.exercise_option();
    assert_eq!(
        e.auths(),
        std::vec![(
            buyer.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    euro_option.address.clone(),
                    Symbol::new(&e, "exercise_option"),
                    ().into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    assert_eq!(underlying_token_client.balance(&buyer), 10000);
    assert_eq!(underlying_token_client.balance(&seller), 0);
    assert_eq!(escrow_token_client.balance(&buyer), 90);
    assert_eq!(escrow_token_client.balance(&seller), 1010);

    assert_eq!(escrow_token_client.balance(&euro_option.address), 0);
    assert_eq!(underlying_token_client.balance(&euro_option.address), 0);
}

#[test]
fn test_in_case_of_strike_is_low() {
    let e = Env::default();
    e.mock_all_auths();

    e.ledger().with_mut(|li| {
        li.timestamp = 12345;
    });

    let token_admin = Address::random(&e);
    let seller = Address::random(&e);
    let buyer = Address::random(&e);

    let escrow_token = create_token_contract(&e, &token_admin);
    let escrow_token_client = escrow_token.0;
    let escrow_token_admin_client = escrow_token.1;

    let underlying_token = create_token_contract(&e, &token_admin);
    let underlying_token_client = underlying_token.0;
    let underlying_admin_client = underlying_token.1;
    // Mint tokens
    escrow_token_admin_client.mint(&seller, &1000);
    escrow_token_admin_client.mint(&buyer, &100);
    underlying_admin_client.mint(&buyer, &10000);

    std::println!("timeStamp {}", e.ledger().timestamp());

    //Init Oracle contract
    let oracle_id = e.register_contract_wasm(None, oracle::WASM);
    let oracle_client = oracle::Client::new(&e, &oracle_id);
    oracle_client.initialize(
        &Address::random(&e),
        &oracle::Asset::Stellar(Address::random(&e)),
        &18,
        &60,
    );

    let price: i128 = 12;
    oracle_client.add_price(
        &0,
        &oracle::Asset::Stellar(escrow_token_client.address.clone()),
        &price,
    );

    let euro_option = create_option_contract(
        &e,
        &seller,
        &escrow_token_client.address,
        &underlying_token_client.address,
        10,
        10,
        100,
        0,
        &oracle_id,
    );

    // check seller's balance after deposit escrow amount
    assert_eq!(escrow_token_client.balance(&seller), 900);
    std::println!("balance check ok");
    // Verify that authorization is required for the buyer.

    euro_option.buy_option(&buyer);
    assert_eq!(
        e.auths(),
        std::vec![(
            buyer.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    euro_option.address.clone(),
                    Symbol::new(&e, "buy_option"),
                    (&buyer,).into_val(&e),
                )),
                sub_invocations: std::vec![
                    AuthorizedInvocation {
                        function: AuthorizedFunction::Contract((
                            escrow_token_client.address.clone(),
                            Symbol::new(&e, "transfer"),
                            (buyer.clone(), seller.clone(), 10_i128,).into_val(&e)
                        )),
                        sub_invocations: std::vec![]
                    },
                    AuthorizedInvocation {
                        function: AuthorizedFunction::Contract((
                            underlying_token_client.address.clone(),
                            Symbol::new(&e, "transfer"),
                            (buyer.clone(), euro_option.address.clone(), 1000_i128,).into_val(&e)
                        )),
                        sub_invocations: std::vec![]
                    },
                ]
            }
        )]
    );
    std::println!("Test step 1 check ok");
    // check buyer's balance after buying option
    assert_eq!(underlying_token_client.balance(&buyer), 9000);
    assert_eq!(escrow_token_client.balance(&buyer), 90);

    // check seller's balance after buying option
    assert_eq!(escrow_token_client.balance(&seller), 910);

    assert_eq!(escrow_token_client.balance(&euro_option.address), 100);
    assert_eq!(underlying_token_client.balance(&euro_option.address), 1000);

    // Check with seller
    euro_option.exercise_option();
    assert_eq!(
        e.auths(),
        std::vec![(
            seller.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    euro_option.address.clone(),
                    Symbol::new(&e, "exercise_option"),
                    ().into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    assert_eq!(underlying_token_client.balance(&buyer), 9000);
    assert_eq!(underlying_token_client.balance(&seller), 1000);
    assert_eq!(escrow_token_client.balance(&buyer), 190);
    assert_eq!(escrow_token_client.balance(&seller), 910);

    assert_eq!(escrow_token_client.balance(&euro_option.address), 0);
    assert_eq!(underlying_token_client.balance(&euro_option.address), 0);
    std::println!("Test step 2 check ok");
}

#[test]
fn test_withdraw() {
    let e = Env::default();
    e.mock_all_auths();

    e.ledger().with_mut(|li| {
        li.timestamp = 12345;
    });

    let token_admin = Address::random(&e);
    let seller = Address::random(&e);
    let buyer = Address::random(&e);

    let escrow_token = create_token_contract(&e, &token_admin);
    let escrow_token_client = escrow_token.0;
    let escrow_token_admin_client = escrow_token.1;

    let underlying_token = create_token_contract(&e, &token_admin);
    let underlying_token_client = underlying_token.0;
    let underlying_admin_client = underlying_token.1;
    // Mint tokens
    escrow_token_admin_client.mint(&seller, &1000);
    escrow_token_admin_client.mint(&buyer, &100);
    underlying_admin_client.mint(&buyer, &10000);

    std::println!("timeStamp {}", e.ledger().timestamp());

    //Init Oracle contract
    let oracle_id = e.register_contract_wasm(None, oracle::WASM);
    let oracle_client = oracle::Client::new(&e, &oracle_id);
    oracle_client.initialize(
        &Address::random(&e),
        &oracle::Asset::Stellar(Address::random(&e)),
        &18,
        &60,
    );

    let price: i128 = 12;
    oracle_client.add_price(
        &0,
        &oracle::Asset::Stellar(escrow_token_client.address.clone()),
        &price,
    );

    let euro_option = create_option_contract(
        &e,
        &seller,
        &escrow_token_client.address,
        &underlying_token_client.address,
        10,
        10,
        100,
        0,
        &oracle_id,
    );

    // check seller and contract's balance after option initiation
    assert_eq!(escrow_token_client.balance(&seller), 900);
    assert_eq!(
        escrow_token_client.balance(&euro_option.address.clone()),
        100
    );

    euro_option.withdraw();

    assert_eq!(
        e.auths(),
        std::vec![(
            seller.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    euro_option.address.clone(),
                    Symbol::new(&e, "withdraw"),
                    ().into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(escrow_token_client.balance(&seller), 1000);
    assert_eq!(escrow_token_client.balance(&euro_option.address.clone()), 0);
}
