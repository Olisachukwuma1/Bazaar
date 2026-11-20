import React, { createContext, useContext, useState, useEffect, useCallback, useMemo } from 'react';
import {
  Contract,
  SorobanRpc,
  Networks,
  TransactionBuilder,
  Transaction,
  Account,
  BASE_FEE,
  Address,
  xdr,
  nativeToScVal,
  scValToNative,
  Keypair,
  StrKey,
  Account,
} from '@stellar/stellar-sdk';
import {
  MarketplaceClient,
  describeSimulationError,
  formatTokenAmount,
} from '@/lib/marketplaceClient';
import { tryLoadEnvConfig } from '@/config/env';

export interface ContractConfig {
  contractId: string;
  rpcUrl: string;
  networkPassphrase: string;
}

export interface StellarContextType {
  contract: Contract | null;
  rpc: SorobanRpc.Server | null;
  config: ContractConfig | null;
  isConnected: boolean;
  connect: (config: ContractConfig) => Promise<void>;
  disconnect: () => void;
  simulateTransaction: (tx: TransactionBuilder) => Promise<SorobanRpc.Api.SimulateTransactionResponse>;
  sendTransaction: (
    tx: TransactionBuilder,
    signFn: (tx: string) => Promise<string>,
  ) => Promise<SorobanRpc.Api.SendTransactionResponse>;
  getContractData: (key: xdr.ScVal, durability?: 'persistent' | 'temporary') => Promise<xdr.ScVal | null>;
  invokeContract: (method: string, ...args: xdr.ScVal[]) => Promise<xdr.ScVal | null>;
  /**
   * Build, sign and submit a real Soroban transaction invoking `method`, then
   * poll until it lands. `sourcePublicKey` must be the account requiring auth
   * for this call (the contract methods here all authorize the transaction's
   * own source account, so no separate auth-entry signing step is needed).
   */
  invokeContractWrite: (
    method: string,
    args: xdr.ScVal[],
    sourcePublicKey: string,
    signTransaction: (xdr: string) => Promise<string>,
  ) => Promise<xdr.ScVal | null>;
  getContractClient: () => MarketplaceClient | null;
}

const StellarContext = createContext<StellarContextType | null>(null);

const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]', '::1']);

/**
 * Decide whether an RPC URL may be used insecurely (plain HTTP).
 * HTTP is only tolerated in development builds or for loopback hosts;
 * production builds must use HTTPS.
 */
export function resolveAllowHttp(
  rpcUrl: string,
  isDev: boolean = Boolean(import.meta.env.DEV),
): boolean {
  let url: URL;
  try {
    url = new URL(rpcUrl);
  } catch {
    throw new Error(`Invalid RPC URL: ${rpcUrl}`);
  }

  if (url.protocol === 'https:') return false;

  if (url.protocol !== 'http:') {
    throw new Error(`Unsupported RPC URL protocol "${url.protocol}" in ${rpcUrl}. Use https://.`);
  }

  if (isDev || LOCAL_HOSTS.has(url.hostname)) return true;

  throw new Error(
    `Insecure RPC URL "${rpcUrl}" is not allowed in production. Use an https:// endpoint.`,
  );
}

export const useStellar = () => {
  const context = useContext(StellarContext);
  if (!context) throw new Error('useStellar must be used within StellarProvider');
  return context;
};

