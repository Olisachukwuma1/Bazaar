//! AI Agent Marketplace - Soroban Smart Contract
//! 
//! A decentralized marketplace for AI agents with:
//! - Agent registration with skill verification
// - Job posting and bidding system
// - Escrow payments with Stellar USDC
// - Reputation and review system
// - Dispute resolution

#![no_std]
#![deny(clippy::unwrap_used)]
use soroban_sdk::{
    auth::Context,
    contract, contracterror, contractimpl, contracttype, symbol_short,
    Address, Bytes, BytesN, Env, Map, String, Symbol, Vec, IntoVal,
    token::TokenClient,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Agent {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub skills: Vec<String>,
    pub price_per_task: i128, // Price in USDC (7 decimals)
    pub reputation: u32,
    pub total_jobs_completed: u32,
    pub total_earnings: i128,
    pub is_active: bool,
    pub created_at: u64,
    pub skills_verified: Map<String, bool>,
    pub metadata_uri: String, // IPFS URI for agent details
    /// #120: Sum of all review ratings (1-5) received by this agent.  Used to
    /// derive `reputation` exactly, avoiding the rounding drift of a rolling
    /// integer average.
    pub rating_sum: u64,
    /// #120: Number of reviews counted in `rating_sum`.
    pub rating_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Job {
    pub id: u64,
    pub client: Address,
    pub title: String,
    pub description: String,
    pub required_skills: Vec<String>,
    pub budget: i128, // Budget in USDC (7 decimals)
    pub deadline: u64,
    pub status: JobStatus,
    pub assigned_agent: Option<Address>,
    // NOTE: bids are stored separately under DataKey::Bid(bid_id) and indexed
    // via DataKey::BidsByJob(job_id).  Use get_bids_by_job() to fetch them.
    pub escrow_amount: i128,
    pub created_at: u64,
    /// #112: ledger timestamp when a bid was accepted and work started.  Used
    /// as the start of the dispute window for jobs that never completed.
    pub accepted_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub deliverable_hash: Option<BytesN<32>>,
    pub dispute_raised: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobStatus {
    Open,
    InProgress,
    Completed,
    Disputed,
    Cancelled,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bid {
    pub agent: Address,
    pub agent_id: u64,
    pub amount: i128,
    pub proposed_timeline: u64,
    pub proposal: String,
    pub submitted_at: u64,
    pub status: BidStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BidStatus {
    Pending,
    Accepted,
    Rejected,
    Withdrawn,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub job_id: u64,
    pub client: Address,
    pub agent: Address,
    pub amount: i128,
    pub token: Address, // USDC token contract
    pub status: EscrowStatus,
    pub created_at: u64,
    pub released_at: Option<u64>,
    pub dispute_raised_at: Option<u64>,
    pub resolver: Option<Address>,
    /// #170: ledger timestamp after which the fallback resolution applies.
    pub dispute_deadline: Option<u64>,
}

/// #169: A balance owed to a recipient that could not be paid because they
/// had no USDC trustline at payout time.  The recipient can withdraw it later
/// via `claim_funds` once they can receive USDC.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Claimable {
    pub job_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub token: Address,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Funded,
    Released,
    Disputed,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    pub id: u64,
    pub job_id: u64,
    pub reviewer: Address, // Client or Agent
    pub reviewee: Address, // Agent or Client
    pub rating: u32, // 1-5
    pub comment: String,
    pub created_at: u64,
}

/// #121: Aggregated reputation for a client, derived from agent reviews and
/// job activity.  Exposed via `get_client_stats` so agents can screen clients
/// before bidding.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientStats {
    pub rating_sum: u32,
    pub rating_count: u32,
    pub jobs_posted: u32,
    pub disputes: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillVerification {
    pub skill: String,
    pub verified_by: Address, // Verifier address (auditor/DAO)
    pub verified_at: u64,
    pub evidence_uri: String, // IPFS hash of proof
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dispute {
    pub id: u64,
    pub job_id: u64,
    pub raised_by: Address,
    pub reason: String,
    pub evidence_uri: String,
    pub status: DisputeStatus,
    pub resolver: Option<Address>,
    /// #110: typed resolution outcome (agent / client / split percentage).
    pub resolution: Option<DisputeResolution>,
    pub created_at: u64,
    pub resolved_at: Option<u64>,
    /// #170: ledger timestamp after which either party may trigger the
    /// deterministic fallback resolution.
    pub dispute_deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    Open,
    UnderReview,
    Resolved,
    Escalated,
}

/// #110: Typed dispute resolution.  Replaces the free-form `String` argument
/// previously accepted by `resolve_dispute`, making invalid resolutions
/// impossible at the type level.  `Split` carries the mandatory percentage of
/// the payout (after fees) that goes to the agent.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeResolution {
    Agent,
    Client,
    Split(u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub admin: Address,
    pub usdc_token: Address,
    pub platform_fee_bps: u32, // Basis points (100 = 1%)
    pub min_job_budget: i128,
    pub max_job_deadline_days: u64,
    pub dispute_window_days: u64,
    pub platform_fee_recipient: Address,
    pub dispute_resolver: Address, // DAO or multisig
    pub min_agent_reputation_to_bid: u32,
    pub platform_fee_bps_dispute: u32, // Higher fee for disputes
    pub escrow_expiry_days: u64,
    /// #115: Number of days after a dispute is raised before anyone may call
    /// `claim_after_resolver_timeout` to apply the fallback resolution.
    pub resolution_deadline_days: u64,
    /// #115: Percentage of the (post-fee) payout sent to the agent when the
    /// resolver times out.  The remainder is refunded to the client.
    pub resolver_timeout_agent_pct: u32,
    pub skill_verifiers: Vec<Address>,
    /// When `true`, every required skill on a job must be verified on the
    /// bidding agent before the bid is accepted.  When `false`, unverified
    /// agents may bid freely; skills are treated as self-attested and the
    /// `skills_verified` map is purely informational.
    ///
    /// Default for new deployments should be `false` so that new agents are
    /// not blocked from the marketplace before a verifier network exists.
    pub require_skill_verification: bool,
    /// Contract storage schema version.  Bumped by `upgrade` so that off-chain
    /// tooling can detect which migration steps have been applied.
    pub version: u32,
    /// Maximum budget a single job may be posted with.  Used as a canary cap
    /// on mainnet to limit exposure while the contract is unproven.
    pub max_job_budget: i128,
    /// Maximum total value that may be locked in escrow at any one time.
    pub max_total_escrow: i128,
    /// Running total of funds currently locked in funded escrows.  Maintained
    /// alongside escrow state transitions so the cap can be enforced.
    pub total_locked_escrow: i128,
    /// #129: Maximum number of bids a single job may accumulate.  Bounds the
    /// cost of `accept_bid`, which must mark every other bid as Rejected.
    pub max_bids_per_job: u32,
    /// #119: Reputation assigned to a newly registered agent.  Defaults to 0.
    /// Combined with the zero-completed-jobs exemption in `place_bid`, this
    /// lets operators seed a starting reputation without blocking cold-start
    /// agents.
    pub starting_reputation: u32,
}

#[contracttype]
pub enum DataKey {
    Config,
    AgentCounter,
    JobCounter,
    BidCounter,
    ReviewCounter,
    DisputeCounter,
    EscrowCounter,
    VerificationCounter,
    Agent(u64),
    AgentByOwner(Address),
    Job(u64),
    JobsByClient(Address),
    JobsByAgent(Address),
    Bid(u64),
    BidsByJob(u64),
    BidsByAgent(Address),
    Escrow(u64),
    EscrowByJob(u64),
    Review(u64),
    ReviewsByReviewee(Address),
    /// #109: reviews written by a given reviewer.
    ReviewsByReviewer(Address),
    /// #123: (job_id, reviewer) -> review_id, for O(1) duplicate checks.
    ReviewByJobReviewer(u64, Address),
    /// #121: Aggregated client reputation.
    ClientStats(Address),
    Dispute(u64),
    DisputesByJob(u64),
    SkillVerification(u64),
    SkillVerificationsByAgent(Address),
    SkillAgents(String),
    PlatformStats,
    IsPaused,
    Denylist,
    /// #116: Admin-managed set of dispute resolvers.
    Resolvers,
    /// #116: Monotonic counter used to round-robin disputes across resolvers.
    ResolverCounter,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformStats {
    pub total_agents: u64,
    pub total_jobs: u64,
    pub total_volume: i128,
    pub total_fees_collected: i128,
    pub total_disputes: u64,
    pub avg_job_completion_time: u64,
    /// #128: Number of jobs currently in `JobStatus::Open`.
    pub open_jobs: u32,
    /// #128: Number of agents whose `is_active` flag is `true`.
    pub active_agents: u32,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    AgentNotFound = 4,
    AgentAlreadyExists = 5,
    AgentNotActive = 6,
    JobNotFound = 7,
    JobNotOpen = 8,
    JobNotInProgress = 9,
    JobNotCompleted = 10,
    JobAlreadyAssigned = 11,
    InsufficientBudget = 12,
    BidNotFound = 13,
    BidNotPending = 14,
    BidAlreadyExists = 15,
    InsufficientBidAmount = 16,
    BidTooLow = 17,
    AgentReputationTooLow = 18,
    InsufficientSkills = 19,
    SkillNotVerified = 20,
    /// Returned by `place_bid` when `config.require_skill_verification = true`
    /// and the bidding agent has not had the required skill verified by an
    /// authorized verifier.  Distinct from `SkillNotVerified` (which is kept
    /// for backward-compatibility) so callers can tell the difference between
    /// "skill not in agent's list" vs. "skill present but unverified while
    /// enforcement is enabled".
    SkillVerificationRequired = 52,
    EscrowNotFound = 21,
    EscrowNotFunded = 22,
    EscrowAlreadyReleased = 23,
    EscrowNotDisputed = 24,
    InsufficientEscrowBalance = 25,
    InvalidDeadline = 26,
    InvalidBudget = 27,
    InvalidRating = 28,
    ReviewNotFound = 29,
    AlreadyReviewed = 30,
    DisputeNotFound = 31,
    DisputeNotOpen = 32,
    DisputeWindowExpired = 33,
    UnauthorizedResolver = 34,
    InvalidToken = 35,
    InsufficientBalance = 36,
    TransferFailed = 37,
    EscrowExpired = 38,
    JobExpired = 39,
    InvalidSkill = 40,
    SkillAlreadyVerified = 41,
    VerifierNotAuthorized = 42,
    VerificationExpired = 43,
    PlatformNotInitialized = 44,
    InvalidConfig = 45,
    JobCancelled = 46,
    AlreadyDisputed = 47,
    DisputeResolved = 48,
    EscrowAlreadyRefunded = 49,
    InvalidDisputeEvidence = 50,
    ContractPaused = 51,
    CannotBidOnOwnJob = 52,
    /// Returned by `post_job` / `place_bid` when the caller's address is on the
    /// admin-managed sanctions/denylist.
    AddressBlocked = 53,
    /// Returned by `leave_review` when the comment exceeds
    /// `MAX_REVIEW_COMMENT_LEN` bytes.
    InputTooLong = 54,
    /// Returned by `place_bid` when the bidding agent has been deactivated.
    AgentInactive = 55,
    /// #111: Returned by `resolve_dispute` when the requested resolution
    /// cannot be honored for the dispute's current state.
    InvalidResolution = 56,
    /// #111: Returned by `resolve_dispute` when a split percentage is greater
    /// than 100.
    InvalidSplit = 57,
}

/// #172: Minimum TTL (in ledgers) we keep on persistent records.  At ~5s per
/// ledger this is roughly 30 days.  Every read/write of a persistent record
/// bumps its TTL back up to at least this value.
pub const MIN_PERSISTENT_TTL: u32 = 518_400;
/// #172: TTL (in ledgers) we extend persistent records to on touch (~90 days).
pub const EXTEND_PERSISTENT_TTL: u32 = 1_555_200;
/// #172: Minimum TTL (in ledgers) for the contract instance (~30 days).
pub const MIN_INSTANCE_TTL: u32 = 518_400;
/// #172: TTL (in ledgers) we extend the contract instance to on touch (~90 days).
pub const EXTEND_INSTANCE_TTL: u32 = 1_555_200;
/// #170: Default fallback split (percentage of payout to the agent) applied
/// when a dispute times out without an admin resolution.
pub const DISPUTE_FALLBACK_AGENT_PCT: u32 = 50;
/// #124: Maximum length, in bytes, of the free-form `comment` accepted by
/// `leave_review`.  Comments are stored on chain forever, so this cap bounds
/// the storage cost per review.  Longer comments are rejected with
/// `Error::InputTooLong`.
pub const MAX_REVIEW_COMMENT_LEN: u32 = 500;
/// #114: Maximum length, in bytes, of the free-form `reason` accepted by
/// `raise_dispute`.  Longer reasons are rejected with
/// `Error::DisputeTextTooLong`.
pub const MAX_DISPUTE_REASON_LEN: u32 = 500;
/// #114: Maximum length, in bytes, of the `evidence_uri` accepted by
/// `raise_dispute`.  Longer URIs are rejected with
/// `Error::DisputeTextTooLong`.
pub const MAX_DISPUTE_EVIDENCE_URI_LEN: u32 = 256;
/// #115: Default number of days after a dispute is raised before the fallback
/// resolution may be triggered.
pub const DEFAULT_RESOLUTION_DEADLINE_DAYS: u64 = 14;
/// #115: Default percentage of the post-fee payout sent to the agent when the
/// resolver times out (remainder refunded to the client).
pub const DEFAULT_RESOLVER_TIMEOUT_AGENT_PCT: u32 = 50;

#[contract]
pub struct AiAgentMarketplace;

#[contractimpl]
impl AiAgentMarketplace {
    /// Initialize the marketplace contract.
    ///
    /// #102: Requires authorization from `admin`.  Deployments MUST deploy and
    /// initialize atomically (e.g. via the deploy script or a `__constructor`
    /// where supported) so that a third party cannot front-run the
    /// initialization and choose the admin, fee recipient, resolver and token.
    ///
    /// #103: All `Config` fields are validated via `validate_config` before
    /// anything is persisted.  See that helper for the documented rules.
    pub fn initialize(
        env: Env,
        admin: Address,
        usdc_token: Address,
        platform_fee_bps: u32,
        min_job_budget: i128,
        max_job_deadline_days: u64,
        dispute_window_days: u64,
        platform_fee_recipient: Address,
        dispute_resolver: Address,
        min_agent_reputation_to_bid: u32,
        platform_fee_bps_dispute: u32,
        escrow_expiry_days: u64,
        skill_verifiers: Vec<Address>,
        require_skill_verification: bool,
    ) -> Result<(), Error> {
        // #102: the admin must authorize initialization.  This prevents a
        // front-runner from initializing the contract with their own admin.
        admin.require_auth();

        if Self::has_config(&env) {
            return Err(Error::AlreadyInitialized);
        }

        // #132: Verify the configured token is a 7-decimal Stellar Asset
        // Contract.  All amounts in this contract assume USDC's 7 decimals.
        let token_client = TokenClient::new(&env, &usdc_token);
        let decimals = token_client.decimals();
        if decimals != USDC_DECIMALS {
            return Err(Error::InvalidToken);
        }

        // Canary defaults: cap a single job at 10_000 USDC and total locked
        // escrow at 100_000 USDC (7 decimals).  Admins raise these in stages
        // via `set_max_job_budget` / `set_max_total_escrow`.
        let max_job_budget: i128 = 10_000 * 10_000_000;
        let max_total_escrow: i128 = 100_000 * 10_000_000;

        let config = Config {
            admin: admin.clone(),
            usdc_token: usdc_token.clone(),
            platform_fee_bps,
            min_job_budget,
            max_job_deadline_days,
            dispute_window_days,
            platform_fee_recipient: platform_fee_recipient.clone(),
            dispute_resolver: dispute_resolver.clone(),
            min_agent_reputation_to_bid,
            platform_fee_bps_dispute,
            escrow_expiry_days,
            resolution_deadline_days: DEFAULT_RESOLUTION_DEADLINE_DAYS,
            resolver_timeout_agent_pct: DEFAULT_RESOLVER_TIMEOUT_AGENT_PCT,
            skill_verifiers,
            require_skill_verification,
            // #105: bumped from 1 to 2 when the unused `min_agent_reputation`
            // field was removed from the schema.
            version: 2,
            max_job_budget,
            max_total_escrow,
            total_locked_escrow: 0,
            max_bids_per_job: DEFAULT_MAX_BIDS_PER_JOB,
            starting_reputation: 0,
        };

        // #103 / #104: validate every field before persisting.
        Self::validate_config(&config)?;

        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().set(&DataKey::AgentCounter, &0u64);
        env.storage().instance().set(&DataKey::JobCounter, &0u64);
        env.storage().instance().set(&DataKey::BidCounter, &0u64);
        env.storage().instance().set(&DataKey::ReviewCounter, &0u64);
        env.storage().instance().set(&DataKey::DisputeCounter, &0u64);
        env.storage().instance().set(&DataKey::EscrowCounter, &0u64);
        env.storage().instance().set(&DataKey::VerificationCounter, &0u64);

        // #116: seed the resolver set with the configured resolver.
        let mut resolvers: Vec<Address> = Vec::new(&env);
        resolvers.push_back(dispute_resolver.clone());
        env.storage().instance().set(&DataKey::Resolvers, &resolvers);
        env.storage().instance().set(&DataKey::ResolverCounter, &0u64);

        let stats = PlatformStats {
            total_agents: 0,
            total_jobs: 0,
            total_volume: 0,
            total_fees_collected: 0,
            total_disputes: 0,
            avg_job_completion_time: 0,
            open_jobs: 0,
            active_agents: 0,
        };
        env.storage().instance().set(&DataKey::PlatformStats, &stats);
        env.storage().instance().set(&DataKey::IsPaused, &false);

        Self::extend_instance_ttl(&env);

        // Emit initialization event
        env.events().publish(
            (symbol_short!("init"),),
            (admin, usdc_token, platform_fee_bps),
        );

        Ok(())
    }


    /// Admin function to pause or unpause contract operations
    pub fn set_paused(env: Env, admin: Address, paused: bool) -> Result<(), Error> {
        admin.require_auth();
        Self::extend_instance_ttl(&env);
        let config = Self::get_config(&env)?;
        if admin != config.admin {
            return Err(Error::Unauthorized);
        }

        env.storage().instance().set(&DataKey::IsPaused, &paused);
        env.events().publish(
            (symbol_short!("pause"),),
            (admin, paused),
        );
        Ok(())
    }

    /// Check if contract is paused
    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::IsPaused).unwrap_or(false)
    }

    /// Admin: pause the contract (alias for `set_paused(true)`).
    ///
    /// Used by the incident response runbook to halt state-changing operations
    /// during an incident.
    pub fn pause(env: Env, admin: Address) -> Result<(), Error> {
        Self::set_paused(env, admin, true)
    }

    /// Admin: unpause the contract (alias for `set_paused(false)`).
    pub fn unpause(env: Env, admin: Address) -> Result<(), Error> {
        Self::set_paused(env, admin, false)
    }

    /// Admin: add an address to the sanctions/denylist.
    ///
    /// Denied addresses cannot post jobs or place bids.  The list is stored on
    /// chain and can be managed without redeploying the contract.
    pub fn add_to_denylist(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_config(&env)?;
        if admin != config.admin {
            return Err(Error::Unauthorized);
        }

        let mut denylist: Map<Address, bool> = env.storage().instance()
            .get(&DataKey::Denylist)
            .unwrap_or(Map::new(&env));
        denylist.set(address.clone(), true);
        env.storage().instance().set(&DataKey::Denylist, &denylist);

        env.events().publish(
            (symbol_short!("deny_add"),),
            (admin, address),
        );
        Ok(())
    }

    /// Admin: remove an address from the sanctions/denylist.
    pub fn remove_from_denylist(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_config(&env)?;
        if admin != config.admin {
            return Err(Error::Unauthorized);
        }

        let mut denylist: Map<Address, bool> = env.storage().instance()
            .get(&DataKey::Denylist)
            .unwrap_or(Map::new(&env));
        denylist.remove(address.clone());
        env.storage().instance().set(&DataKey::Denylist, &denylist);

        env.events().publish(
            (symbol_short!("deny_rem"),),
            (admin, address),
        );
        Ok(())
    }

    /// Get the current sanctions/denylist.
    pub fn get_denylist(env: Env) -> Map<Address, bool> {
        env.storage().instance()
            .get(&DataKey::Denylist)
            .unwrap_or(Map::new(&env))
    }

    /// Check whether an address is on the denylist.
    pub fn is_denylisted(env: Env, address: Address) -> bool {
        Self::is_denylisted(&env, &address)
    }

    /// Register a new AI agent
    pub fn register_agent(
        env: Env,
        owner: Address,
        name: String,
        description: String,
        skills: Vec<String>,
        price_per_task: i128,
        metadata_uri: String,
    ) -> Result<u64, Error> {
        owner.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        if price_per_task < config.min_job_budget {
            return Err(Error::InvalidBudget);
        }

        // Check if owner already has an agent
        let existing_agent_id: Option<u64> = env.storage().instance().get(&DataKey::AgentByOwner(owner.clone()));
        if existing_agent_id.is_some() {
            return Err(Error::AgentAlreadyExists);
        }

        let agent_id = Self::next_agent_id(&env)?;
        let now = env.ledger().timestamp();

        let agent = Agent {
            id: agent_id,
            owner: owner.clone(),
            name,
            description,
            skills: skills.clone(),
            price_per_task,
            // #119: seed from config so cold-start agents are not blocked.
            reputation: config.starting_reputation,
            total_jobs_completed: 0,
            total_earnings: 0,
            is_active: true,
            created_at: now,
            skills_verified: Map::new(&env),
            metadata_uri,
            // #120: exact reputation inputs.
            rating_sum: 0,
            rating_count: 0,
        };

        // Store agent
        env.storage().instance().set(&DataKey::Agent(agent_id), &agent);
        env.storage().instance().set(&DataKey::AgentByOwner(owner.clone()), &agent_id);
        Self::extend_instance_ttl(&env);

        // Index skills
        for skill in skills.iter() {
            let mut skill_agents: Vec<u64> = env.storage().instance()
                .get(&DataKey::SkillAgents(skill.clone()))
                .unwrap_or(Vec::new(&env));
            skill_agents.push_back(agent_id);
            env.storage().instance().set(&DataKey::SkillAgents(skill.clone()), &skill_agents);
        }

        // Update stats
        let mut stats = Self::get_platform_stats(&env)?;
        stats.total_agents += 1;
        stats.active_agents += 1;
        env.storage().instance().set(&DataKey::PlatformStats, &stats);

        env.events().publish(
            (symbol_short!("agent_reg"), agent_id, owner.clone()),
            (agent_id, owner, agent.price_per_task),
        );

        Ok(agent_id)
    }

    /// Update agent details
    pub fn update_agent(
        env: Env,
        owner: Address,
        agent_id: u64,
        name: Option<String>,
        description: Option<String>,
        price_per_task: Option<i128>,
        is_active: Option<bool>,
        metadata_uri: Option<String>,
    ) -> Result<(), Error> {
        owner.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::extend_instance_ttl(&env);

        let mut agent = Self::get_agent(&env, agent_id)?;
        if agent.owner != owner {
            return Err(Error::Unauthorized);
        }

        if let Some(name) = name {
            agent.name = name;
        }
        if let Some(description) = description {
            agent.description = description;
        }
        if let Some(price) = price_per_task {
            let config = Self::get_config(&env)?;
            if price < config.min_job_budget {
                return Err(Error::InvalidBudget);
            }
            agent.price_per_task = price;
        }
        if let Some(active) = is_active {
            if agent.is_active && !active {
                let mut stats = Self::get_platform_stats(&env)?;
                stats.active_agents = stats.active_agents.saturating_sub(1);
                env.storage().instance().set(&DataKey::PlatformStats, &stats);
            } else if !agent.is_active && active {
                let mut stats = Self::get_platform_stats(&env)?;
                stats.active_agents += 1;
                env.storage().instance().set(&DataKey::PlatformStats, &stats);
            }
            agent.is_active = active;
        }
        if let Some(uri) = metadata_uri {
            agent.metadata_uri = uri;
        }

        env.storage().instance().set(&DataKey::Agent(agent_id), &agent);

        env.events().publish(
            (symbol_short!("agent_upd"), agent_id, owner.clone()),
            (agent_id, owner),
        );

        Ok(())
    }

    /// #125: Deactivate an agent (owner only).
    ///
    /// Flips `agent.is_active` to `false`, which prevents the agent from
    /// bidding on new jobs and hides it from skill indexes.  Jobs already in
    /// progress are unaffected.  Emits an `agent_deact` event.
    pub fn deactivate_agent(
        env: Env,
        owner: Address,
        agent_id: u64,
    ) -> Result<(), Error> {
        owner.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::extend_instance_ttl(&env);

        let mut agent = Self::get_agent(&env, agent_id)?;
        if agent.owner != owner {
            return Err(Error::Unauthorized);
        }

        if agent.is_active {
            agent.is_active = false;
            env.storage().instance().set(&DataKey::Agent(agent_id), &agent);

            let mut stats = Self::get_platform_stats(&env)?;
            stats.active_agents = stats.active_agents.saturating_sub(1);
            env.storage().instance().set(&DataKey::PlatformStats, &stats);
        }

        env.events().publish(
            (symbol_short!("agent_deact"), agent_id, owner.clone()),
            (agent_id, owner, agent.is_active),
        );

        Ok(())
    }

    /// #125: Reactivate an agent (owner only).
    ///
    /// Flips `agent.is_active` back to `true`, allowing the agent to bid again
    /// and reappear in skill indexes.  Emits an `agent_react` event.
    pub fn reactivate_agent(
        env: Env,
        owner: Address,
        agent_id: u64,
    ) -> Result<(), Error> {
        owner.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::extend_instance_ttl(&env);

        let mut agent = Self::get_agent(&env, agent_id)?;
        if agent.owner != owner {
            return Err(Error::Unauthorized);
        }

        if !agent.is_active {
            agent.is_active = true;
            env.storage().instance().set(&DataKey::Agent(agent_id), &agent);

            let mut stats = Self::get_platform_stats(&env)?;
            stats.active_agents += 1;
            env.storage().instance().set(&DataKey::PlatformStats, &stats);
        }

        env.events().publish(
            (symbol_short!("agent_react"), agent_id, owner.clone()),
            (agent_id, owner, agent.is_active),
        );

        Ok(())
    }

    /// Verify an agent's skill (called by authorized verifier)
    pub fn verify_skill(
        env: Env,
        verifier: Address,
        agent_id: u64,
        skill: String,
        evidence_uri: String,
        expires_at: u64,
    ) -> Result<(), Error> {
        verifier.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        if !config.skill_verifiers.contains(&verifier) {
            return Err(Error::VerifierNotAuthorized);
        }

        let mut agent = Self::get_agent(&env, agent_id)?;
        if !agent.skills.contains(&skill) {
            return Err(Error::InvalidSkill);
        }

        let now = env.ledger().timestamp();

        if agent.skills_verified.get(skill.clone()).unwrap_or(false) {
            // Allow re-verification if the existing verification has expired.
            let agent_verifications: Vec<u64> = env.storage().instance()
                .get(&DataKey::SkillVerificationsByAgent(agent.owner.clone()))
                .unwrap_or(Vec::new(&env));
            let mut is_expired = false;
            for ver_id in agent_verifications.iter() {
                let ver: SkillVerification = env.storage().instance()
                    .get(&DataKey::SkillVerification(ver_id))
                    .ok_or(Error::VerificationExpired)?;
                if ver.skill == skill && now > ver.expires_at {
                    is_expired = true;
                    break;
                }
            }
            if !is_expired {
                return Err(Error::SkillAlreadyVerified);
            }
        }

        if expires_at <= now {
            return Err(Error::InvalidDeadline);
        }

        let verification = SkillVerification {
            skill: skill.clone(),
            verified_by: verifier.clone(),
            verified_at: now,
            evidence_uri,
            expires_at,
        };

        let verification_id = Self::next_verification_id(&env)?;
        env.storage().instance().set(&DataKey::SkillVerification(verification_id), &verification);

        // Update agent's verified skills
        agent.skills_verified.set(skill.clone(), true);
        env.storage().instance().set(&DataKey::Agent(agent_id), &agent);

        // Index verification
        let mut agent_verifications: Vec<u64> = env.storage().instance()
            .get(&DataKey::SkillVerificationsByAgent(agent.owner.clone()))
            .unwrap_or(Vec::new(&env));
        agent_verifications.push_back(verification_id);
        env.storage().instance().set(&DataKey::SkillVerificationsByAgent(agent.owner.clone()), &agent_verifications);

        env.events().publish(
            (symbol_short!("skill_ver"), agent_id, verifier.clone()),
            (agent_id, skill, verifier, expires_at),
        );

        Ok(())
    }

    /// Post a new job
    pub fn post_job(
        env: Env,
        client: Address,
        title: String,
        description: String,
        required_skills: Vec<String>,
        budget: i128,
        deadline_days: u64,
    ) -> Result<u64, Error> {
        client.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::ensure_not_denylisted(&env, &client)?;

        let config = Self::get_config(&env)?;
        
        if budget < config.min_job_budget {
            return Err(Error::InsufficientBudget);
        }

        // Canary exposure caps: reject jobs above the per-job cap and refuse
        // to post a job that would push total locked escrow over the cap.
        if budget > config.max_job_budget {
            return Err(Error::InvalidBudget);
        }
        if config.total_locked_escrow + budget > config.max_total_escrow {
            return Err(Error::InsufficientEscrowBalance);
        }

        if deadline_days > config.max_job_deadline_days || deadline_days == 0 {
            return Err(Error::InvalidDeadline);
        }

        let job_id = Self::next_job_id(&env)?;
        let now = env.ledger().timestamp();
        let deadline = now + (deadline_days * 86400); // Convert days to seconds

        let job = Job {
            id: job_id,
            client: client.clone(),
            title,
            description,
            required_skills: required_skills.clone(),
            budget,
            deadline,
            status: JobStatus::Open,
            assigned_agent: None,
            escrow_amount: 0,
            created_at: now,
            accepted_at: None,
            completed_at: None,
            deliverable_hash: None,
            dispute_raised: false,
        };

        env.storage().instance().set(&DataKey::Job(job_id), &job);
        Self::extend_instance_ttl(&env);

        // Index job by client
        let mut client_jobs: Vec<u64> = env.storage().instance()
            .get(&DataKey::JobsByClient(client.clone()))
            .unwrap_or(Vec::new(&env));
        client_jobs.push_back(job_id);
        env.storage().instance().set(&DataKey::JobsByClient(client.clone()), &client_jobs);

        // Update stats
        let mut stats = Self::get_platform_stats(&env)?;
        stats.total_jobs += 1;
        stats.open_jobs += 1;
        env.storage().instance().set(&DataKey::PlatformStats, &stats);

        // #121: track client reputation.
        let mut client_stats = Self::get_client_stats_internal(&env, &client);
        client_stats.jobs_posted = client_stats.jobs_posted.saturating_add(1);
        env.storage().instance().set(&DataKey::ClientStats(client.clone()), &client_stats);

        env.events().publish(
            (symbol_short!("job_post"), job_id, client.clone()),
            (job_id, client, budget, deadline),
        );

        Ok(job_id)
    }

    /// Cancel an open job (client only)
    pub fn cancel_job(env: Env, client: Address, job_id: u64) -> Result<(), Error> {
        client.require_auth();
        // #118: cancellation is a settlement-type call — allowed while paused.
        Self::extend_instance_ttl(&env);

        let mut job = Self::get_job(&env, job_id)?;
        if job.client != client {
            return Err(Error::Unauthorized);
        }
        if job.status != JobStatus::Open {
            return Err(Error::JobNotOpen);
        }

        // Reject all pending bids so bidders know the job is no longer available.
        let existing_bids: Vec<u64> = env.storage().instance()
            .get(&DataKey::BidsByJob(job_id))
            .unwrap_or(Vec::new(&env));
        for bid_id in existing_bids.iter() {
            if let Ok(mut bid) = Self::get_bid(&env, *bid_id) {
                if bid.status == BidStatus::Pending {
                    bid.status = BidStatus::Rejected;
                    env.storage().instance().set(&DataKey::Bid(*bid_id), &bid);
                }
            }
        }

        job.status = JobStatus::Cancelled;
        env.storage().instance().set(&DataKey::Job(job_id), &job);

        let mut stats = Self::get_platform_stats(&env)?;
        stats.open_jobs = stats.open_jobs.saturating_sub(1);
        env.storage().instance().set(&DataKey::PlatformStats, &stats);

        env.events().publish(
            (symbol_short!("job_cancel"), job_id, client.clone()),
            (job_id, client),
        );

        Ok(())
    }

    /// Place a bid on a job
    pub fn place_bid(
        env: Env,
        agent_owner: Address,
        job_id: u64,
        amount: i128,
        proposed_timeline_days: u64,
        proposal: String,
    ) -> Result<u64, Error> {
        agent_owner.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::ensure_not_denylisted(&env, &agent_owner)?;

        let config = Self::get_config(&env)?;
        let job = Self::get_job(&env, job_id)?;

        if job.status != JobStatus::Open {
            return Err(Error::JobNotOpen);
        }

        if env.ledger().timestamp() > job.deadline {
            return Err(Error::JobExpired);
        }

        // An agent owner cannot bid on their own job (if the client is also an agent).
        if job.client == agent_owner {
            return Err(Error::CannotBidOnOwnJob);
        }

        // Check if agent exists and is active
        let agent_id: u64 = env.storage().instance()
            .get(&DataKey::AgentByOwner(agent_owner.clone()))
            .ok_or(Error::AgentNotFound)?;
        let agent = Self::get_agent(&env, agent_id)?;

        if !agent.is_active {
            return Err(Error::AgentInactive);
        }

        // #119: cold-start exemption.  Agents that have never completed a job
        // are exempt from the minimum-reputation gate so they can earn their
        // first review.  Once they complete a job, the gate applies normally.
        if agent.total_jobs_completed > 0
            && agent.reputation < config.min_agent_reputation_to_bid
        {
            return Err(Error::AgentReputationTooLow);
        }

        // Check if agent has required skills
        for required_skill in job.required_skills.iter() {
            if !agent.skills.contains(&required_skill) {
                return Err(Error::InsufficientSkills);
            }
            // Enforce skill verification only when the platform operator has
            // enabled hard verification mode.  When disabled, unverified agents
            // may bid; the `skills_verified` map is still shown on the agent
            // profile as a trust signal but does not gate participation.
            if config.require_skill_verification
                && !agent.skills_verified.get(required_skill.clone()).unwrap_or(false)
            {
                return Err(Error::SkillVerificationRequired);
            }
        }

        // Check if agent already bid
        let existing_bids: Vec<u64> = env.storage().instance()
            .get(&DataKey::BidsByJob(job_id))
            .unwrap_or(Vec::new(&env));
        for bid_id in existing_bids.iter() {
            let bid = Self::get_bid(&env, bid_id)?;
            if bid.agent == agent_owner {
                return Err(Error::BidAlreadyExists);
            }
        }

        // #129: Cap the number of bids per job so that `accept_bid`'s loop
        // over the job's bids stays bounded.
        if existing_bids.len() >= config.max_bids_per_job {
            return Err(Error::TooManyBids);
        }

        // Check bid amount
        if amount < job.budget / 2 {
            // Minimum 50% of budget
            return Err(Error::BidTooLow);
        }

        if amount > job.budget {
            return Err(Error::InsufficientBidAmount);
        }

        let bid_id = Self::next_bid_id(&env)?;
        let now = env.ledger().timestamp();

        let bid = Bid {
            agent: agent_owner.clone(),
            agent_id,
            amount,
            proposed_timeline: now + (proposed_timeline_days * 86400),
            proposal,
            submitted_at: now,
            status: BidStatus::Pending,
        };

        env.storage().instance().set(&DataKey::Bid(bid_id), &bid);

        // Index bid
        let mut job_bids = existing_bids;
        job_bids.push_back(bid_id);
        env.storage().instance().set(&DataKey::BidsByJob(job_id), &job_bids);

        let mut agent_bids: Vec<u64> = env.storage().instance()
            .get(&DataKey::BidsByAgent(agent_owner.clone()))
            .unwrap_or(Vec::new(&env));
        agent_bids.push_back(bid_id);
        env.storage().instance().set(&DataKey::BidsByAgent(agent_owner.clone()), &agent_bids);

        env.events().publish(
            (symbol_short!("bid_placed"), job_id, agent_owner.clone()),
            (bid_id, job_id, agent_owner, amount),
        );

        Ok(bid_id)
    }

    /// Withdraw a pending bid (agent only)
    pub fn withdraw_bid(
        env: Env,
        agent_owner: Address,
        job_id: u64,
        bid_id: u64,
    ) -> Result<(), Error> {
        agent_owner.require_auth();
        // #118: settlement-type call — allowed while paused.
        Self::extend_instance_ttl(&env);

        let job = Self::get_job(&env, job_id)?;
        if job.status != JobStatus::Open {
            return Err(Error::JobNotOpen);
        }

        let mut bid = Self::get_bid(&env, bid_id)?;
        if bid.agent != agent_owner {
            return Err(Error::Unauthorized);
        }
        if bid.status != BidStatus::Pending {
            return Err(Error::BidNotPending);
        }

        // Verify bid belongs to this job
        let job_bids: Vec<u64> = env.storage().instance()
            .get(&DataKey::BidsByJob(job_id))
            .unwrap_or(Vec::new(&env));
        if !job_bids.contains(&bid_id) {
            return Err(Error::BidNotFound);
        }

        bid.status = BidStatus::Withdrawn;
        env.storage().instance().set(&DataKey::Bid(bid_id), &bid);

        env.events().publish(
            (symbol_short!("bid_withdraw"), job_id, agent_owner.clone()),
            (bid_id, job_id, agent_owner),
        );

        Ok(())
    }

    /// Accept a bid and create escrow
    pub fn accept_bid(
        env: Env,
        client: Address,
        job_id: u64,
        bid_id: u64,
    ) -> Result<(), Error> {
        client.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        let mut job = Self::get_job(&env, job_id)?;

        if job.client != client {
            return Err(Error::Unauthorized);
        }

        if job.status != JobStatus::Open {
            return Err(Error::JobNotOpen);
        }

        let mut bid = Self::get_bid(&env, bid_id)?;

        if bid.status != BidStatus::Pending {
            return Err(Error::BidNotPending);
        }

        // #169: refuse to accept a bid from an agent that cannot receive USDC,
        // otherwise the payout in `approve_deliverable` would be trapped.
        if !Self::is_payable(&env, &config.usdc_token, &bid.agent) {
            return Err(Error::AgentNotPayable);
        }

        // Verify bid belongs to this job
        let job_bids: Vec<u64> = env.storage().instance()
            .get(&DataKey::BidsByJob(job_id))
            .unwrap_or(Vec::new(&env));
        if !job_bids.contains(&bid_id) {
            return Err(Error::BidNotFound);
        }

        // Calculate platform fee
        let platform_fee = (bid.amount * config.platform_fee_bps as i128) / 10000;
        let escrow_amount = bid.amount + platform_fee;

        // Enforce the total locked escrow cap before moving any funds.
        if config.total_locked_escrow + escrow_amount > config.max_total_escrow {
            return Err(Error::InsufficientEscrowBalance);
        }

        // ===== CHECKS-EFFECTS-INTERACTIONS =====
        // All validation above; all state writes below; token transfers last.
        // A panicking/failing token contract therefore leaves no partial state
        // (the whole invocation reverts, and no state was written before the
        // external calls).

        // ----- Effects: write all state before any external call -----

        // Create escrow
        let escrow_id = Self::next_escrow_id(&env)?;
        let now = env.ledger().timestamp();

        let escrow = Escrow {
            job_id,
            client: client.clone(),
            agent: bid.agent.clone(),
            amount: bid.amount,
            token: config.usdc_token.clone(),
            status: EscrowStatus::Funded,
            created_at: now,
            released_at: None,
            dispute_raised_at: None,
            resolver: None,
            dispute_deadline: None,
        };

        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);
        // EscrowByJob maps job_id -> escrow_id, created atomically in accept_bid.
        // This guarantees the lookup succeeds for any job that has ever had a funded escrow.
        env.storage().instance().set(&DataKey::EscrowByJob(job_id), &escrow_id);
        Self::extend_instance_ttl(&env);

        // Update job
        job.status = JobStatus::InProgress;
        job.assigned_agent = Some(bid.agent.clone());
        job.escrow_amount = escrow_amount;
        // #112: record when work started so the dispute window for jobs that
        // never complete is measured from acceptance, not from posting.
        job.accepted_at = Some(now);
        env.storage().instance().set(&DataKey::Job(job_id), &job);

        // Update bid status
        bid.status = BidStatus::Accepted;
        env.storage().instance().set(&DataKey::Bid(bid_id), &bid);

        // Reject other bids.  #129: bounded by config.max_bids_per_job.
        for other_bid_id in job_bids.iter() {
            if *other_bid_id != bid_id {
                let mut other_bid = Self::get_bid(&env, other_bid_id)?;
                other_bid.status = BidStatus::Rejected;
                env.storage().instance().set(&DataKey::Bid(other_bid_id), &other_bid);
            }
        }

        // Index job by agent
        let mut agent_jobs: Vec<u64> = env.storage().instance()
            .get(&DataKey::JobsByAgent(bid.agent.clone()))
            .unwrap_or(Vec::new(&env));
        agent_jobs.push_back(job_id);
        env.storage().instance().set(&DataKey::JobsByAgent(bid.agent.clone()), &agent_jobs);

        // Update stats
        let mut stats = Self::get_platform_stats(&env)?;
        stats.total_volume += bid.amount;
        stats.total_fees_collected += platform_fee;
        stats.open_jobs = stats.open_jobs.saturating_sub(1);
        env.storage().instance().set(&DataKey::PlatformStats, &stats);

        // Track the newly locked escrow against the canary cap.
        let mut updated_config = config.clone();
        updated_config.total_locked_escrow += escrow_amount;
        env.storage().instance().set(&DataKey::Config, &updated_config);

        // ----- Interactions: two direct transfers, fee moved only once -----
        // #130: transfer the fee directly from the client to the recipient and
        // the bid amount directly from the client into escrow, instead of
        // pulling bid+fee in and pushing the fee back out.
        let usdc_client = TokenClient::new(&env, &config.usdc_token);
        if platform_fee > 0 {
            usdc_client.transfer(&client, &config.platform_fee_recipient, &platform_fee);
        }
        usdc_client.transfer(&client, &env.current_contract_address(), &bid.amount);

        env.events().publish(
            (symbol_short!("bid_accept"), job_id, bid.agent.clone()),
            (job_id, bid_id, bid.agent, bid.amount, platform_fee),
        );

        Ok(())
    }

    /// Submit work deliverable
    pub fn submit_deliverable(
        env: Env,
        agent: Address,
        job_id: u64,
        deliverable_hash: BytesN<32>,
    ) -> Result<(), Error> {
        agent.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::extend_instance_ttl(&env);

        let mut job = Self::get_job(&env, job_id)?;

        if job.assigned_agent != Some(agent.clone()) {
            return Err(Error::Unauthorized);
        }

        if job.status != JobStatus::InProgress {
            return Err(Error::JobNotInProgress);
        }

        job.deliverable_hash = Some(deliverable_hash);
        job.status = JobStatus::Completed;
        job.completed_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::Job(job_id), &job);

        env.events().publish(
            (symbol_short!("deliver"), job_id, agent.clone()),
            (job_id, agent, deliverable_hash),
        );

        Ok(())
    }

    /// Approve deliverable and release escrow to agent
    pub fn approve_deliverable(
        env: Env,
        client: Address,
        job_id: u64,
    ) -> Result<(), Error> {
        client.require_auth();
        // #118: settlement-type call — allowed while paused.
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        let mut job = Self::get_job(&env, job_id)?;

        if job.client != client {
            return Err(Error::Unauthorized);
        }

        if job.status != JobStatus::Completed {
            return Err(Error::JobNotCompleted);
        }

        if job.deliverable_hash.is_none() {
            return Err(Error::InvalidDisputeEvidence);
        }

        // Get escrow
        let escrow_id: u64 = env.storage().instance()
            .get(&DataKey::EscrowByJob(job_id))
            .ok_or(Error::EscrowNotFound)?;
        let mut escrow = Self::get_escrow(&env, escrow_id)?;

        if escrow.status != EscrowStatus::Funded {
            return Err(Error::EscrowNotFunded);
        }

        // ===== CHECKS-EFFECTS-INTERACTIONS =====
        // All validation above; all state writes below; token transfers last.

        // ----- Effects: write all state before any external call -----

        // Update escrow
        escrow.status = EscrowStatus::Released;
        escrow.released_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);
        Self::extend_instance_ttl(&env);

        // #117: shared finalization — credits agent stats, sets completed_at,
        // updates platform stats and releases the locked-escrow accounting.
        Self::finalize_job_payout(&env, job_id, &mut job, &escrow.agent, escrow.amount)?;

        // ----- Interactions: release payment to agent -----
        // #169: if the agent cannot receive USDC (missing trustline), record a
        // claimable balance instead of trapping the funds in escrow.  The agent
        // withdraws it later via `claim_funds`.
        let usdc_client = TokenClient::new(&env, &config.usdc_token);
        if Self::is_payable(&env, &config.usdc_token, &escrow.agent) {
            usdc_client.transfer(
                &env.current_contract_address(),
                &escrow.agent,
                &escrow.amount,
            );
        } else {
            Self::record_claimable(&env, job_id, &escrow.agent, escrow.amount, &config.usdc_token);
        }

        env.events().publish(
            (symbol_short!("approved"), job_id, escrow.agent.clone()),
            (job_id, escrow.agent, escrow.amount),
        );

        Ok(())
    }

    /// Raise a dispute
    pub fn raise_dispute(
        env: Env,
        raiser: Address,
        job_id: u64,
        reason: String,
        evidence_uri: String,
    ) -> Result<u64, Error> {
        raiser.require_auth();
        // #118: dispute entrypoint — allowed while paused.
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        let mut job = Self::get_job(&env, job_id)?;

        if job.client != raiser && job.assigned_agent != Some(raiser.clone()) {
            return Err(Error::Unauthorized);
        }

        // #113: return the correct error per state.
        if job.status == JobStatus::Cancelled {
            return Err(Error::JobCancelled);
        }
        if job.status == JobStatus::Disputed {
            return Err(Error::AlreadyDisputed);
        }

        if job.dispute_raised {
            return Err(Error::AlreadyDisputed);
        }

        // Check dispute window.  #112: measured from completion when the job
        // completed, otherwise from bid acceptance (when work started), and
        // only falling back to creation for jobs that were never accepted.
        let window_start = job.completed_at
            .unwrap_or_else(|| job.accepted_at.unwrap_or(job.created_at));
        let dispute_deadline = window_start + (config.dispute_window_days * 86400);
        if env.ledger().timestamp() > dispute_deadline {
            return Err(Error::DisputeWindowExpired);
        }

        // Get escrow
        let escrow_id: u64 = env.storage().instance()
            .get(&DataKey::EscrowByJob(job_id))
            .ok_or(Error::EscrowNotFound)?;
        let mut escrow = Self::get_escrow(&env, escrow_id)?;

        if escrow.status != EscrowStatus::Funded {
            return Err(Error::EscrowNotFunded);
        }

        // Create dispute
        let dispute_id = Self::next_dispute_id(&env)?;
        let now = env.ledger().timestamp();
        // #115: after this timestamp anyone may trigger the fallback.
        let dispute_deadline = now + (config.resolution_deadline_days * 86400);

        // #116: assign a resolver round-robin from the admin-managed set.
        let assigned_resolver = Self::assign_resolver(&env, &config)?;

        // #114: hash the free-form text so the event payload stays constant.
        let mut text = Bytes::new(&env);
        text.append(&reason.clone().into());
        text.append(&evidence_uri.clone().into());
        let text_hash: BytesN<32> = env.crypto().sha256(&text).into();

        let dispute = Dispute {
            id: dispute_id,
            job_id,
            raised_by: raiser.clone(),
            reason,
            evidence_uri,
            status: DisputeStatus::Open,
            resolver: Some(assigned_resolver.clone()),
            resolution: None,
            created_at: now,
            resolved_at: None,
            dispute_deadline,
        };

        env.storage().instance().set(&DataKey::Dispute(dispute_id), &dispute);

        // Index dispute
        let mut job_disputes: Vec<u64> = env.storage().instance()
            .get(&DataKey::DisputesByJob(job_id))
            .unwrap_or(Vec::new(&env));
        job_disputes.push_back(dispute_id);
        env.storage().instance().set(&DataKey::DisputesByJob(job_id), &job_disputes);

        // Update job and escrow
        job.dispute_raised = true;
        job.status = JobStatus::Disputed;
        env.storage().instance().set(&DataKey::Job(job_id), &job);

        escrow.status = EscrowStatus::Disputed;
        escrow.dispute_raised_at = Some(now);
        escrow.dispute_deadline = Some(dispute_deadline);
        escrow.resolver = Some(assigned_resolver);
        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);
        Self::extend_instance_ttl(&env);

        // Update stats
        let mut stats = Self::get_platform_stats(&env)?;
        stats.total_disputes += 1;
        env.storage().instance().set(&DataKey::PlatformStats, &stats);

        // #121: track client reputation.
        let mut client_stats = Self::get_client_stats_internal(&env, &job.client);
        client_stats.disputes = client_stats.disputes.saturating_add(1);
        env.storage().instance().set(&DataKey::ClientStats(job.client.clone()), &client_stats);

        // #114: constant-size payload — only ids plus the text hash.
        env.events().publish(
            (symbol_short!("dispute"), job_id, raiser.clone()),
            (dispute_id, job_id, raiser, text_hash),
        );

        Ok(dispute_id)
    }

    /// Resolve dispute (by authorized resolver)
    pub fn resolve_dispute(
        env: Env,
        resolver: Address,
        dispute_id: u64,
        resolution: DisputeResolution,
    ) -> Result<(), Error> {
        resolver.require_auth();
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;

        let mut dispute = Self::get_dispute(&env, dispute_id)?;

        if dispute.status != DisputeStatus::Open && dispute.status != DisputeStatus::UnderReview {
            return Err(Error::DisputeResolved);
        }

        // #116: only the assigned resolver (or the admin) may resolve.
        if dispute.resolver != Some(resolver.clone()) && resolver != config.admin {
            return Err(Error::UnauthorizedResolver);
        }

        // Get job and escrow
        let mut job = Self::get_job(&env, dispute.job_id)?;
        let escrow_id: u64 = env.storage().instance()
            .get(&DataKey::EscrowByJob(dispute.job_id))
            .ok_or(Error::EscrowNotFound)?;
        let mut escrow = Self::get_escrow(&env, escrow_id)?;

        // Deduct the (higher) dispute fee from the escrow payout.
        let dispute_fee = (escrow.amount * config.platform_fee_bps_dispute as i128) / 10000;
        let payout = escrow.amount - dispute_fee;

        // ===== CHECKS-EFFECTS-INTERACTIONS =====
        // All validation above; all state writes below; token transfers last.

        // ----- Effects: write all state before any external call -----
        let (agent_payout, client_payout) = match resolution.clone() {
            DisputeResolution::Agent => {
                escrow.status = EscrowStatus::Released;
                (payout, 0)
            }
            DisputeResolution::Client => {
                escrow.status = EscrowStatus::Refunded;
                job.status = JobStatus::Refunded;
                (0, payout)
            }
            DisputeResolution::Split(percentage) => {
                // #111: the percentage is mandatory (carried by the enum) and
                // validated with a dedicated error.
                if percentage > 100 {
                    return Err(Error::InvalidSplit);
                }
                let agent_amount = (payout * percentage as i128) / 100;
                let client_amount = payout - agent_amount;
                escrow.status = EscrowStatus::Released;
                (agent_amount, client_amount)
            }
        };

        escrow.resolver = Some(resolver.clone());
        escrow.released_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);

        // #117: for agent/split outcomes, run the same finalization as
        // `approve_deliverable` so agent stats, completion time and job status
        // stay consistent.  For "client" the job is refunded, not completed.
        if agent_payout > 0 {
            Self::finalize_job_payout(&env, dispute.job_id, &mut job, &escrow.agent, agent_payout)?;
        } else {
            // Release the locked-escrow accounting now that funds have left.
            let mut updated_config = config.clone();
            updated_config.total_locked_escrow =
                (updated_config.total_locked_escrow - escrow.amount).max(0);
            env.storage().instance().set(&DataKey::Config, &updated_config);
            env.storage().instance().set(&DataKey::Job(dispute.job_id), &job);
        }

        dispute.status = DisputeStatus::Resolved;
        dispute.resolution = Some(resolution.clone());
        dispute.resolved_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::Dispute(dispute_id), &dispute);

        // ----- Interactions: token transfers last -----
        let usdc_client = TokenClient::new(&env, &config.usdc_token);
        if dispute_fee > 0 {
            usdc_client.transfer(
                &env.current_contract_address(),
                &config.platform_fee_recipient,
                &dispute_fee,
            );
        }
        Self::payout_or_claim(&env, &config, dispute.job_id, &escrow.agent, agent_payout);
        Self::payout_or_claim(&env, &config, dispute.job_id, &escrow.client, client_payout);

        env.events().publish(
            (symbol_short!("dispute_res"), dispute.job_id, resolver.clone()),
            (dispute_id, resolver, resolution),
        );

        Ok(())
    }

    /// Leave a review for a job
    pub fn leave_review(
        env: Env,
        reviewer: Address,
        job_id: u64,
        rating: u32,
        comment: String,
    ) -> Result<(), Error> {
        reviewer.require_auth();
        // #118: review entrypoint — allowed while paused.
        Self::extend_instance_ttl(&env);

        let job = Self::get_job(&env, job_id)?;

        // Verify reviewer is part of the job
        if job.client != reviewer && job.assigned_agent != Some(reviewer.clone()) {
            return Err(Error::Unauthorized);
        }

        if job.status != JobStatus::Completed && job.status != JobStatus::Refunded {
            return Err(Error::JobNotCompleted);
        }

        if rating < 1 || rating > 5 {
            return Err(Error::InvalidRating);
        }

        // #123: O(1) duplicate-review check.
        if env.storage().instance()
            .has(&DataKey::ReviewByJobReviewer(job_id, reviewer.clone()))
        {
            return Err(Error::AlreadyReviewed);
        }

        let review_id = Self::next_review_id(&env)?;
        let now = env.ledger().timestamp();

        // #122: never unwrap `assigned_agent`; a Completed/Refunded job without
        // an assigned agent is an error, not a trap.
        let reviewee = if job.client == reviewer {
            job.assigned_agent.clone().ok_or(Error::AgentNotFound)?
        } else {
            job.client.clone()
        };

        let review = Review {
            id: review_id,
            job_id,
            reviewer: reviewer.clone(),
            reviewee: reviewee.clone(),
            rating,
            comment,
            created_at: now,
        };

        env.storage().instance().set(&DataKey::Review(review_id), &review);
        // #123: index by (job_id, reviewer) for O(1) duplicate checks.
        env.storage().instance().set(
            &DataKey::ReviewByJobReviewer(job_id, reviewer.clone()),
            &review_id,
        );

        // Index review
        let mut reviewee_reviews: Vec<u64> = env.storage().instance()
            .get(&DataKey::ReviewsByReviewee(reviewee.clone()))
            .unwrap_or(Vec::new(&env));
        reviewee_reviews.push_back(review_id);
        env.storage().instance().set(&DataKey::ReviewsByReviewee(reviewee.clone()), &reviewee_reviews);

        // #109: index review by its author so a user can list reviews they wrote.
        let mut reviewer_reviews: Vec<u64> = env.storage().instance()
            .get(&DataKey::ReviewsByReviewer(reviewer.clone()))
            .unwrap_or(Vec::new(&env));
        reviewer_reviews.push_back(review_id);
        env.storage().instance().set(&DataKey::ReviewsByReviewer(reviewer.clone()), &reviewer_reviews);

        // #121: when the agent reviews the client, update the client's stats.
        if job.client != reviewer {
            let mut client_stats = Self::get_client_stats_internal(&env, &reviewee);
            client_stats.rating_sum = client_stats.rating_sum.saturating_add(rating);
            client_stats.rating_count = client_stats.rating_count.saturating_add(1);
            env.storage().instance().set(&DataKey::ClientStats(reviewee.clone()), &client_stats);
        }

        // Update agent reputation when the client reviews the agent
        if job.client == reviewer {
            let agent_id: u64 = env.storage().instance()
                .get(&DataKey::AgentByOwner(reviewee.clone()))
                .ok_or(Error::AgentNotFound)?;
            let mut agent = Self::get_agent(&env, agent_id)?;
            // #120: derive reputation exactly from the accumulated rating sum
            // and count instead of a lossy rolling average.  Reputation is a
            // 0-100 scale; round half up.
            agent.rating_sum = agent.rating_sum.saturating_add(rating as u64);
            agent.rating_count = agent.rating_count.saturating_add(1);
            let count = agent.rating_count as u64;
            agent.reputation = ((agent.rating_sum * 20 + count / 2) / count) as u32;
            env.storage().instance().set(&DataKey::Agent(agent_id), &agent);
        }

        env.events().publish(
            (symbol_short!("review"), job_id, reviewer.clone()),
            (review_id, job_id, reviewer, reviewee, rating),
        );

        Ok(())
    }


    /// Reclaim escrow funds after expiry (client only)
    ///
    /// If an agent accepts a job but never submits a deliverable, the client's
    /// USDC would otherwise be locked forever.  This function allows the client
    /// to reclaim the escrowed amount once `config.escrow_expiry_days` have
    /// elapsed since the escrow was created.
    ///
    /// Preconditions:
    ///   - Caller must be the job's client.
    ///   - Escrow status must be `Funded` (not already released/refunded/disputed).
    ///   - `now > escrow.created_at + (config.escrow_expiry_days * 86_400)`.
    ///
    /// Effects:
    ///   - Transfers `escrow.amount` back to the client.
    ///   - Sets `escrow.status = EscrowStatus::Refunded`.
    ///   - Sets `job.status    = JobStatus::Refunded`.
    ///   - Emits a `refund` event.
    pub fn reclaim_expired_escrow(
        env: Env,
        client: Address,
        job_id: u64,
    ) -> Result<(), Error> {
        client.require_auth();
        // #118: refund entrypoint — allowed while paused.
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        let mut job = Self::get_job(&env, job_id)?;

        // Only the job's client may reclaim
        if job.client != client {
            return Err(Error::Unauthorized);
        }

        // Look up the escrow
        let escrow_id: u64 = env.storage().instance()
            .get(&DataKey::EscrowByJob(job_id))
            .ok_or(Error::EscrowNotFound)?;
        let mut escrow = Self::get_escrow(&env, escrow_id)?;

        // Escrow must still be in Funded state
        if escrow.status != EscrowStatus::Funded {
            return Err(Error::EscrowNotFunded);
        }

        // Verify the expiry window has passed
        let expiry_seconds = config.escrow_expiry_days * 86_400;
        let expiry_time = escrow.created_at + expiry_seconds;
        if env.ledger().timestamp() <= expiry_time {
            // EscrowExpired is repurposed here to mean 'not yet expired / too early to reclaim'.
            // A future refactor may introduce a dedicated EscrowNotExpired error variant.
            return Err(Error::EscrowExpired);
        }

        // ===== CHECKS-EFFECTS-INTERACTIONS =====
        // All validation above; all state writes below; token transfers last.

        // ----- Effects: write all state before any external call -----

        // Update escrow state
        escrow.status = EscrowStatus::Refunded;
        escrow.released_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);

        // Release the locked-escrow accounting now that funds have left.
        let mut updated_config = config.clone();
        updated_config.total_locked_escrow =
            (updated_config.total_locked_escrow - escrow.amount).max(0);
        env.storage().instance().set(&DataKey::Config, &updated_config);

        // Update job state
        job.status = JobStatus::Refunded;
        env.storage().instance().set(&DataKey::Job(job_id), &job);

        // ----- Interactions: transfer the escrowed amount back to the client -----
        let usdc_client = TokenClient::new(&env, &config.usdc_token);
        usdc_client.transfer(
            &env.current_contract_address(),
            &client,
            &escrow.amount,
        );

        // Emit refund event
        env.events().publish(
            (symbol_short!("refund"), job_id, client.clone()),
            (job_id, client, escrow.amount),
        );

        Ok(())
    }

    /// #171: Reclaim escrow after the job deadline elapses without an accepted
    /// deliverable.
    ///
    /// Permissionless in the sense that only the client can call it, but it
    /// requires no admin involvement.  Preconditions:
    ///   - Caller is the job's client.
    ///   - `now > job.deadline`.
    ///   - The job has not been approved/completed (`JobStatus::Completed`).
    ///   - Escrow is still `Funded`.
    ///
    /// Effects: refunds the client, sets escrow/job to `Refunded`, emits
    /// `reclaim`.
    pub fn reclaim_escrow(env: Env, client: Address, job_id: u64) -> Result<(), Error> {
        client.require_auth();
        // #118: refund entrypoint — allowed while paused.
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        let mut job = Self::get_job(&env, job_id)?;

        if job.client != client {
            return Err(Error::Unauthorized);
        }

        // Cannot reclaim once the deliverable has been approved.
        if job.status == JobStatus::Completed {
            return Err(Error::AlreadyApproved);
        }

        // Only allowed strictly after the deadline.
        if env.ledger().timestamp() <= job.deadline {
            return Err(Error::DeadlineNotReached);
        }

        let escrow_id: u64 = env.storage().instance()
            .get(&DataKey::EscrowByJob(job_id))
            .ok_or(Error::EscrowNotFound)?;
        let mut escrow = Self::get_escrow(&env, escrow_id)?;

        if escrow.status != EscrowStatus::Funded {
            return Err(Error::EscrowNotFunded);
        }

        // ===== CHECKS-EFFECTS-INTERACTIONS =====
        // All validation above; all state writes below; token transfers last.

        // ----- Effects: write all state before any external call -----
        escrow.status = EscrowStatus::Refunded;
        escrow.released_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);

        job.status = JobStatus::Refunded;
        env.storage().instance().set(&DataKey::Job(job_id), &job);
        Self::extend_instance_ttl(&env);

        // ----- Interactions: transfer the escrowed amount back to the client -----
        let usdc_client = TokenClient::new(&env, &config.usdc_token);
        usdc_client.transfer(
            &env.current_contract_address(),
            &client,
            &escrow.amount,
        );

        env.events().publish(
            (symbol_short!("reclaim"), job_id, client.clone()),
            (job_id, client, escrow.amount),
        );

        Ok(())
    }

    /// #169: Withdraw a claimable balance recorded when a payout could not be
    /// delivered because the recipient had no USDC trustline.
    ///
    /// Callable by the recorded recipient once they can receive USDC.
    pub fn claim_funds(env: Env, agent: Address, job_id: u64) -> Result<(), Error> {
        agent.require_auth();
        // #118: settlement entrypoint — allowed while paused.
        Self::extend_instance_ttl(&env);

        let claim: Claimable = env.storage().instance()
            .get(&DataKey::Claimable(job_id))
            .ok_or(Error::NothingToClaim)?;

        if claim.recipient != agent {
            return Err(Error::Unauthorized);
        }

        let config = Self::get_config(&env)?;
        if !Self::is_payable(&env, &claim.token, &agent) {
            return Err(Error::AgentNotPayable);
        }

        // ===== CHECKS-EFFECTS-INTERACTIONS =====
        // All validation above; all state writes below; token transfers last.

        // ----- Effects: write all state before any external call -----
        env.storage().instance().remove(&DataKey::Claimable(job_id));
        Self::extend_instance_ttl(&env);

        // ----- Interactions: transfer the claimable amount to the recipient -----
        let usdc_client = TokenClient::new(&env, &claim.token);
        usdc_client.transfer(
            &env.current_contract_address(),
            &agent,
            &claim.amount,
        );

        env.events().publish(
            (symbol_short!("claim"), job_id, agent.clone()),
            (job_id, agent, claim.amount, config.usdc_token),
        );

        Ok(())
    }

    /// #170: Deterministic fallback resolution once the dispute window has
    /// elapsed without an admin resolution.
    ///
    /// Callable by either the client or the assigned agent.  Applies a fixed
    /// split (`DISPUTE_FALLBACK_AGENT_PCT` to the agent, remainder to the
    /// client) minus the dispute fee, and marks the dispute resolved.
    pub fn resolve_dispute_fallback(
        env: Env,
        caller: Address,
        dispute_id: u64,
    ) -> Result<(), Error> {
        caller.require_auth();
        // #118: dispute resolution entrypoint — allowed while paused.
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        let mut dispute = Self::get_dispute(&env, dispute_id)?;

        if dispute.status != DisputeStatus::Open && dispute.status != DisputeStatus::UnderReview {
            return Err(Error::DisputeResolved);
        }

        let mut job = Self::get_job(&env, dispute.job_id)?;

        // Only the two parties to the job may trigger the fallback.
        if job.client != caller && job.assigned_agent != Some(caller.clone()) {
            return Err(Error::Unauthorized);
        }

        // Window must have elapsed.
        if env.ledger().timestamp() <= dispute.dispute_deadline {
            return Err(Error::DisputeWindowNotExpired);
        }

        let escrow_id: u64 = env.storage().instance()
            .get(&DataKey::EscrowByJob(dispute.job_id))
            .ok_or(Error::EscrowNotFound)?;
        let mut escrow = Self::get_escrow(&env, escrow_id)?;

        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::EscrowNotDisputed);
        }

        let dispute_fee = (escrow.amount * config.platform_fee_bps_dispute as i128) / 10000;
        let payout = escrow.amount - dispute_fee;

        let agent_amount = (payout * DISPUTE_FALLBACK_AGENT_PCT as i128) / 100;
        let client_amount = payout - agent_amount;

        // ===== CHECKS-EFFECTS-INTERACTIONS =====
        // All validation above; all state writes below; token transfers last.

        // ----- Effects: write all state before any external call -----
        escrow.status = EscrowStatus::Released;
        escrow.released_at = Some(env.ledger().timestamp());
        escrow.resolver = Some(caller.clone());
        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);

        dispute.status = DisputeStatus::Resolved;
        dispute.resolution = Some(DisputeResolution::Split(DISPUTE_FALLBACK_AGENT_PCT));
        dispute.resolved_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::Dispute(dispute_id), &dispute);

        job.status = JobStatus::Completed;
        env.storage().instance().set(&DataKey::Job(dispute.job_id), &job);
        Self::extend_instance_ttl(&env);

        // ----- Interactions: token transfers last -----
        let usdc_client = TokenClient::new(&env, &config.usdc_token);
        if dispute_fee > 0 {
            usdc_client.transfer(
                &env.current_contract_address(),
                &config.platform_fee_recipient,
                &dispute_fee,
            );
        }
        Self::payout_or_claim(&env, &config, dispute.job_id, &escrow.agent, agent_amount);
        Self::payout_or_claim(&env, &config, dispute.job_id, &escrow.client, client_amount);

        env.events().publish(
            (symbol_short!("disp_fb"), dispute.job_id, caller.clone()),
            (dispute_id, caller, agent_amount, client_amount),
        );

        Ok(())
    }

    /// #115: Apply the fallback resolution once the resolver deadline has
    /// elapsed without an admin/resolver decision.
    ///
    /// Permissionless: anyone may call this after `dispute.dispute_deadline`.
    /// The post-fee payout is split per `config.resolver_timeout_agent_pct`
    /// (agent) with the remainder refunded to the client, so the full escrow
    /// amount is always accounted for.
    pub fn claim_after_resolver_timeout(
        env: Env,
        caller: Address,
        dispute_id: u64,
    ) -> Result<(), Error> {
        caller.require_auth();
        Self::ensure_not_paused(&env)?;
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        let mut dispute = Self::get_dispute(&env, dispute_id)?;

        if dispute.status != DisputeStatus::Open && dispute.status != DisputeStatus::UnderReview {
            return Err(Error::DisputeResolved);
        }

        // Deadline must have elapsed.
        if env.ledger().timestamp() <= dispute.dispute_deadline {
            return Err(Error::ResolutionDeadlineNotReached);
        }

        let mut job = Self::get_job(&env, dispute.job_id)?;

        let escrow_id: u64 = env.storage().instance()
            .get(&DataKey::EscrowByJob(dispute.job_id))
            .ok_or(Error::EscrowNotFound)?;
        let mut escrow = Self::get_escrow(&env, escrow_id)?;

        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::EscrowNotDisputed);
        }

        let dispute_fee = (escrow.amount * config.platform_fee_bps_dispute as i128) / 10000;
        let payout = escrow.amount - dispute_fee;

        let pct = config.resolver_timeout_agent_pct.min(100);
        let agent_amount = (payout * pct as i128) / 100;
        let client_amount = payout - agent_amount;

        // ===== CHECKS-EFFECTS-INTERACTIONS =====
        // All validation above; all state writes below; token transfers last.

        // ----- Effects: write all state before any external call -----
        escrow.status = EscrowStatus::Released;
        escrow.released_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);

        // Release the locked-escrow accounting now that funds have left.
        let mut updated_config = config.clone();
        updated_config.total_locked_escrow =
            (updated_config.total_locked_escrow - escrow.amount).max(0);
        env.storage().instance().set(&DataKey::Config, &updated_config);

        dispute.status = DisputeStatus::Resolved;
        dispute.resolution = Some(String::from_str(&env, "resolver_timeout"));
        dispute.resolved_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::Dispute(dispute_id), &dispute);

        job.status = JobStatus::Completed;
        env.storage().instance().set(&DataKey::Job(dispute.job_id), &job);
        Self::extend_instance_ttl(&env);

        // ----- Interactions: token transfers last -----
        let usdc_client = TokenClient::new(&env, &config.usdc_token);
        if dispute_fee > 0 {
            usdc_client.transfer(
                &env.current_contract_address(),
                &config.platform_fee_recipient,
                &dispute_fee,
            );
        }
        Self::payout_or_claim(&env, &config, dispute.job_id, &escrow.agent, agent_amount);
        Self::payout_or_claim(&env, &config, dispute.job_id, &escrow.client, client_amount);

        env.events().publish(
            (symbol_short!("disp_to"), dispute.job_id, caller.clone()),
            (dispute_id, caller, agent_amount, client_amount),
        );

        Ok(())
    }

    /// #116: Admin — add a dispute resolver to the managed set.
    pub fn add_resolver(env: Env, admin: Address, resolver: Address) -> Result<(), Error> {
        admin.require_auth();
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        if config.admin != admin {
            return Err(Error::Unauthorized);
        }

        let mut resolvers: Vec<Address> = env.storage().instance()
            .get(&DataKey::Resolvers)
            .unwrap_or(Vec::new(&env));
        if !resolvers.contains(&resolver) {
            resolvers.push_back(resolver.clone());
            env.storage().instance().set(&DataKey::Resolvers, &resolvers);
        }

        env.events().publish(
            (symbol_short!("res_add"), admin),
            resolver,
        );

        Ok(())
    }

    /// #116: Admin — remove a dispute resolver from the managed set.
    pub fn remove_resolver(env: Env, admin: Address, resolver: Address) -> Result<(), Error> {
        admin.require_auth();
        Self::extend_instance_ttl(&env);

        let config = Self::get_config(&env)?;
        if config.admin != admin {
            return Err(Error::Unauthorized);
        }

        let resolvers: Vec<Address> = env.storage().instance()
            .get(&DataKey::Resolvers)
            .unwrap_or(Vec::new(&env));
        let mut updated: Vec<Address> = Vec::new(&env);
        for r in resolvers.iter() {
            if r != resolver {
                updated.push_back(r);
            }
        }
        env.storage().instance().set(&DataKey::Resolvers, &updated);

        env.events().publish(
            (symbol_short!("res_rem"), admin),
            resolver,
        );

        Ok(())
    }

    /// #116: Get the current dispute resolver set.
    pub fn get_resolvers(env: Env) -> Vec<Address> {
        env.storage().instance()
            .get(&DataKey::Resolvers)
            .unwrap_or(Vec::new(&env))
    }

    /// #172: Keeper entrypoint — bump the TTL of a job and its escrow.
    ///
    /// Anyone may call this to keep long-running escrows alive on mainnet.
    pub fn bump_ttl(env: Env, job_id: u64) -> Result<(), Error> {
        Self::extend_instance_ttl(&env);

        // Touch the job so its TTL is extended.
        let _ = Self::get_job(&env, job_id)?;

        // Touch the escrow, if any.
        if let Some(escrow_id) = env.storage().instance()
            .get::<DataKey, u64>(&DataKey::EscrowByJob(job_id))
        {
            let _ = Self::get_escrow(&env, escrow_id)?;
        }

        env.events().publish(
            (symbol_short!("ttl_bump"), job_id),
            (job_id, env.ledger().sequence()),
        );

        Ok(())
    }

    /// Get agent by ID
    pub fn get_agent(env: Env, agent_id: u64) -> Result<Agent, Error> {
        Self::get_agent(&env, agent_id)
    }

    /// Get job by ID
    pub fn get_job(env: Env, job_id: u64) -> Result<Job, Error> {
        Self::get_job(&env, job_id)
    }

    /// Get bid by ID
    pub fn get_bid(env: Env, bid_id: u64) -> Result<Bid, Error> {
        Self::get_bid(&env, bid_id)
    }

    /// Get escrow by ID
    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<Escrow, Error> {
        Self::get_escrow(&env, escrow_id)
    }

    /// Get dispute by ID
    pub fn get_dispute(env: Env, dispute_id: u64) -> Result<Dispute, Error> {
        Self::get_dispute(&env, dispute_id)
    }

    /// #107: Get the dispute IDs raised against a job.
    ///
    /// Returns an empty Vec when the job has no disputes (including unknown
    /// job IDs), so callers need not distinguish "no disputes" from "no job".
    pub fn get_disputes_by_job(env: Env, job_id: u64) -> Vec<u64> {
        env.storage().instance()
            .get(&DataKey::DisputesByJob(job_id))
            .unwrap_or(Vec::new(&env))
    }

    /// #107: Convenience getter returning the first open dispute for a job.
    ///
    /// Scans the job's dispute index and returns the ID of the first dispute
    /// whose status is `DisputeStatus::Open`.  Returns `None` when the job has
    /// no open dispute (or no disputes at all).
    pub fn get_open_dispute(env: Env, job_id: u64) -> Option<u64> {
        let dispute_ids: Vec<u64> = env.storage().instance()
            .get(&DataKey::DisputesByJob(job_id))
            .unwrap_or(Vec::new(&env));
        for dispute_id in dispute_ids.iter() {
            if let Ok(dispute) = Self::get_dispute(&env, dispute_id) {
                if dispute.status == DisputeStatus::Open {
                    return Some(dispute_id);
                }
            }
        }
        None
    }

    /// #108: Get a skill verification record by ID.
    ///
    /// Returns `None` when no verification with the given ID exists.
    pub fn get_verification(env: Env, verification_id: u64) -> Option<SkillVerification> {
        env.storage().instance()
            .get(&DataKey::SkillVerification(verification_id))
    }

    /// #108: Get all skill verification records for an agent owner.
    ///
    /// Returns the stored `SkillVerification` structs (verifier, evidence URI,
    /// expiry, etc.) in the order they were recorded.  Returns an empty Vec
    /// when the address has no verifications.
    pub fn get_verifications_by_agent(env: Env, agent: Address) -> Vec<SkillVerification> {
        let ids: Vec<u64> = env.storage().instance()
            .get(&DataKey::SkillVerificationsByAgent(agent))
            .unwrap_or(Vec::new(&env));
        let mut verifications = Vec::new(&env);
        for id in ids.iter() {
            if let Some(ver) = env.storage().instance()
                .get::<DataKey, SkillVerification>(&DataKey::SkillVerification(id))
            {
                verifications.push_back(ver);
            }
        }
        verifications
    }

    /// Get review by ID
    pub fn get_review(env: Env, review_id: u64) -> Result<Review, Error> {
        Self::get_review(&env, review_id)
    }

    /// Get jobs by client (paginated).
    ///
    /// Returns a slice of the client's job-ID list starting at `offset` with
    /// at most `limit` entries.  Returns an empty Vec when `offset` is beyond
    /// the end of the stored list or when `limit` is 0.
    ///
    /// Clients should walk pages by incrementing `offset` by `limit` until the
    /// returned Vec has fewer elements than `limit` (indicating the final page).
    pub fn get_jobs_by_client(
        env: Env,
        client: Address,
        offset: u64,
        limit: u64,
    ) -> Vec<u64> {
        let all: Vec<u64> = env.storage().instance()
            .get(&DataKey::JobsByClient(client))
            .unwrap_or(Vec::new(&env));
        Self::paginate(&env, &all, offset, limit)
    }

    /// Get jobs by agent (paginated).
    ///
    /// Returns a slice of the agent's job-ID list starting at `offset` with
    /// at most `limit` entries.  Returns an empty Vec when `offset` is out of
    /// bounds or `limit` is 0.
    pub fn get_jobs_by_agent(
        env: Env,
        agent: Address,
        offset: u64,
        limit: u64,
    ) -> Vec<u64> {
        let all: Vec<u64> = env.storage().instance()
            .get(&DataKey::JobsByAgent(agent))
            .unwrap_or(Vec::new(&env));
        Self::paginate(&env, &all, offset, limit)
    }

    /// Get bids by job (paginated).
    ///
    /// Returns a slice of bid IDs for the given job starting at `offset` with
    /// at most `limit` entries.
    pub fn get_bids_by_job(
        env: Env,
        job_id: u64,
        offset: u64,
        limit: u64,
    ) -> Vec<u64> {
        let all: Vec<u64> = env.storage().instance()
            .get(&DataKey::BidsByJob(job_id))
            .unwrap_or(Vec::new(&env));
        Self::paginate(&env, &all, offset, limit)
    }

    /// Get bids by agent (paginated).
    ///
    /// Returns a slice of the agent's bid-ID list starting at `offset` with
    /// at most `limit` entries.
    pub fn get_bids_by_agent(
        env: Env,
        agent: Address,
        offset: u64,
        limit: u64,
    ) -> Vec<u64> {
        let all: Vec<u64> = env.storage().instance()
            .get(&DataKey::BidsByAgent(agent))
            .unwrap_or(Vec::new(&env));
        Self::paginate(&env, &all, offset, limit)
    }

    /// #109: Get reviews written by a reviewer (paginated).
    ///
    /// Returns a slice of review IDs authored by `reviewer` starting at
    /// `offset` with at most `limit` entries.
    pub fn get_reviews_by_reviewer(
        env: Env,
        reviewer: Address,
        offset: u64,
        limit: u64,
    ) -> Vec<u64> {
        let all: Vec<u64> = env.storage().instance()
            .get(&DataKey::ReviewsByReviewer(reviewer))
            .unwrap_or(Vec::new(&env));
        Self::paginate(&env, &all, offset, limit)
    }

    /// Get reviews for an agent/user (paginated).
    ///
    /// Returns a slice of review IDs for the reviewee starting at `offset`
    /// with at most `limit` entries.
    pub fn get_reviews_by_reviewee(
        env: Env,
        reviewee: Address,
        offset: u64,
        limit: u64,
    ) -> Vec<u64> {
        let all: Vec<u64> = env.storage().instance()
            .get(&DataKey::ReviewsByReviewee(reviewee))
            .unwrap_or(Vec::new(&env));
        Self::paginate(&env, &all, offset, limit)
    }

    /// Get agents by skill (paginated).
    ///
    /// Returns a slice of agent IDs that have the given skill starting at
    /// `offset` with at most `limit` entries.
    pub fn get_agents_by_skill(
        env: Env,
        skill: String,
        offset: u64,
        limit: u64,
    ) -> Vec<u64> {
        let all: Vec<u64> = env.storage().instance()
            .get(&DataKey::SkillAgents(skill))
            .unwrap_or(Vec::new(&env));
        // #125: hide deactivated agents from skill indexes.
        let mut active = Vec::new(&env);
        for id in all.iter() {
            if let Ok(agent) = Self::get_agent(&env, id) {
                if agent.is_active {
                    active.push_back(id);
                }
            }
        }
        Self::paginate(&env, &active, offset, limit)
    }

    /// Get all agents (paginated).
    ///
    /// Returns full `Agent` records for the page starting at `offset` with at
    /// most `limit` entries. Agent IDs are sequential and never deleted, so a
    /// list index `i` maps to agent id `i + 1`. Returns an empty Vec when
    /// `offset` is beyond the last agent or `limit` is 0.
    pub fn get_all_agents(env: Env, offset: u64, limit: u64) -> Vec<Agent> {
        let count: u64 = env.storage().instance()
            .get(&DataKey::AgentCounter)
            .unwrap_or(0);
        let mut page = Vec::new(&env);
        if limit == 0 || offset >= count {
            return page;
        }
        let end = (offset + limit).min(count);
        for idx in offset..end {
            let id = idx + 1;
            if let Ok(agent) = Self::get_agent(&env, id) {
                page.push_back(agent);
            }
        }
        page
    }

    /// Get all jobs (paginated).
    ///
    /// Returns full `Job` records for the page starting at `offset` with at
    /// most `limit` entries. Job IDs are sequential and never deleted, so a
    /// list index `i` maps to job id `i + 1`. Callers can filter the returned
    /// page client-side by status (e.g. only show `Open` jobs). Returns an
    /// empty Vec when `offset` is beyond the last job or `limit` is 0.
    pub fn get_all_jobs(env: Env, offset: u64, limit: u64) -> Vec<Job> {
        let count: u64 = env.storage().instance()
            .get(&DataKey::JobCounter)
            .unwrap_or(0);
        let mut page = Vec::new(&env);
        if limit == 0 || offset >= count {
            return page;
        }
        let end = (offset + limit).min(count);
        for idx in offset..end {
            let id = idx + 1;
            if let Ok(job) = Self::get_job(&env, id) {
                page.push_back(job);
            }
        }
        page
    }

    /// Get the agent owned by `owner`, if any.
    ///
    /// Returns `Some(Agent)` when the owner has registered an agent, otherwise
    /// `None`.
    pub fn get_agent_by_owner(env: Env, owner: Address) -> Option<Agent> {
        let agent_id: Option<u64> = env.storage().instance()
            .get(&DataKey::AgentByOwner(owner));
        agent_id.and_then(|id| Self::get_agent(&env, id).ok())
    }

    /// Get the escrow funding a job.
    ///
    /// Returns `Err(Error::EscrowNotFound)` when no bid has been accepted for
    /// the job yet (or the escrow record is missing).
    pub fn get_escrow_by_job(env: Env, job_id: u64) -> Result<Escrow, Error> {
        let escrow_id: u64 = env.storage().instance()
            .get(&DataKey::EscrowByJob(job_id))
            .ok_or(Error::EscrowNotFound)?;
        Self::get_escrow(&env, escrow_id)
    }

    /// #121: Get aggregated reputation stats for a client.
    ///
    /// Returns zeroed stats when the client has no recorded activity yet.
    pub fn get_client_stats(env: Env, client: Address) -> ClientStats {
        Self::get_client_stats_internal(&env, &client)
    }

    /// Get platform configuration
    pub fn get_config(env: Env) -> Result<Config, Error> {
        Self::get_config(&env)
    }

    /// Get platform statistics
    pub fn get_platform_stats(env: Env) -> PlatformStats {
        Self::get_platform_stats(&env).unwrap_or(PlatformStats {
            total_agents: 0,
            total_jobs: 0,
            total_volume: 0,
            total_fees_collected: 0,
            total_disputes: 0,
            avg_job_completion_time: 0,
            open_jobs: 0,
            active_agents: 0,
        })
    }

    /// Admin: Update configuration
    pub fn update_config(
        env: Env,
        admin: Address,
        platform_fee_bps: Option<u32>,
        min_job_budget: Option<i128>,
        max_job_deadline_days: Option<u64>,
        dispute_window_days: Option<u64>,
        platform_fee_recipient: Option<Address>,
        dispute_resolver: Option<Address>,
        min_agent_reputation_to_bid: Option<u32>,
        platform_fee_bps_dispute: Option<u32>,
        escrow_expiry_days: Option<u64>,
        skill_verifiers: Option<Vec<Address>>,
        require_skill_verification: Option<bool>,
        resolution_deadline_days: Option<u64>,
        resolver_timeout_agent_pct: Option<u32>,
    ) -> Result<(), Error> {
        admin.require_auth();
        Self::extend_instance_ttl(&env);

        let mut config = Self::get_config(&env)?;
        if config.admin != admin {
            return Err(Error::Unauthorized);
        }

        // #126: emit one `config_upd` event per changed field, carrying
        // `(field_symbol, old_value, new_value)` so indexers need not refetch.
        if let Some(fee) = platform_fee_bps {
            if fee != config.platform_fee_bps {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("fee_bps")),
                    (config.platform_fee_bps, fee),
                );
            }
            config.platform_fee_bps = fee;
        }
        if let Some(budget) = min_job_budget {
            if budget != config.min_job_budget {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("min_budget")),
                    (config.min_job_budget, budget),
                );
            }
            config.min_job_budget = budget;
        }
        if let Some(days) = max_job_deadline_days {
            if days != config.max_job_deadline_days {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("max_days")),
                    (config.max_job_deadline_days, days),
                );
            }
            config.max_job_deadline_days = days;
        }
        if let Some(days) = dispute_window_days {
            if days != config.dispute_window_days {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("disp_win")),
                    (config.dispute_window_days, days),
                );
            }
            config.dispute_window_days = days;
        }
        if let Some(recipient) = platform_fee_recipient {
            if recipient != config.platform_fee_recipient {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("fee_rcpt")),
                    (config.platform_fee_recipient.clone(), recipient.clone()),
                );
            }
            config.platform_fee_recipient = recipient;
        }
        if let Some(resolver) = dispute_resolver {
            if resolver != config.dispute_resolver {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("resolver")),
                    (config.dispute_resolver.clone(), resolver.clone()),
                );
            }
            config.dispute_resolver = resolver;
        }
        if let Some(rep) = min_agent_reputation_to_bid {
            if rep != config.min_agent_reputation_to_bid {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("bid_rep")),
                    (config.min_agent_reputation_to_bid, rep),
                );
            }
            config.min_agent_reputation_to_bid = rep;
        }
        if let Some(fee) = platform_fee_bps_dispute {
            if fee != config.platform_fee_bps_dispute {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("disp_fee")),
                    (config.platform_fee_bps_dispute, fee),
                );
            }
            config.platform_fee_bps_dispute = fee;
        }
        if let Some(days) = escrow_expiry_days {
            if days != config.escrow_expiry_days {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("esc_exp")),
                    (config.escrow_expiry_days, days),
                );
            }
            config.escrow_expiry_days = days;
        }
        if let Some(days) = resolution_deadline_days {
            if days != config.resolution_deadline_days {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("res_days")),
                    (config.resolution_deadline_days, days),
                );
            }
            config.resolution_deadline_days = days;
        }
        if let Some(pct) = resolver_timeout_agent_pct {
            if pct > 100 {
                return Err(Error::InvalidConfig);
            }
            if pct != config.resolver_timeout_agent_pct {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("res_pct")),
                    (config.resolver_timeout_agent_pct, pct),
                );
            }
            config.resolver_timeout_agent_pct = pct;
        }
        if let Some(verifiers) = skill_verifiers {
            if verifiers != config.skill_verifiers {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("verifiers")),
                    (config.skill_verifiers.clone(), verifiers.clone()),
                );
            }
            config.skill_verifiers = verifiers;
        }
        if let Some(flag) = require_skill_verification {
            if flag != config.require_skill_verification {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("req_skill")),
                    (config.require_skill_verification, flag),
                );
            }
            config.require_skill_verification = flag;
        }
        if let Some(rep) = starting_reputation {
            if rep != config.starting_reputation {
                env.events().publish(
                    (symbol_short!("config_upd"), symbol_short!("start_rep")),
                    (config.starting_reputation, rep),
                );
            }
            config.starting_reputation = rep;
        }

        // #103 / #104: validate the fully-applied config before persisting.
        Self::validate_config(&config)?;

        env.storage().instance().set(&DataKey::Config, &config);

        Ok(())
    }

    /// Admin: Add skill verifier
    pub fn add_skill_verifier(env: Env, admin: Address, verifier: Address) -> Result<(), Error> {
        admin.require_auth();

        let mut config = Self::get_config(&env)?;
        if config.admin != admin {
            return Err(Error::Unauthorized);
        }

        if !config.skill_verifiers.contains(&verifier) {
            config.skill_verifiers.push_back(verifier.clone());
            env.storage().instance().set(&DataKey::Config, &config);
        }

        Ok(())
    }

    /// Admin: Remove skill verifier
    pub fn remove_skill_verifier(env: Env, admin: Address, verifier: Address) -> Result<(), Error> {
        admin.require_auth();

        let mut config = Self::get_config(&env)?;
        if config.admin != admin {
            return Err(Error::Unauthorized);
        }

        config.skill_verifiers = config.skill_verifiers.iter()
            .filter(|v| v != &verifier)
            .collect();
        env.storage().instance().set(&DataKey::Config, &config);

        Ok(())
    }

    // ========== Helper Functions ==========
}

