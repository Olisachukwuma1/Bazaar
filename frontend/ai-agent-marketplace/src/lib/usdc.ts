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