export const StellarProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [contract, setContract] = useState<Contract | null>(null);
  const [rpc, setRpc] = useState<SorobanRpc.Server | null>(null);
  const [config, setConfig] = useState<ContractConfig | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  const connect = useCallback(async (newConfig: ContractConfig) => {
    try {
      const rpcServer = new SorobanRpc.Server(newConfig.rpcUrl, {
        allowHttp: resolveAllowHttp(newConfig.rpcUrl),
      });
      const contractInstance = new Contract(newConfig.contractId);
      
      await rpcServer.getLatestLedger();
      
      setRpc(rpcServer);
      setContract(contractInstance);
      setConfig(newConfig);
      setIsConnected(true);
      
      localStorage.setItem('stellar-contract-config', JSON.stringify(newConfig));
    } catch (error) {
      console.error('Failed to connect to Stellar:', error);
      throw error;
    }
  }, []);

  const disconnect = useCallback(() => {
    setContract(null);
    setRpc(null);
    setConfig(null);
    setIsConnected(false);
    localStorage.removeItem('stellar-contract-config');
  }, []);

  const simulateTransaction = useCallback(async (tx: TransactionBuilder) => {
    if (!rpc) throw new Error('RPC not connected');
    return rpc.simulateTransaction(tx.build());
  }, [rpc]);

  const sendTransaction = useCallback(async (
    tx: TransactionBuilder, 
    signFn: (tx: string) => Promise<string>
  ) => {
    if (!rpc || !config) throw new Error('Not connected');
    
    const builtTx = tx.build();
    const signedTxXdr = await signFn(builtTx.toXDR());
    const signedTx = TransactionBuilder.fromXDR(signedTxXdr, config.networkPassphrase);
    
    return rpc.sendTransaction(signedTx);
  }, [rpc, config]);

  const getContractData = useCallback(async (
    key: xdr.ScVal, 
    durability: 'persistent' | 'temporary' = 'persistent'
  ) => {
    if (!rpc || !config) throw new Error('Not connected');
    
    try {
      const result = await rpc.getContractData(
        config.contractId,
        key,
        durability === 'temporary'
          ? SorobanRpc.Durability.Temporary
          : SorobanRpc.Durability.Persistent,
      );
      return result?.val ?? null;
    } catch (error) {
      if ((error as Error).message.includes('not found')) return null;
      throw error;
    }
  }, [rpc, config]);

  const invokeContract = useCallback(async (method: string, ...args: xdr.ScVal[]) => {
    if (!contract || !rpc || !config) throw new Error('Not connected');
    
    const account = new Account('GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF', '0');
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: config.networkPassphrase,
    })
      .addOperation(contract.call(method, ...args))
      .setTimeout(300)
      .build();
    
    const simResult = await rpc.simulateTransaction(tx);
    
    if (SorobanRpc.Api.isSimulationError(simResult)) {
      throw new Error(describeSimulationError(simResult.error));
    }
    
    return simResult.result?.retval ?? null;
  }, [contract, rpc, config]);

  // Build, sign and submit a real transaction (as opposed to `invokeContract`,
  // which only simulates). Used for every mutation — posting jobs, bidding,
  // accepting bids, etc.
  const invokeContractWrite = useCallback(async (
    method: string,
    args: xdr.ScVal[],
    sourcePublicKey: string,
    signTransaction: (xdr: string) => Promise<string>,
  ): Promise<xdr.ScVal | null> => {
    if (!contract || !rpc || !config) throw new Error('Not connected to the marketplace contract.');

    const sourceAccount = await rpc.getAccount(sourcePublicKey);
    const tx = new TransactionBuilder(sourceAccount, {
      fee: BASE_FEE,
      networkPassphrase: config.networkPassphrase,
    })
      .addOperation(contract.call(method, ...args))
      .setTimeout(60)
      .build();

    // Always simulate first: a failed simulation blocks signing, and a
    // successful one gives us the footprint/auth/resource fees to attach.
    const simResult = await rpc.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(simResult)) {
      throw new Error(describeSimulationError(simResult.error));
    }
    const prepared = SorobanRpc.assembleTransaction(tx, simResult).build();

    // Show the estimated network fee (and the USDC amount, if the call takes
    // one) before handing the transaction to the wallet for signing.
    const feeLines = [`Estimated network fee: ${formatTokenAmount(prepared.fee, 7)} XLM`];
    const usdcArg = args.find((arg) => arg.switch().name === 'scvI128');
    if (usdcArg) {
      feeLines.push(`USDC amount: ${formatTokenAmount(scValToNative(usdcArg), 7)} USDC`);
    }
    if (!window.confirm(`${feeLines.join('\n')}\n\nSign and submit this transaction?`)) {
      throw new Error('Transaction cancelled.');
    }

    const signedXdr = await signTransaction(prepared.toXDR());
    const signedTx = TransactionBuilder.fromXDR(signedXdr, config.networkPassphrase) as Transaction;

    // #186: when a fee sponsor is configured, the platform pays the network
    // fee via a fee-bump transaction. Fall back to a user-paid submission if
    // the sponsor declines (disabled, rate limited, budget exhausted).
    let hash = await submitSponsored(signedXdr);
    if (!hash) {
      const sendResult = await rpc.sendTransaction(signedTx);
      if (sendResult.status === 'ERROR') {
        throw new Error('Failed to submit transaction to the network.');
      }
      hash = sendResult.hash;
    }

    // #184: track the hash so its status survives a page refresh.
    const tracker = useTxTracker.getState();
    tracker.track({
      hash,
      label: method,
      status: 'pending',
      networkPassphrase: config.networkPassphrase,
      submittedAt: Date.now(),
    });

    let txResult = await rpc.getTransaction(hash);
    const deadline = Date.now() + 30000;
    while (txResult.status === SorobanRpc.Api.GetTransactionStatus.NOT_FOUND) {
      if (Date.now() > deadline) {
        throw new Error('Timed out waiting for transaction confirmation.');
      }
      await new Promise((resolve) => setTimeout(resolve, 1500));
      txResult = await rpc.getTransaction(hash);
    }

    if (txResult.status !== SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
      tracker.setStatus(hash, 'failed');
      throw new Error(`Transaction failed with status: ${txResult.status}`);
    }
    tracker.setStatus(hash, 'success');

    return (txResult as SorobanRpc.Api.GetSuccessfulTransactionResponse).returnValue ?? null;
  }, [contract, rpc, config]);

  // #184: resume polling transactions that were still pending when the page
  // was last closed or refreshed.
  useEffect(() => {
    if (!rpc || !config) return;
    let cancelled = false;
    const { txs, setStatus } = useTxTracker.getState();
    const pending = txs.filter(
      (tx) => tx.status === 'pending' && tx.networkPassphrase === config.networkPassphrase,
    );

    pending.forEach(async (tx) => {
      while (!cancelled) {
        try {
          const result = await rpc.getTransaction(tx.hash);
          if (result.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
            setStatus(tx.hash, 'success');
            return;
          }
          if (result.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
            setStatus(tx.hash, 'failed');
            return;
          }
        } catch {
          // Transient RPC error; retry on the next tick.
        }
        if (Date.now() - tx.submittedAt > TX_EXPIRY_MS) {
          setStatus(tx.hash, 'failed');
          return;
        }
        await new Promise((resolve) => setTimeout(resolve, 3000));
      }
    });

    return () => {
      cancelled = true;
    };
  }, [rpc, config]);

  // Typed contract client built from the current `invokeContract`. Returns
  // null while not connected so pages can show a "connect" prompt instead of
  // a generic fetch error.
  const contractClient = useMemo<MarketplaceClient | null>(() => {
    // E2E test seam: the Playwright harness injects a fixture-backed contract
    // client via `window.__E2E_MARKETPLACE_CLIENT__` (see
    // frontend/ai-agent-marketplace/e2e/fixtures.ts) so the marketplace can be
    // exercised in CI without a live Soroban RPC. The global is only ever set
    // by the e2e setup and is never present in production or normal
    // development, so this branch is inert outside of `pnpm test:e2e`.
    if (typeof window !== 'undefined') {
      const injected = (window as Window & {
        __E2E_MARKETPLACE_CLIENT__?: unknown;
      }).__E2E_MARKETPLACE_CLIENT__;
      if (injected) return injected as MarketplaceClient;
    }
    return isConnected && invokeContract
      ? new MarketplaceClient(invokeContract, invokeContractWrite)
      : null;
  }, [isConnected, invokeContract, invokeContractWrite]);

  useEffect(() => {
    const saved = localStorage.getItem('stellar-contract-config');
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        connect(parsed).catch(() => localStorage.removeItem('stellar-contract-config'));
        return;
      } catch {
        localStorage.removeItem('stellar-contract-config');
      }
    }

    // No saved config — fall back to the validated build-time env config (see
    // src/config/env.ts and .env.example) so the marketplace connects
    // automatically rather than requiring a manual "connect" step that the UI
    // never actually offers. Invalid env is surfaced by <EnvErrorScreen />.
    const { config: envConfig } = tryLoadEnvConfig();
    if (envConfig) {
      connect({
        contractId: envConfig.VITE_CONTRACT_ID,
        rpcUrl: envConfig.VITE_RPC_URL,
        networkPassphrase: envConfig.VITE_NETWORK_PASSPHRASE,
      }).catch((err) => console.error('Failed to auto-connect using env contract config:', err));
    }
  }, [connect]);

  return (
    <StellarContext.Provider value={{
      contract,
      rpc,
      config,
      isConnected,
      connect,
      disconnect,
      simulateTransaction,
      sendTransaction,
      getContractData,
      invokeContract,
      invokeContractWrite,
      getContractClient: () => contractClient,
    }}>
      {children}
    </StellarContext.Provider>
  );
};

