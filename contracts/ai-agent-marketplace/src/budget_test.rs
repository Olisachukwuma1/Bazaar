//! #173: CPU and memory budget regression checks per entrypoint.
//!
//! Each entrypoint is invoked once on a realistic happy path and its metered
//! CPU instructions and memory bytes are compared against 70% of the mainnet
//! per-transaction limits.  Run with `cargo test budget -- --nocapture` to
//! print the table used in `docs/BUDGETS.md`.

#![cfg(test)]

extern crate std;

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token::StellarAssetClient,
    Address, BytesN, Env, String, Vec,
};

/// Mainnet per-transaction instruction limit.
const MAINNET_TX_MAX_INSTRUCTIONS: u64 = 100_000_000;
/// Mainnet per-transaction memory limit (40 MiB).
const MAINNET_TX_MAX_MEMORY_BYTES: u64 = 41_943_040;
/// No entrypoint may use more than this share of a limit.
const MAX_LIMIT_PCT: u64 = 70;

fn measure<T>(env: &Env, name: &str, f: impl FnOnce() -> T) -> T {
    env.budget().reset_unlimited();
    let out = f();
    let cpu = env.budget().cpu_instruction_cost();
    let mem = env.budget().memory_bytes_cost();
    let cpu_pct = cpu * 100 / MAINNET_TX_MAX_INSTRUCTIONS;
    let mem_pct = mem * 100 / MAINNET_TX_MAX_MEMORY_BYTES;

    std::println!("| {name} | {cpu} | {cpu_pct}% | {mem} | {mem_pct}% |");

    assert!(
        cpu * 100 <= MAINNET_TX_MAX_INSTRUCTIONS * MAX_LIMIT_PCT,
        "{name} uses {cpu} instructions ({cpu_pct}% of the mainnet limit)"
    );
    assert!(
        mem * 100 <= MAINNET_TX_MAX_MEMORY_BYTES * MAX_LIMIT_PCT,
        "{name} uses {mem} memory bytes ({mem_pct}% of the mainnet limit)"
    );
    out
}

fn skills(env: &Env) -> Vec<String> {
    let mut skills = Vec::new(env);
    skills.push_back(String::from_str(env, "nlp"));
    skills
}

#[test]
fn budget_per_entrypoint_within_mainnet_limits() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let client = Address::generate(&env);
    let agent = Address::generate(&env);

    let usdc = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let usdc_admin = StellarAssetClient::new(&env, &usdc);
    usdc_admin.mint(&client, &100_000_000_000);

    let contract_id = env.register_contract(None, AiAgentMarketplace);
    let market = AiAgentMarketplaceClient::new(&env, &contract_id);

    std::println!("| Entrypoint | CPU instructions | % of limit | Memory bytes | % of limit |");
    std::println!("|---|---|---|---|---|");

    measure(&env, "initialize", || {
        market.initialize(
            &admin,
            &usdc,
            &100,         // platform_fee_bps (1%)
            &10_000_000,  // min_job_budget (1 USDC)
            &30,          // max_job_deadline_days
            &7,           // dispute_window_days
            &0,           // min_agent_reputation
            &admin,       // platform_fee_recipient
            &admin,       // dispute_resolver
            &0,           // min_agent_reputation_to_bid
            &200,         // platform_fee_bps_dispute (2%)
            &30,          // escrow_expiry_days
            &Vec::new(&env),
            &false,
            &50,          // max_bids_per_job
        )
    });

    measure(&env, "register_agent", || {
        market.register_agent(
            &agent,
            &String::from_str(&env, "Agent"),
            &String::from_str(&env, "Summarises documents"),
            &skills(&env),
            &100_000_000,
            &String::from_str(&env, "ipfs://agent"),
        )
    });

    let budget: i128 = 1_000_000_000;
    let deliverable = BytesN::from_array(&env, &[7u8; 32]);

    // Happy path: post -> bid -> accept -> deliver -> approve -> review.
    let job_id = measure(&env, "post_job", || {
        market.post_job(
            &client,
            &String::from_str(&env, "Summarise"),
            &String::from_str(&env, "Summarise a report"),
            &skills(&env),
            &budget,
            &7,
        )
    });
    let bid_id = measure(&env, "place_bid", || {
        market.place_bid(&agent, &job_id, &budget, &5, &String::from_str(&env, "proposal"), &0)
    });
    measure(&env, "accept_bid", || market.accept_bid(&client, &job_id, &bid_id));

    // #129: accept_bid must stay bounded even when the job is at the bid cap,
    // since it marks every other bid Rejected.
    let max_bids: u32 = 50;
    let capped_job_id = market.post_job(
        &client,
        &String::from_str(&env, "Capped"),
        &String::from_str(&env, "Job at the bid cap"),
        &skills(&env),
        &budget,
        &7,
    );
    let mut capped_bid_id = 0u64;
    for _ in 0..max_bids {
        let bidder = Address::generate(&env);
        let id = market.place_bid(
            &bidder,
            &capped_job_id,
            &budget,
            &5,
            &String::from_str(&env, "p"),
            &0,
        );
        capped_bid_id = id;
    }
    measure(&env, "accept_bid (max bids)", || {
        market.accept_bid(&client, &capped_job_id, &capped_bid_id)
    });

    measure(&env, "submit_deliverable", || {
        market.submit_deliverable(&agent, &job_id, &deliverable)
    });
    measure(&env, "approve_deliverable", || market.approve_deliverable(&client, &job_id));
    measure(&env, "leave_review", || {
        market.leave_review(&client, &job_id, &5, &String::from_str(&env, "great"))
    });

    // Dispute path: post -> bid -> accept -> raise -> resolve.
    let job_id = market.post_job(
        &client,
        &String::from_str(&env, "Translate"),
        &String::from_str(&env, "Translate a report"),
        &skills(&env),
        &budget,
        &7,
    );
    let bid_id = market.place_bid(&agent, &job_id, &budget, &5, &String::from_str(&env, "p"), &0);
    market.accept_bid(&client, &job_id, &bid_id);
    let dispute_id = measure(&env, "raise_dispute", || {
        market.raise_dispute(
            &client,
            &job_id,
            &String::from_str(&env, "late"),
            &String::from_str(&env, "ipfs://evidence"),
        )
    });
    measure(&env, "resolve_dispute", || {
        market.resolve_dispute(&admin, &dispute_id, &String::from_str(&env, "split"), &Some(50))
    });

    // Cancellation and keeper paths.
    let job_id = market.post_job(
        &client,
        &String::from_str(&env, "Classify"),
        &String::from_str(&env, "Classify tickets"),
        &skills(&env),
        &budget,
        &7,
    );
    measure(&env, "bump_ttl", || market.bump_ttl(&job_id));
    measure(&env, "cancel_job", || market.cancel_job(&client, &job_id));

    // Admin and read paths.
    measure(&env, "pause", || market.pause(&admin));
    measure(&env, "unpause", || market.unpause(&admin));
    measure(&env, "add_to_denylist", || market.add_to_denylist(&admin, &Address::generate(&env)));
    measure(&env, "get_all_jobs", || market.get_all_jobs(&0, &50));
    measure(&env, "get_all_agents", || market.get_all_agents(&0, &50));
}
