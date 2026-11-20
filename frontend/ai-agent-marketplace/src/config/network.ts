export type StellarNetwork = 'testnet' | 'mainnet';

export interface NetworkConfig {
  network: StellarNetwork;
  rpcUrl: string;
  networkPassphrase: string;
  usdcContractId: string;
  label: string;
}

const CANONICAL: Record<StellarNetwork, Omit<NetworkConfig, 'network'>> = {
  testnet: {
    rpcUrl: 'https://soroban-testnet.stellar.org',
    networkPassphrase: 'Test SDF Network ; September 2015',
    usdcContractId: 'CBIELSO6KZLFBUSGZ47QRYCYMHK4D2H5IMKBORJGUJVAVKZOQK4D7IAJ',
    label: 'Testnet',
  },
  mainnet: {
    rpcUrl: 'https://soroban-mainnet.stellar.org',
    networkPassphrase: 'Public Global Stellar Network ; September 2015',
    usdcContractId: 'CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75',
    label: 'Mainnet',
  },
};

export function loadNetworkConfig(): NetworkConfig {
  const raw = (import.meta.env.VITE_STELLAR_NETWORK ?? 'testnet').toLowerCase();
  const network: StellarNetwork = raw === 'mainnet' ? 'mainnet' : 'testnet';
  const base = CANONICAL[network];
  return { network, ...base };
}

export const networkConfig = loadNetworkConfig();