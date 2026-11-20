import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import * as freighter from '@stellar/freighter-api';
import { Horizon } from '@stellar/stellar-sdk';
import { networkConfig } from '../config/network';

export type NetworkType = string;

// Circle USDC issuers — keep these as constants so they're easy to audit
const USDC_ISSUER_TESTNET = 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5';
const USDC_ISSUER_MAINNET = 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';

const getHorizonUrl = (network: string | null) =>
  network === 'PUBLIC'
    ? 'https://horizon.stellar.org'
    : 'https://horizon-testnet.stellar.org';

// ---------------------------------------------------------------------------
// Compliance: sanctions screening & region restrictions
// ---------------------------------------------------------------------------
// The blocklist is configurable at build time via environment variables so it
// can be updated without a code change:
//   VITE_BLOCKED_ADDRESSES  comma-separated Stellar public keys (G...)
//   VITE_BLOCKED_REGIONS    comma-separated ISO 3166-1 alpha-2 region codes
//                           (e.g. "KP,IR,SY")
// Addresses are compared case-insensitively; regions are upper-cased.
// ---------------------------------------------------------------------------

const parseList = (raw: string | undefined | null): string[] =>
  (raw ?? '')
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean);

const BLOCKED_ADDRESSES: string[] = parseList(
  import.meta.env?.VITE_BLOCKED_ADDRESSES,
).map((address) => address.toUpperCase());

const BLOCKED_REGIONS: string[] = parseList(
  import.meta.env?.VITE_BLOCKED_REGIONS,
).map((region) => region.toUpperCase());

/** Returns true when the given Stellar address is on the compliance blocklist. */
export const isAddressBlocked = (address: string | null | undefined): boolean => {
  if (!address) return false;
  return BLOCKED_ADDRESSES.includes(address.trim().toUpperCase());
};

/** Returns true when the given region code is restricted. */
export const isRegionBlocked = (region: string | null | undefined): boolean => {
  if (!region) return false;
  return BLOCKED_REGIONS.includes(region.trim().toUpperCase());
};

/** The configured blocklists, exposed for UI display / debugging. */
export const getComplianceLists = () => ({
  blockedAddresses: [...BLOCKED_ADDRESSES],
  blockedRegions: [...BLOCKED_REGIONS],
});

/** #157: Block signing when the wallet passphrase disagrees with the app build. */
export class WalletNetworkMismatchError extends Error {
  constructor(public readonly expected: string, public readonly actual: string) {
    super(`Switch your wallet to ${expected.includes('Public') ? 'Mainnet' : 'Testnet'} (app expects this network).`);
    this.name = 'WalletNetworkMismatchError';
  }
}

export async function assertWalletNetworkMatchesApp(): Promise<void> {
  const walletPassphrase = await freighter.getNetworkDetails().then((d) => d.networkPassphrase);
  if (walletPassphrase && walletPassphrase !== networkConfig.networkPassphrase) {
    throw new WalletNetworkMismatchError(networkConfig.networkPassphrase, walletPassphrase);
  }
}

// ---------------------------------------------------------------------------
// Terms of Service / Privacy Policy acceptance
// ---------------------------------------------------------------------------
// Bump TERMS_VERSION whenever the terms change so users are asked to accept
// them again on their next wallet connect. Acceptance is recorded per address
// in localStorage under `terms-accepted:<publicKey>`.
// ---------------------------------------------------------------------------

export const TERMS_VERSION = '2026-11-19';

const termsKey = (publicKey: string) => `terms-accepted:${publicKey}`;

const hasAcceptedTerms = (publicKey: string): boolean => {
  try {
    const saved = JSON.parse(localStorage.getItem(termsKey(publicKey)) ?? 'null');
    return saved?.version === TERMS_VERSION;
  } catch {
    return false;
  }
};

const recordTermsAcceptance = (publicKey: string) => {
  localStorage.setItem(
    termsKey(publicKey),
    JSON.stringify({ version: TERMS_VERSION, acceptedAt: new Date().toISOString() }),
  );
};

interface Balances {
  /** Native XLM balance as a decimal string, e.g. "9.9999900" */
  xlmBalance: string;
  /** USDC balance as a decimal string, or "0" when no trustline exists */
  usdcBalance: string;
}

/**
 * Fetch both the native XLM balance and the Circle USDC trustline balance
 * for a given account.  Returns "0" for whichever asset is absent rather
 * than throwing, so callers never need to guard against undefined.
 */
const fetchBalances = async (
  publicKey: string,
  network: string | null,
): Promise<Balances> => {
  try {
    const server = new Horizon.Server(getHorizonUrl(network));
    const account = await server.loadAccount(publicKey);

    const native = account.balances.find((b) => b.asset_type === 'native');
    const xlmBalance = native?.balance ?? '0';

    const usdcIssuer =
      network === 'PUBLIC' ? USDC_ISSUER_MAINNET : USDC_ISSUER_TESTNET;

    const usdc = account.balances.find(
      (b) =>
        b.asset_type === 'credit_alphanum4' &&
        b.asset_code === 'USDC' &&
        b.asset_issuer === usdcIssuer,
    );
    const usdcBalance = usdc?.balance ?? '0';

    return { xlmBalance, usdcBalance };
  } catch {
    return { xlmBalance: '0', usdcBalance: '0' };
  }
};

