//! #145: Negative-path tests for `register_agent`.

#![cfg(test)]

use super::{test_support, Error};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

#[test]
fn register_agent_duplicate_owner_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = test_support::token(&e);
    let market = test_support::init_client(&e, &admin, &usdc);
    let skills = Vec::from_array(&e, [String::from_str(&e, "nlp")]);
    market
        .register_agent(
            &owner,
            &String::from_str(&e, "A"),
            &String::from_str(&e, "d"),
            &skills,
            &50_000_000,
            &String::from_str(&e, "ipfs://a"),
        )
        .unwrap();
    let again = market.try_register_agent(
        &owner,
        &String::from_str(&e, "B"),
        &String::from_str(&e, "d2"),
        &skills,
        &50_000_000,
        &String::from_str(&e, "ipfs://b"),
    );
    assert_eq!(again, Err(Ok(Error::AgentAlreadyExists)));
}

#[test]
fn register_agent_invalid_budget_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = test_support::token(&e);
    let market = test_support::init_client(&e, &admin, &usdc);
    let result = market.try_register_agent(
        &owner,
        &String::from_str(&e, "A"),
        &String::from_str(&e, "d"),
        &Vec::from_array(&e, [String::from_str(&e, "nlp")]),
        &1,
        &String::from_str(&e, "ipfs://a"),
    );
    assert_eq!(result, Err(Ok(Error::InvalidBudget)));
}

#[test]
fn register_agent_while_paused_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = test_support::token(&e);
    let market = test_support::init_client(&e, &admin, &usdc);
    market.set_paused(&admin, &true).unwrap();
    let result = market.try_register_agent(
        &owner,
        &String::from_str(&e, "A"),
        &String::from_str(&e, "d"),
        &Vec::from_array(&e, [String::from_str(&e, "nlp")]),
        &50_000_000,
        &String::from_str(&e, "ipfs://a"),
    );
    assert_eq!(result, Err(Ok(Error::ContractPaused)));
}