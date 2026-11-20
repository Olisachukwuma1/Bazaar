export type StellarNetwork = 'testnet' | 'mainnet';

export interface NetworkConfig {
  network: StellarNetwork;
  rpcUrl: string;
  networkPassphrase: string;
  usdcContractId: string;
}

const CANONICAL: Record<StellarNetwork, Omit<NetworkConfig, 'network'>> = {
  testnet: {
    rpcUrl: 'https://soroban-testnet.stellar.org',
    networkPassphrase: 'Test SDF Network ; September 2015',
    usdcContractId: 'CBIELSO6KZLFBUSGZ47QRYCYMHK4D2H5IMKBORJGUJVAVKZOQK4D7IAJ',
  },
  mainnet: {
    rpcUrl: 'https://soroban-mainnet.stellar.org',
    networkPassphrase: 'Public Global Stellar Network ; September 2015',
    usdcContractId: 'CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75',
  },
};

function assertMatch(label: string, expected: string, actual: string | undefined) {
  if (actual && actual !== expected) {
    throw new Error(
      `${label} (${actual}) does not match selected network (expected ${expected}). ` +
        'Clear overrides or fix VITE_STELLAR_NETWORK.',
    );
  }
}

/** Resolve network from VITE_STELLAR_NETWORK with optional explicit overrides (must agree). */
export function loadNetworkConfig(): NetworkConfig {
  const raw = (import.meta.env.VITE_STELLAR_NETWORK ?? 'testnet').toLowerCase();
  const network: StellarNetwork = raw === 'mainnet' ? 'mainnet' : 'testnet';
  const base = CANONICAL[network];
  assertMatch('VITE_RPC_URL', base.rpcUrl, import.meta.env.VITE_RPC_URL);
  assertMatch('VITE_NETWORK_PASSPHRASE', base.networkPassphrase, import.meta.env.VITE_NETWORK_PASSPHRASE);
  assertMatch('VITE_USDC_CONTRACT_ID', base.usdcContractId, import.meta.env.VITE_USDC_CONTRACT_ID);
  return {
    network,
    rpcUrl: import.meta.env.VITE_RPC_URL ?? base.rpcUrl,
    networkPassphrase: import.meta.env.VITE_NETWORK_PASSPHRASE ?? base.networkPassphrase,
    usdcContractId: import.meta.env.VITE_USDC_CONTRACT_ID ?? base.usdcContractId,
  };
}

export const networkConfig = loadNetworkConfig();