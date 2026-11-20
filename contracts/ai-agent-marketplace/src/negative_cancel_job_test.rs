#![cfg(test)]
use super::{test_support, Error};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
#[test]
fn cancel_job_unauthorized() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let client = Address::generate(&e);
    let stranger = Address::generate(&e);
    let usdc = test_support::token(&e);
    let m = test_support::init_client(&e, &admin, &usdc);
    let job = m.post_job(&client, &String::from_str(&e, "t"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &10_000_000, &7).unwrap();
    let r = m.try_cancel_job(&stranger, &job);
    assert_eq!(r, Err(Ok(Error::Unauthorized)));
}