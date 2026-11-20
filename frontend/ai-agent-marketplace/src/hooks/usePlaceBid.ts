import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useWallet } from '@/contexts/WalletContext';
import { useStellar } from '@/contexts/StellarContext';
import { isAddressBlocked } from '@/lib/compliance';
import { contractErrorMessage, describeSimulationError } from '@/lib/marketplaceClient';
import type { Bid } from '@/lib/types';

export interface PlaceBidParams {
  jobId: number;
  /** Amount in stroops (7-decimal USDC units). */
  amount: number | bigint;
  proposedTimelineDays: number;
  proposal: string;
}

/**
 * Map any thrown error from the bid flow to a human-readable message using the
 * contract error table (`contractErrorMessage` / `describeSimulationError`).
 */
export function bidErrorMessage(err: unknown): string {
  if (err instanceof Error) {
    const sim = describeSimulationError(err.message);
    if (!sim.startsWith('Simulation failed:')) return sim;
    return err.message;
  }
  if (typeof err === 'string') {
    const sim = describeSimulationError(err);
    return sim.startsWith('Simulation failed:') ? err : sim;
  }
  return contractErrorMessage('UnknownError');
}

/**
 * Shared `place_bid` mutation. Used by JobDetail and AgentProfile so both
 * surfaces sign, submit and invalidate the same way.
 */
export function usePlaceBid() {
  const { publicKey, signTransaction } = useWallet();
  const { getContractClient } = useStellar();
  const queryClient = useQueryClient();

  return useMutation<Bid, Error, PlaceBidParams>({
    mutationFn: async (params: PlaceBidParams) => {
      if (!publicKey) throw new Error('Connect your wallet to place a bid.');
      const client = getContractClient();
      if (!client) {
        throw new Error('Not connected to the marketplace contract. Configure it in Settings.');
      }
      // Compliance: re-check the blocklist right before submitting so a stale
      // cache cannot let a blocked address post a bid.
      if (await isAddressBlocked(publicKey, client)) {
        await queryClient.invalidateQueries({ queryKey: ['compliance-blocked', publicKey] });
        throw new Error(
          'Compliance error: your wallet address is blocked from participating in this marketplace. ' +
            'Bidding is not permitted. Contact support if you believe this is a mistake.',
        );
      }
      const bidId = await client.placeBid(publicKey, signTransaction, params);
      return client.getBid(bidId);
    },
    onSuccess: (_bid, params) => {
      queryClient.invalidateQueries({ queryKey: ['job', params.jobId] });
      queryClient.invalidateQueries({ queryKey: ['job-bids', params.jobId] });
      if (publicKey) queryClient.invalidateQueries({ queryKey: ['agent-bids', publicKey] });
    },
    onError: (err) => {
      console.error('Bid failed:', err);
    },
  });
}
