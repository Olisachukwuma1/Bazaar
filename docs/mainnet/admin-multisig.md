# Admin multisig rehearsal (#164)

## Goal

No single hot key may `set_paused`, `update_config`, `set_treasury`, or manage verifiers.

## Testnet procedure

1. Create a 2-of-3 multisig on testnet (`stellar account merge` / weighted signers).
2. Set `ADMIN_ADDRESS` to the multisig when running `scripts/init-contract.sh`.
3. Run pause → unpause, `update_config`, and `set_treasury` with two signer keys.
4. Document signer rotation in ops runbook (replace signer, bump threshold, revoke old key).

## Signer rotation runbook

1. Add new signer on multisig account via Horizon tx signed by incumbent quorum.
2. Remove compromised signer after new signer is confirmed in CI rehearsal.
3. Update GitHub Actions / deploy secrets to reference new `ADMIN_SOURCE` key only as one of N signers.

## Exit criteria

- Admin is a multisig on testnet rehearsal
- Runbook merged
- No single hot key can pause or reconfigure alone