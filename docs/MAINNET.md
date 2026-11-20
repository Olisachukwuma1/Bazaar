# Mainnet launch roadmap

Phased rollout for real-funds USDC on the AI Agent Marketplace.

## Phase 0 — Audit & threat review

**Owners:** security + contract leads  
**Exit criteria**

- [ ] External audit complete with no open critical findings
- [ ] [`docs/threat-model.md`](../contracts/../docs/threat-model.md) and [`THREAT_MODEL.md`](./THREAT_MODEL.md) reviewed
- [ ] Issues labelled `mainnet` (#155–#164, #176) merged or tracked

## Phase 1 — Canary (capped exposure)

**Owners:** ops + admin multisig (#164)  
**Exit criteria**

- [ ] `max_job_budget` and `max_total_escrow` set to conservative caps
- [ ] Treasury separate from admin (#163)
- [ ] Network badge + wallet network checks enabled (#156–#158)
- [ ] USDC token verified on deploy (#160)

## Phase 2 — Public launch

**Owners:** product + ops  
**Exit criteria**

- [ ] `SECURITY.md` published and contact monitored (#176)
- [ ] Fee cap governance documented (#162)
- [ ] Monitoring and escrow reconciliation jobs green for 30 days
- [ ] Go/no-go checklist signed by admin multisig

## Mainnet issue checklist

| Issue | Topic |
|-------|--------|
| #155 | This roadmap |
| #156 | Per-network frontend config |
| #157 | Wallet network verification |
| #158 | Network badge / real-funds warning |
| #159 | USDC trustline preflight |
| #160 | Canonical USDC on init |
| #161 | Decimal-aware formatting |
| #162 | Configurable platform fee cap |
| #163 | Treasury address |
| #164 | Admin multisig |
| #176 | Security disclosure policy |