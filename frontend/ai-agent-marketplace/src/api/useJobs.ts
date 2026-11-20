import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import { useStellar } from '@/contexts/StellarContext';
import { PAGE_SIZE, getNextPageParam } from '@/hooks/useProfile';
import type { Bid, Job, PlatformStats, Review } from '@/lib/types';
import { queryKeys, STALE_TIME } from './queryKeys';

const NOT_CONNECTED = 'Not connected to the marketplace contract. Configure it in Settings.';

/** Paginated list of every job on the marketplace. */
export function useJobs() {
  const { getContractClient } = useStellar();
  return useInfiniteQuery<Job[]>({
    queryKey: queryKeys.jobs.list(),
    queryFn: async ({ pageParam }) => {
      const offset = (pageParam as number) ?? 0;
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      return client.getAllJobs(offset, PAGE_SIZE);
    },
    initialPageParam: 0,
    staleTime: STALE_TIME.list,
    getNextPageParam,
  });
}

/** Single job, enriched with the escrow creation timestamp when available. */
export function useJob(jobId: number) {
  const { getContractClient } = useStellar();
  return useQuery<Job>({
    queryKey: queryKeys.jobs.detail(jobId),
    queryFn: async () => {
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      const job = await client.getJob(jobId);
      try {
        const escrow = await client.getEscrowByJob(jobId);
        if (escrow) job.escrowCreatedAt = escrow.createdAt;
      } catch {
        // non-fatal — escrowCreatedAt stays null
      }
      return job;
    },
    enabled: !!jobId,
    staleTime: STALE_TIME.detail,
  });
}

/** Paginated jobs published by a client wallet. */
export function useClientJobs(publicKey: string | null) {
  const { getContractClient } = useStellar();
  return useInfiniteQuery<Job[]>({
    queryKey: queryKeys.jobs.byClient(publicKey),
    queryFn: async ({ pageParam }) => {
      const offset = (pageParam as number) ?? 0;
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      const ids: number[] = await client.getJobsByClient(publicKey as string, offset, PAGE_SIZE);
      return Promise.all(ids.map((id: number) => client.getJob(id)));
    },
    initialPageParam: 0,
    enabled: !!publicKey,
    getNextPageParam,
  });
}

/** Paginated bids placed on a job. */
export function useBids(jobId: number) {
  const { getContractClient } = useStellar();
  return useInfiniteQuery<Bid[]>({
    queryKey: queryKeys.jobs.bids(jobId),
    queryFn: async ({ pageParam }) => {
      const offset = (pageParam as number) ?? 0;
      const client = getContractClient();
      if (!client) return [];
      const ids = await client.getBidsByJob(jobId, offset, PAGE_SIZE);
      return Promise.all(ids.map((id: number) => client.getBid(id)));
    },
    initialPageParam: 0,
    enabled: !!jobId,
    getNextPageParam,
  });
}

/** Paginated bids placed by an agent wallet. */
export function useAgentBids(publicKey: string | null) {
  const { getContractClient } = useStellar();
  return useInfiniteQuery<Bid[]>({
    queryKey: queryKeys.bids.byAgent(publicKey),
    queryFn: async ({ pageParam }) => {
      const offset = (pageParam as number) ?? 0;
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      const ids: number[] = await client.getBidsByAgent(publicKey as string, offset, PAGE_SIZE);
      return Promise.all(ids.map((id: number) => client.getBid(id)));
    },
    initialPageParam: 0,
    enabled: !!publicKey,
    getNextPageParam,
  });
}

/** Reviews left on a job. */
export function useJobReviews(jobId: number) {
  const { getContractClient } = useStellar();
  return useQuery<Review[]>({
    queryKey: queryKeys.jobs.reviews(jobId),
    queryFn: async () => {
      const client = getContractClient();
      if (!client) return [];
      return client.getReviewsByJob(jobId);
    },
    enabled: !!jobId,
    staleTime: STALE_TIME.detail,
  });
}

/** Paginated reviews received by a wallet. */
export function useReceivedReviews(publicKey: string | null, enabled = true) {
  const { getContractClient } = useStellar();
  return useInfiniteQuery<Review[]>({
    queryKey: queryKeys.reviews.received(publicKey),
    queryFn: async ({ pageParam }) => {
      const offset = (pageParam as number) ?? 0;
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      const ids: number[] = await client.getReviewsByReviewee(publicKey as string, offset, PAGE_SIZE);
      return client.getReviewsByIds(ids);
    },
    initialPageParam: 0,
    enabled: !!publicKey && enabled,
    getNextPageParam,
  });
}

/** Platform-wide aggregates. */
export function usePlatformStats() {
  const { getContractClient } = useStellar();
  return useQuery<PlatformStats>({
    queryKey: queryKeys.stats.platform(),
    queryFn: async () => {
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      return client.getPlatformStats();
    },
    staleTime: STALE_TIME.stats,
  });
}
