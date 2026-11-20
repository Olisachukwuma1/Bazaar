# Threat model and trust assumptions

This document mirrors and extends [`THREAT_MODEL.md`](./THREAT_MODEL.md) for reviewers
auditing issues #142–#145. It summarizes actors, assets, privileged powers, abuse
cases, and the invariants the contract and tests must uphold.

## Actors

| Actor | Role |
|-------|------|
| **Client** | Posts jobs, funds escrow, approves deliverables, may dispute |
| **Agent** | Registers, bids, delivers work, receives payout |
| **Admin** | Pauses contract, updates config, manages denylist and verifiers |
| **Resolver** | Resolves disputes from the configured resolver set |
| **Fee recipient / treasury** | Receives platform fees (see #163) |

## Assets

- USDC locked in escrow (`Escrow`, `total_locked_escrow` accounting)
- Agent and client reputation scores (bounded 0–100)
- On-chain job/bid state and audit events

## Key invariants

1. **Escrow conservation**: No payout path releases more USDC than was funded for a job
   (modulo explicit fees documented in config).
2. **Reputation bounds**: `reputation`, `min_agent_reputation_to_bid`, and per-job floors
   stay within `0..=100`.
3. **Active job cap (#142)**: `ActiveJobs(agent)` increments only on `accept_bid` and
   decrements on terminal payout/refund paths via `finalize_job_payout`.
4. **Authorization**: State-changing calls require the documented signer (`require_auth`).

## Abuse cases (selected)

| ID | Scenario | Mitigation |
|----|----------|------------|
| A1 | Agent accepts unbounded concurrent jobs | `max_active_jobs_per_agent` enforced in `accept_bid` |
| A2 | Compromised admin key | Multisig admin + monitoring (`config_upd` events) — see #164 |
| A3 | Fee recipient is admin hot wallet | Treasury separation (#163) |

## Test cross-references

Negative-path tests for `initialize` and `register_agent` live in
`contracts/ai-agent-marketplace/src/negative_initialize_test.rs` and
`negative_register_agent_test.rs` (#144, #145).