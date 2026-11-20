#![cfg(test)]
use super::{test_support, Error};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String, Vec};
#[test]
fn submit_deliverable_unauthorized() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let client = Address::generate(&e);
    let agent = Address::generate(&e);
    let usdc = test_support::token(&e);
    let m = test_support::init_client(&e, &admin, &usdc);
    m.register_agent(&agent, &String::from_str(&e, "A"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &50_000_000, &String::from_str(&e, "ipfs://a")).unwrap();
    let job = m.post_job(&client, &String::from_str(&e, "t"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &10_000_000, &7).unwrap();
    let hash = BytesN::from_array(&e, &[1u8; 32]);
    let r = m.try_submit_deliverable(&client, &job, &hash);
    assert_eq!(r, Err(Ok(Error::Unauthorized)));
}