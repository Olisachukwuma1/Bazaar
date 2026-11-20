#![cfg(test)]
use super::{test_support, Error};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
#[test]
fn post_job_insufficient_budget() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let client = Address::generate(&e);
    let usdc = test_support::token(&e);
    let m = test_support::init_client(&e, &admin, &usdc);
    let r = m.try_post_job(&client, &String::from_str(&e, "t"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &1, &7);
    assert_eq!(r, Err(Ok(Error::InsufficientBudget)));
}
#[test]
fn post_job_invalid_deadline() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let client = Address::generate(&e);
    let usdc = test_support::token(&e);
    let m = test_support::init_client(&e, &admin, &usdc);
    let r = m.try_post_job(&client, &String::from_str(&e, "t"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &10_000_000, &0);
    assert_eq!(r, Err(Ok(Error::InvalidDeadline)));
}