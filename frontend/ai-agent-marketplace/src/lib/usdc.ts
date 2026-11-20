/** #161: BigInt-safe USDC formatting (7 decimals by default). */

const TEN = 10n;

export function formatUsdc(amount: bigint, decimals = 7): string {
  const scale = TEN ** BigInt(decimals);
  const whole = amount / scale;
  const frac = (amount % scale).toString().padStart(decimals, '0');
  return `${whole}.${frac}`.replace(/\.?0+$/, (m) => (m.startsWith('.') ? m.replace(/0+$/, '') || '.0' : ''));
}

export function parseUsdc(input: string, decimals = 7): bigint {
  const [whole, frac = ''] = input.split('.');
  const fracPadded = (frac + '0'.repeat(decimals)).slice(0, decimals);
  return BigInt(whole || '0') * TEN ** BigInt(decimals) + BigInt(fracPadded || '0');
}

/** Strict decimal string -> stroops. Throws on malformed input or too many decimals. */
export function parseUnits(input: string, decimals = 7): bigint {
  const value = input.trim();
  if (!/^\d+(\.\d+)?$/.test(value)) {
    throw new Error(`Invalid USDC amount: "${input}"`);
  }
  const [, frac = ''] = value.split('.');
  if (frac.length > decimals) {
    throw new Error(`USDC amount supports at most ${decimals} decimal places`);
  }
  return parseUsdc(value, decimals);
}

/** Stroops -> decimal string with exactly `decimals` places (no float math). */
export function formatUnits(amount: bigint, decimals = 7): string {
  const negative = amount < 0n;
  const abs = negative ? -amount : amount;
  const scale = TEN ** BigInt(decimals);
  const whole = abs / scale;
  const frac = (abs % scale).toString().padStart(decimals, '0');
  return `${negative ? '-' : ''}${whole}.${frac}`;
}