# Security policy

## Contact

Email: security@bazaar-market.example (monitored by the core maintainers).

## Scope

- `contracts/ai-agent-marketplace` deployed instances
- `frontend/ai-agent-marketplace` payment and wallet flows
- `scripts/init-contract.sh` and deploy tooling affecting mainnet funds

Out of scope: third-party wallets, phishing, social engineering.

## Response SLA

| Severity | First response | Fix target |
|----------|----------------|------------|
| Critical (fund loss) | 24h | 7 days |
| High | 48h | 30 days |
| Medium / Low | 5 business days | best effort |

## Bounty tiers (indicative)

| Tier | Example |
|------|---------|
| **Critical** | Unauthorised escrow drain |
| **High** | Fee redirection bypassing treasury |
| **Medium** | Denylist or auth bypass without fund loss |
| **Low** | Informational / gas griefing |

Report responsible disclosures via the contact above. Do not post exploits publicly before coordination.