// Private helpers — separate impl block so they are not exposed as contract
// entry-points by the #[contractimpl] macro above.
impl AiAgentMarketplace {
    // ========== Helper Functions ==========

    /// Slice a Vec<u64> from `offset` up to `offset + limit`.
    ///
    /// - Returns an empty Vec when `offset >= all.len()` or `limit == 0`.
    /// - Clamps the end index to the actual length so callers never need to
    ///   worry about out-of-bounds accesses.
    fn paginate(env: &Env, all: &Vec<u64>, offset: u64, limit: u64) -> Vec<u64> {
        let total = all.len() as u64;
        if limit == 0 || offset >= total {
            return Vec::new(env);
        }
        let start = offset.min(total);
        let end_exclusive = (start + limit).min(total);
        let mut page = Vec::new(env);
        for i in start..end_exclusive {
            page.push_back(all.get(i as u32).unwrap());
        }
        page
    }

    fn has_config(env: &Env) -> bool {
        env.storage().instance().has(&DataKey::Config)
    }

    /// #103 / #104: Validate every `Config` field.
    ///
    /// Rules:
    /// - `min_job_budget` must be strictly positive (`> 0`), otherwise
    ///   zero-value jobs would be allowed through.
    /// - `max_job_deadline_days`, `dispute_window_days` and
    ///   `escrow_expiry_days` must be strictly positive (`> 0`); a zero
    ///   deadline cap would make `post_job` always fail.
    /// - `platform_fee_bps` and `platform_fee_bps_dispute` must be within
    ///   `0..=10000` basis points (0% to 100%).
    /// - `min_agent_reputation_to_bid` must be within `0..=100` because
    ///   reputation is stored as a 0-100 score (rating * 20).
    ///
    /// Returns `Error::InvalidConfig` when any rule is violated.
    fn validate_config(config: &Config) -> Result<(), Error> {
        if config.min_job_budget <= 0 {
            return Err(Error::InvalidConfig);
        }
        if config.max_job_deadline_days == 0 {
            return Err(Error::InvalidConfig);
        }
        if config.dispute_window_days == 0 {
            return Err(Error::InvalidConfig);
        }
        if config.escrow_expiry_days == 0 {
            return Err(Error::InvalidConfig);
        }
        if config.platform_fee_bps > 10000 {
            return Err(Error::InvalidConfig);
        }
        if config.platform_fee_bps_dispute > 10000 {
            return Err(Error::InvalidConfig);
        }
        if config.min_agent_reputation_to_bid > 100 {
            return Err(Error::InvalidConfig);
        }
        Ok(())
    }

