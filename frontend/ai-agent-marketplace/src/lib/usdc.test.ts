import { describe, expect, it } from 'vitest';
import { formatUsdc, parseUsdc } from './usdc';

describe('usdc helpers', () => {
  it('formats and parses 7-decimal amounts without float math', () => {
    const raw = parseUsdc('10.5');
    expect(formatUsdc(raw)).toBe('10.5');
    expect(parseUsdc(formatUsdc(1_234_567_890n))).toBe(1_234_567_890n);
  });
});