interface WalletState {
  publicKey: string | null;
  network: string | null;
  isConnected: boolean;
  isConnecting: boolean;
  /** Native XLM balance */
  xlmBalance: string;
  /** Circle USDC balance (zero when no trustline exists) */
  usdcBalance: string;
  isLoading: boolean;
  availableNetworks: string[];
  /** True when the connected wallet address is on the compliance blocklist */
  isBlocked: boolean;
  /** Human-readable reason when the wallet is blocked, otherwise null */
  blockedReason: string | null;
}

interface WalletContextType extends WalletState {
  connect: (network?: string) => Promise<void>;
  disconnect: () => void;
  signTransaction: (xdr: string, opts?: any) => Promise<string>;
  signAuthEntry: (xdr: string, address: string, networkPassphrase: string) => Promise<string>;
  switchNetwork: (network: string) => Promise<void>;
  refreshBalance: () => Promise<void>;
  isFreighterInstalled: boolean;
  /** Compliance helpers usable before posting or bidding */
  isAddressBlocked: (address: string | null | undefined) => boolean;
  isRegionBlocked: (region: string | null | undefined) => boolean;
}

const WalletContext = createContext<WalletContextType | null>(null);

export const useWallet = () => {
  const context = useContext(WalletContext);
  if (!context) throw new Error('useWallet must be used within WalletProvider');
  return context;
};

const INITIAL_STATE: WalletState = {
  publicKey: null,
  network: 'TESTNET',
  isConnected: false,
  isConnecting: false,
  xlmBalance: '0',
  usdcBalance: '0',
  isLoading: false,
  availableNetworks: ['TESTNET', 'PUBLIC'],
  isBlocked: false,
  blockedReason: null,
};

export const WalletProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [state, setState] = useState<WalletState>(INITIAL_STATE);
  const [isFreighterInstalled, setIsFreighterInstalled] = useState(false);

  const checkConnection = useCallback(async () => {
    try {
      const isConnected = await freighter.isConnected();
      setIsFreighterInstalled(Boolean(isConnected));
      if (isConnected) {
        const publicKey = await freighter.getPublicKey();
        if (publicKey) {
          const blocked = isAddressBlocked(publicKey);
          setState((prev) => ({
            ...prev,
            publicKey,
            isConnected: true,
            isBlocked: blocked,
            blockedReason: blocked
              ? 'This wallet address is restricted by compliance policy.'
              : null,
          }));
        }
      }
    } catch (err) {
      console.warn('Freighter connection check error:', err);
    }
  }, []);

  useEffect(() => {
    checkConnection();
  }, [checkConnection]);

  const connect = async () => {
    setState((prev) => ({ ...prev, isConnecting: true }));
    try {
      const publicKey = await freighter.getPublicKey();
      if (!hasAcceptedTerms(publicKey)) {
        const accepted = window.confirm(
          'By connecting your wallet you agree to the AgentMarket Terms of Service ' +
          '(including the fee schedule) and Privacy Policy, available at /terms and /privacy.\n\n' +
          'Platform fee: 1% of the accepted bid. Dispute fee: 2% of escrow. ' +
          'Stellar network fees apply to every transaction.\n\nDo you accept?',
        );
        if (!accepted) {
          setState((prev) => ({ ...prev, isConnecting: false }));
          return;
        }
        recordTermsAcceptance(publicKey);
      }
      const { xlmBalance, usdcBalance } = await fetchBalances(publicKey, state.network);
      const blocked = isAddressBlocked(publicKey);
      setState((prev) => ({
        ...prev,
        publicKey,
        xlmBalance,
        usdcBalance,
        isConnected: true,
        isConnecting: false,
        isBlocked: blocked,
        blockedReason: blocked
          ? 'This wallet address is restricted by compliance policy.'
          : null,
      }));
    } catch (err) {
      console.error('Failed to connect Freighter wallet:', err);
      setState((prev) => ({ ...prev, isConnecting: false }));
    }
  };

  const disconnect = () => {
    setState(INITIAL_STATE);
  };

  const signTransaction = async (xdr: string, opts?: any): Promise<string> => {
    await assertWalletNetworkMatchesApp();
    const result = await freighter.signTransaction(xdr, opts);
    return result.signedTxXdr;
  };

  const signAuthEntry = async (
    xdr: string,
    address: string,
    networkPassphrase: string,
  ): Promise<string> => {
    const result = await freighter.signAuthEntry(xdr, {
      accountToSign: address,
      networkPassphrase,
    });
    return result.signedAuthEntry;
  };

  const switchNetwork = async (net: string) => {
    setState((prev) => ({ ...prev, network: net }));
  };

  const refreshBalance = async () => {
    if (!state.publicKey) return;
    setState((prev) => ({ ...prev, isLoading: true }));
    const { xlmBalance, usdcBalance } = await fetchBalances(
      state.publicKey,
      state.network,
    );
    setState((prev) => ({ ...prev, xlmBalance, usdcBalance, isLoading: false }));
  };

  return (
    <WalletContext.Provider
      value={{
        ...state,
        connect,
        disconnect,
        signTransaction,
        signAuthEntry,
        switchNetwork,
        refreshBalance,
        isFreighterInstalled,
        isAddressBlocked,
        isRegionBlocked,
      }}
    >
      {children}
    </WalletContext.Provider>
  );
};