    /// #172: Extend the contract instance TTL.  Called on every entrypoint so
    /// the instance never expires while the contract is in use.
    fn extend_instance_ttl(env: &Env) {
        env.storage().instance().extend_ttl(MIN_INSTANCE_TTL, EXTEND_INSTANCE_TTL);
    }

    /// #169: Returns `true` when `addr` can receive `token` (i.e. has a
    /// trustline).  We probe by attempting a zero-value transfer from the
    /// address to itself; a missing trustline makes the token contract panic,
    /// which we catch via `try_`-style probing using the client's `try_transfer`.
    fn is_payable(env: &Env, token: &Address, addr: &Address) -> bool {
        let client = TokenClient::new(env, token);
        // A zero-amount self-transfer succeeds iff the trustline exists.
        client.try_transfer(addr, addr, &0).is_ok()
    }

    /// #169: Record a claimable balance for `recipient` on `job_id`.
    fn record_claimable(
        env: &Env,
        job_id: u64,
        recipient: &Address,
        amount: i128,
        token: &Address,
    ) {
        let claim = Claimable {
            job_id,
            recipient: recipient.clone(),
            amount,
            token: token.clone(),
            created_at: env.ledger().timestamp(),
        };
        env.storage().instance().set(&DataKey::Claimable(job_id), &claim);
        env.events().publish(
            (symbol_short!("claimable"), job_id),
            (job_id, recipient.clone(), amount),
        );
    }

