#![cfg(test)]

use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env, Vec};

use super::{AiAgentMarketplace, AiAgentMarketplaceClient};

pub fn token(e: &Env) -> Address {
    let issuer = Address::generate(e);
    e.register_stellar_asset_contract_v2(issuer).address()
}

pub fn init_client(e: &Env, admin: &Address, usdc: &Address) -> AiAgentMarketplaceClient<'_> {
    let id = e.register_contract(None, AiAgentMarketplace);
  let client = AiAgentMarketplaceClient::new(e, &id);
    client
        .initialize(
            admin,
            usdc,
            &100,
            &10_000_000,
            &30,
            &7,
            admin,
            admin,
            &0,
            &200,
            &30,
            &Vec::new(e),
            &false,
        )
        .expect("initialize");
    client
}