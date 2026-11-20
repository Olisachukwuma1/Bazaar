import { describe, expect, it } from 'vitest';
import { loadNetworkConfig } from './network';

describe('network config', () => {
  it('defaults to testnet canonical values', () => {
    const cfg = loadNetworkConfig();
    expect(['testnet', 'mainnet']).toContain(cfg.network);
    expect(cfg.rpcUrl).toContain('stellar');
    expect(cfg.usdcContractId.length).toBeGreaterThan(10);
  });
});