    /// #169: Transfer `amount` to `recipient`, or record a claimable balance
    /// when the recipient cannot receive USDC yet.
    fn payout_or_claim(
        env: &Env,
        config: &Config,
        job_id: u64,
        recipient: &Address,
        amount: i128,
    ) {
        if amount <= 0 {
            return;
        }
        if Self::is_payable(env, &config.usdc_token, recipient) {
            let usdc_client = TokenClient::new(env, &config.usdc_token);
            usdc_client.transfer(&env.current_contract_address(), recipient, &amount);
        } else {
            Self::record_claimable(env, job_id, recipient, amount, &config.usdc_token);
        }
    }

    /// #117: Shared finalization for a job whose escrow is being paid out to
    /// the agent (either via `approve_deliverable` or a dispute resolved in
    /// the agent's favour / split).  Credits the agent's earnings and completed
    /// job count, marks the job `Completed` with a `completed_at` timestamp,
    /// updates the platform's average completion time, and releases the
    /// locked-escrow accounting.  Callers must have already updated the escrow
    /// record before invoking this.
    fn finalize_job_payout(
        env: &Env,
        job_id: u64,
        job: &mut Job,
        agent_owner: &Address,
        agent_amount: i128,
    ) -> Result<(), Error> {
        let now = env.ledger().timestamp();

        // Update agent stats.
        let agent_id: u64 = env.storage().instance()
            .get(&DataKey::AgentByOwner(agent_owner.clone()))
            .ok_or(Error::AgentNotFound)?;
        let mut agent = Self::get_agent(env, agent_id)?;
        agent.total_jobs_completed = agent.total_jobs_completed.saturating_add(1);
        agent.total_earnings += agent_amount;
        env.storage().instance().set(&DataKey::Agent(agent_id), &agent);

        // Update job: mark completed and stamp the completion time.
        job.status = JobStatus::Completed;
        if job.completed_at.is_none() {
            job.completed_at = Some(now);
        }
        env.storage().instance().set(&DataKey::Job(job_id), job);

        // Update platform stats (average completion time).
        let mut stats = Self::get_platform_stats(env)?;
        let elapsed = now.saturating_sub(job.created_at);
        stats.avg_job_completion_time =
            (stats.avg_job_completion_time * stats.total_jobs + elapsed) / (stats.total_jobs + 1);
        env.storage().instance().set(&DataKey::PlatformStats, &stats);

        // Release the locked-escrow accounting now that funds have left.
        let mut config = Self::get_config(env)?;
        config.total_locked_escrow = (config.total_locked_escrow - job.escrow_amount).max(0);
        env.storage().instance().set(&DataKey::Config, &config);

        Self::extend_instance_ttl(env);
        Ok(())
    }

