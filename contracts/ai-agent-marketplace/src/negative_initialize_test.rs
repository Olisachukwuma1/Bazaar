//! #144: Negative-path tests for `initialize`.

#![cfg(test)]

use super::{test_support, AiAgentMarketplace, AiAgentMarketplaceClient, DataKey, Error};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

#[test]
fn initialize_already_initialized_unchanged() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let usdc = test_support::token(&e);
    let market = test_support::init_client(&e, &admin, &usdc);
    let before = market.get_config().platform_fee_bps;
    let result = market.try_initialize(
        &admin,
        &usdc,
        &200,
        &10_000_000,
        &30,
        &7,
        &admin,
        &admin,
        &0,
        &200,
        &30,
        &Vec::new(&e),
        &false,
    );
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
    assert_eq!(market.get_config().platform_fee_bps, before);
}

#[test]
fn initialize_invalid_config_leaves_uninitialized() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let usdc = test_support::token(&e);
    let id = e.register_contract(None, AiAgentMarketplace);
    let client = AiAgentMarketplaceClient::new(&e, &id);
    let result = client.try_initialize(
        &admin,
        &usdc,
        &10_001,
        &10_000_000,
        &30,
        &7,
        &admin,
        &admin,
        &0,
        &200,
        &30,
        &Vec::new(&e),
        &false,
    );
    assert_eq!(result, Err(Ok(Error::InvalidConfig)));
    assert!(!e.storage().instance().has(&DataKey::Config));
}