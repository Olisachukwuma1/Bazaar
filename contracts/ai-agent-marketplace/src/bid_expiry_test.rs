#![cfg(test)]
use super::{test_support, Error};
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String, Vec};
#[test]
fn expired_bid_cannot_be_accepted() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let client = Address::generate(&e);
    let agent = Address::generate(&e);
    let usdc = test_support::token(&e);
    let m = test_support::init_client(&e, &admin, &usdc);
    m.register_agent(&agent, &String::from_str(&e, "A"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &50_000_000, &String::from_str(&e, "ipfs://a")).unwrap();
    let job = m.post_job(&client, &String::from_str(&e, "t"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &10_000_000, &7).unwrap();
    let now = e.ledger().timestamp();
    let bid = m.place_bid(&agent, &job, &8_000_000, &5, &String::from_str(&e, "p"), &(now + 86400)).unwrap();
    e.ledger().with_mut(|l| l.timestamp = now + 86400 + 1);
    let r = m.try_accept_bid(&client, &job, &bid);
    assert_eq!(r, Err(Ok(Error::BidExpired)));
}
#[test]
fn bid_validity_beyond_max_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let client = Address::generate(&e);
    let agent = Address::generate(&e);
    let usdc = test_support::token(&e);
    let m = test_support::init_client(&e, &admin, &usdc);
    m.set_bid_validity(&admin, &1, &2);
    m.register_agent(&agent, &String::from_str(&e, "A"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &50_000_000, &String::from_str(&e, "ipfs://a")).unwrap();
    let job = m.post_job(&client, &String::from_str(&e, "t"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &10_000_000, &7).unwrap();
    let now = e.ledger().timestamp();
    let r = m.try_place_bid(&agent, &job, &8_000_000, &5, &String::from_str(&e, "p"), &(now + 3 * 86400));
    assert_eq!(r, Err(Ok(Error::InvalidDeadline)));
}