    fn get_config(env: &Env) -> Result<Config, Error> {
        env.storage().instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)
    }

    /// #116: Pick the next resolver round-robin from the managed set, falling
    /// back to `config.dispute_resolver` when the set is empty.
    fn assign_resolver(env: &Env, config: &Config) -> Result<Address, Error> {
        let resolvers: Vec<Address> = env.storage().instance()
            .get(&DataKey::Resolvers)
            .unwrap_or(Vec::new(env));
        if resolvers.is_empty() {
            return Ok(config.dispute_resolver.clone());
        }
        let counter: u64 = env.storage().instance()
            .get(&DataKey::ResolverCounter)
            .unwrap_or(0);
        let idx = (counter % resolvers.len() as u64) as u32;
        env.storage().instance().set(&DataKey::ResolverCounter, &(counter + 1));
        resolvers.get(idx).ok_or(Error::ResolverNotFound)
    }

    fn ensure_not_paused(env: &Env) -> Result<(), Error> {
        let paused: bool = env.storage().instance().get(&DataKey::IsPaused).unwrap_or(false);
        if paused {
            return Err(Error::ContractPaused);
        }
        Ok(())
    }

    fn is_denylisted(env: &Env, address: &Address) -> bool {
        let denylist: Map<Address, bool> = env.storage().instance()
            .get(&DataKey::Denylist)
            .unwrap_or(Map::new(env));
        denylist.get(address.clone()).unwrap_or(false)
    }

    fn ensure_not_denylisted(env: &Env, address: &Address) -> Result<(), Error> {
        if Self::is_denylisted(env, address) {
            return Err(Error::AddressBlocked);
        }
        Ok(())
    }

    fn get_agent(env: &Env, agent_id: u64) -> Result<Agent, Error> {
        Self::extend_instance_ttl(env);
        env.storage().instance()
            .get(&DataKey::Agent(agent_id))
            .ok_or(Error::AgentNotFound)
    }

    fn get_job(env: &Env, job_id: u64) -> Result<Job, Error> {
        Self::extend_instance_ttl(env);
        env.storage().instance()
            .get(&DataKey::Job(job_id))
            .ok_or(Error::JobNotFound)
    }

    fn get_bid(env: &Env, bid_id: u64) -> Result<Bid, Error> {
        env.storage().instance()
            .get(&DataKey::Bid(bid_id))
            .ok_or(Error::BidNotFound)
    }

    fn get_escrow(env: &Env, escrow_id: u64) -> Result<Escrow, Error> {
        env.storage().instance()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)
    }

    fn get_dispute(env: &Env, dispute_id: u64) -> Result<Dispute, Error> {
        env.storage().instance()
            .get(&DataKey::Dispute(dispute_id))
            .ok_or(Error::DisputeNotFound)
    }

    fn get_review(env: &Env, review_id: u64) -> Result<Review, Error> {
        env.storage().instance()
            .get(&DataKey::Review(review_id))
            .ok_or(Error::ReviewNotFound)
    }

    fn get_platform_stats(env: &Env) -> Result<PlatformStats, Error> {
        env.storage().instance()
            .get(&DataKey::PlatformStats)
            .ok_or(Error::NotInitialized)
    }

    /// #121: Read a client's stats, defaulting to zeroed stats when absent.
    fn get_client_stats_internal(env: &Env, client: &Address) -> ClientStats {
        env.storage().instance()
            .get(&DataKey::ClientStats(client.clone()))
            .unwrap_or(ClientStats {
                rating_sum: 0,
                rating_count: 0,
                jobs_posted: 0,
                disputes: 0,
            })
    }

    fn next_agent_id(env: &Env) -> Result<u64, Error> {
        let id: u64 = env.storage().instance()
            .get(&DataKey::AgentCounter)
            .unwrap_or(0);
        let new_id = id + 1;
        env.storage().instance().set(&DataKey::AgentCounter, &new_id);
        Ok(new_id)
    }

    fn next_job_id(env: &Env) -> Result<u64, Error> {
        let id: u64 = env.storage().instance()
            .get(&DataKey::JobCounter)
            .unwrap_or(0);
        let new_id = id + 1;
        env.storage().instance().set(&DataKey::JobCounter, &new_id);
        Ok(new_id)
    }

    fn next_bid_id(env: &Env) -> Result<u64, Error> {
        let id: u64 = env.storage().instance()
            .get(&DataKey::BidCounter)
            .unwrap_or(0);
        let new_id = id + 1;
        env.storage().instance().set(&DataKey::BidCounter, &new_id);
        Ok(new_id)
    }

    fn next_review_id(env: &Env) -> Result<u64, Error> {
        let id: u64 = env.storage().instance()
            .get(&DataKey::ReviewCounter)
            .unwrap_or(0);
        let new_id = id + 1;
        env.storage().instance().set(&DataKey::ReviewCounter, &new_id);
        Ok(new_id)
    }

    fn next_dispute_id(env: &Env) -> Result<u64, Error> {
        let id: u64 = env.storage().instance()
            .get(&DataKey::DisputeCounter)
            .unwrap_or(0);
        let new_id = id + 1;
        env.storage().instance().set(&DataKey::DisputeCounter, &new_id);
        Ok(new_id)
    }

    fn next_escrow_id(env: &Env) -> Result<u64, Error> {
        let id: u64 = env.storage().instance()
            .get(&DataKey::EscrowCounter)
            .unwrap_or(0);
        let new_id = id + 1;
        env.storage().instance().set(&DataKey::EscrowCounter, &new_id);
        Ok(new_id)
    }

    fn next_verification_id(env: &Env) -> Result<u64, Error> {
        let id: u64 = env.storage().instance()
            .get(&DataKey::VerificationCounter)
            .unwrap_or(0);
        let new_id = id + 1;
        env.storage().instance().set(&DataKey::VerificationCounter, &new_id);
        Ok(new_id)
    }
}

#[cfg(test)]
mod test;

#[cfg(test)]
mod budget_test;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod negative_approve_deliverable_test;
