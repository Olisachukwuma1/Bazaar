import { Horizon } from '@stellar/stellar-sdk';
import { networkConfig } from '../config/network';

export type PreflightResult =
  | { ok: true; available: bigint }
  | { ok: false; code: 'NO_TRUSTLINE' | 'INSUFFICIENT'; required: bigint; available: bigint };

const USDC_ISSUER_MAINNET = 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';
const USDC_ISSUER_TESTNET = 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5';

export async function preflightUsdcFunding(
  publicKey: string,
  escrowAmount: bigint,
  feeBps = 100,
): Promise<PreflightResult> {
  const horizon =
    networkConfig.network === 'mainnet'
      ? 'https://horizon.stellar.org'
      : 'https://horizon-testnet.stellar.org';
  const server = new Horizon.Server(horizon);
  const account = await server.loadAccount(publicKey);
  const issuer = networkConfig.network === 'mainnet' ? USDC_ISSUER_MAINNET : USDC_ISSUER_TESTNET;
  const line = account.balances.find(
    (b) =>
      b.asset_type === 'credit_alphanum4' &&
      b.asset_code === 'USDC' &&
      b.asset_issuer === issuer,
  );
  if (!line) {
    return { ok: false, code: 'NO_TRUSTLINE', required: escrowAmount, available: 0n };
  }
  const available = BigInt(Math.floor(parseFloat(line.balance) * 10_000_000));
  const fee = (escrowAmount * BigInt(feeBps)) / 10_000n;
  const required = escrowAmount + fee;
  if (available < required) {
    return { ok: false, code: 'INSUFFICIENT', required, available };
  }
  return { ok: true, available };
}