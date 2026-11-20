#![cfg(test)]
use super::{test_support, Error};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
#[test]
fn update_agent_unauthorized() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let stranger = Address::generate(&e);
    let usdc = test_support::token(&e);
    let m = test_support::init_client(&e, &admin, &usdc);
    let skills = Vec::from_array(&e, [String::from_str(&e, "nlp")]);
    m.register_agent(&owner, &String::from_str(&e, "A"), &String::from_str(&e, "d"), &skills, &50_000_000, &String::from_str(&e, "ipfs://a")).unwrap();
    let r = m.try_update_agent(&stranger, &String::from_str(&e, "X"), &String::from_str(&e, "y"), &skills, &50_000_000, &String::from_str(&e, "ipfs://b"));
    assert_eq!(r, Err(Ok(Error::Unauthorized)));
}
#[test]
fn update_agent_invalid_budget() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = test_support::token(&e);
    let m = test_support::init_client(&e, &admin, &usdc);
    let skills = Vec::from_array(&e, [String::from_str(&e, "nlp")]);
    m.register_agent(&owner, &String::from_str(&e, "A"), &String::from_str(&e, "d"), &skills, &50_000_000, &String::from_str(&e, "ipfs://a")).unwrap();
    let r = m.try_update_agent(&owner, &String::from_str(&e, "A"), &String::from_str(&e, "d"), &skills, &1, &String::from_str(&e, "ipfs://a"));
    assert_eq!(r, Err(Ok(Error::InvalidBudget)));
}