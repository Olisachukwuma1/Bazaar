#![cfg(test)]
use super::{AiAgentMarketplace, AiAgentMarketplaceClient, Error};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};
#[test]
fn fee_above_cap_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let usdc = {
        let issuer = Address::generate(&e);
        e.register_stellar_asset_contract_v2(issuer).address()
    };
    let id = e.register_contract(None, AiAgentMarketplace);
    let c = AiAgentMarketplaceClient::new(&e, &id);
    c.initialize(admin.clone(), usdc.clone(), 100, 10_000_000, 30, 7, admin.clone(), admin.clone(), 0, 200, 30, Vec::new(&e), false).unwrap();
    let r = c.try_update_config(&admin, &Some(501u32), &None, &None, &None, &None, &None, &None, &None, &None, &None, &None, &None, &None);
    assert_eq!(r, Err(Ok(Error::InvalidConfig)));
}