/**
 * #186: Submit a signed transaction through the fee sponsor (see
 * api/fee-bump.ts). Returns the fee-bump transaction hash, or null when no
 * sponsor is configured or it declined the transaction.
 */
const submitSponsored = async (signedXdr: string): Promise<string | null> => {
  const sponsorUrl = import.meta.env.VITE_FEE_SPONSOR_URL;
  if (!sponsorUrl) return null;
  try {
    const response = await fetch(sponsorUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ xdr: signedXdr }),
    });
    const body = await response.json().catch(() => ({}));
    if (!response.ok || typeof body.hash !== 'string') {
      console.warn('Fee sponsor declined transaction:', body.error ?? response.status);
      return null;
    }
    return body.hash;
  } catch (error) {
    console.warn('Fee sponsor unavailable:', error);
    return null;
  }
};

// Type conversion helpers
export const parseScVal = <T extends unknown>(val: xdr.ScVal): T => {
  return scValToNative(val) as T;
};

export const addressToScVal = (address: string): xdr.ScVal => {
  return nativeToScVal(address, { type: 'address' });
};

export const u64ToScVal = (val: number | bigint): xdr.ScVal => {
  return nativeToScVal(val, { type: 'u64' });
};

export const i128ToScVal = (val: number | bigint | string): xdr.ScVal => {
  return nativeToScVal(val, { type: 'i128' });
};

export const stringToScVal = (val: string): xdr.ScVal => {
  return nativeToScVal(val, { type: 'string' });
};

export const vecToScVal = (arr: unknown[], typeStr: string): xdr.ScVal => {
  return nativeToScVal(arr, { type: `vec<${typeStr}>` as any });
};

export const mapToScVal = (entries: [unknown, unknown][], keyType: string, valType: string): xdr.ScVal => {
  return nativeToScVal(entries, { type: `map<${keyType},${valType}>` as any });
};

export const optionToScVal = (val: unknown | null, typeStr: string): xdr.ScVal => {
  return nativeToScVal(val ?? null, { type: `option<${typeStr}>` as any });
};

export const bytesN32ToScVal = (val: Buffer | Uint8Array): xdr.ScVal => {
  return nativeToScVal(val, { type: 'bytes<32>' as any });
};