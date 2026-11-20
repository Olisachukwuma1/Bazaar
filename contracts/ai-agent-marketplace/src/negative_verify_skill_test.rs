#![cfg(test)]
use super::{test_support, Error};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
#[test]
fn verify_skill_unauthorized_verifier() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let bad = Address::generate(&e);
    let usdc = test_support::token(&e);
    let m = test_support::init_client(&e, &admin, &usdc);
    let id = m.register_agent(&owner, &String::from_str(&e, "A"), &String::from_str(&e, "d"), &Vec::from_array(&e, [String::from_str(&e, "nlp")]), &50_000_000, &String::from_str(&e, "ipfs://a")).unwrap();
    let r = m.try_verify_skill(&bad, &id, &String::from_str(&e, "nlp"), &String::from_str(&e, "ipfs://p"), &(e.ledger().timestamp() + 86400));
    assert_eq!(r, Err(Ok(Error::VerifierNotAuthorized)));
}