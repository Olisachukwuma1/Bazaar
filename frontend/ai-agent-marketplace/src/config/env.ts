import { z } from 'zod';

/**
 * Build-time environment configuration.
 *
 * Parses and validates the VITE_* variables the app needs to talk to Soroban.
 * Used as the default contract config when no `stellar-contract-config` entry
 * exists in localStorage (see contexts/StellarContext.tsx).
 *
 * Validation is fail-fast: `loadEnvConfig()` throws an `EnvConfigError` with a
 * readable message listing every invalid/missing variable.
 */

const contractId = z
  .string()
  .trim()
  .regex(/^C[A-Z2-7]{55}$/, 'must be a Soroban contract ID (C…, 56 chars, base32)');

const rpcUrl = z
  .string()
  .trim()
  .url('must be a valid URL')
  .refine((value) => /^https?:\/\//.test(value), 'must use http(s)');

const networkPassphrase = z.string().trim().min(1, 'must not be empty');

export const envSchema = z.object({
  VITE_CONTRACT_ID: contractId,
  VITE_RPC_URL: rpcUrl,
  VITE_NETWORK_PASSPHRASE: networkPassphrase,
  VITE_USDC_CONTRACT_ID: contractId,
});

export type EnvConfig = z.infer<typeof envSchema>;

export class EnvConfigError extends Error {
  readonly issues: string[];

  constructor(issues: string[]) {
    super(
      `Invalid environment configuration:\n${issues.map((issue) => `  • ${issue}`).join('\n')}\n` +
        'Fix the VITE_* variables in your .env file (see .env.example) and reload.',
    );
    this.name = 'EnvConfigError';
    this.issues = issues;
  }
}

/** Parse a raw env bag (defaults to `import.meta.env`). Throws `EnvConfigError`. */
export function parseEnvConfig(raw: Record<string, unknown> = import.meta.env): EnvConfig {
  const result = envSchema.safeParse(raw);
  if (!result.success) {
    const issues = result.error.issues.map((issue) => {
      const key = issue.path.join('.') || '(root)';
      return `${key}: ${issue.message}`;
    });
    throw new EnvConfigError(issues);
  }
  return result.data;
}

let cached: EnvConfig | null = null;

/** Lazily parsed, cached env config. Throws `EnvConfigError` when invalid. */
export function loadEnvConfig(): EnvConfig {
  if (!cached) cached = parseEnvConfig();
  return cached;
}

/** Non-throwing variant for UI code that renders an error screen. */
export function tryLoadEnvConfig(): { config: EnvConfig | null; error: EnvConfigError | null } {
  try {
    return { config: loadEnvConfig(), error: null };
  } catch (error) {
    return {
      config: null,
      error:
        error instanceof EnvConfigError
          ? error
          : new EnvConfigError([error instanceof Error ? error.message : String(error)]),
    };
  }
}
