import { describe, expect, it } from 'vitest';
import { resolveAllowHttp } from './StellarContext';

describe('resolveAllowHttp', () => {
  it('never allows http for https URLs', () => {
    expect(resolveAllowHttp('https://soroban-testnet.stellar.org', false)).toBe(false);
    expect(resolveAllowHttp('https://soroban-testnet.stellar.org', true)).toBe(false);
  });

  it('allows http in development builds', () => {
    expect(resolveAllowHttp('http://rpc.example.com', true)).toBe(true);
  });

  it('allows http for loopback hosts outside development', () => {
    expect(resolveAllowHttp('http://localhost:8000', false)).toBe(true);
    expect(resolveAllowHttp('http://127.0.0.1:8000', false)).toBe(true);
  });

  it('rejects http for remote hosts in production builds', () => {
    expect(() => resolveAllowHttp('http://rpc.example.com', false)).toThrow(/not allowed in production/);
  });

  it('rejects unsupported protocols and malformed URLs', () => {
    expect(() => resolveAllowHttp('ftp://rpc.example.com', false)).toThrow(/Unsupported RPC URL protocol/);
    expect(() => resolveAllowHttp('not a url', false)).toThrow(/Invalid RPC URL/);
  });
});
