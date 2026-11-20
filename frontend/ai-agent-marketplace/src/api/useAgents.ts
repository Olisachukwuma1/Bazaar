import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import { useStellar } from '@/contexts/StellarContext';
import { PAGE_SIZE, getNextPageParam } from '@/hooks/useProfile';
import type { Agent, Job, Review } from '@/lib/types';
import { queryKeys, STALE_TIME } from './queryKeys';

const NOT_CONNECTED = 'Not connected to the marketplace contract. Configure it in Settings.';

/** Paginated agent list, optionally filtered by an intersection of skills. */
export function useAgents(skills: readonly string[] = []) {
  const { getContractClient } = useStellar();
  return useInfiniteQuery<Agent[]>({
    queryKey: queryKeys.agents.list(skills),
    queryFn: async ({ pageParam }) => {
      const offset = (pageParam as number) ?? 0;
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      if (skills.length > 0) {
        const sets = await Promise.all(
          skills.map((skill) => client.getAgentsBySkill(skill, offset, PAGE_SIZE)),
        );
        const intersection = sets[0].filter((id: number) =>
          sets.every((s: number[]) => s.includes(id)),
        );
        return Promise.all(intersection.map((id: number) => client.getAgent(id)));
      }
      return client.getAllAgents(offset, PAGE_SIZE);
    },
    initialPageParam: 0,
    staleTime: STALE_TIME.list,
    getNextPageParam,
  });
}

/** Single agent by id. */
export function useAgent(agentId: number) {
  const { getContractClient } = useStellar();
  return useQuery<Agent>({
    queryKey: queryKeys.agents.detail(agentId),
    queryFn: async () => {
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      return client.getAgent(agentId);
    },
    enabled: !!agentId,
    staleTime: STALE_TIME.detail,
  });
}

/** Agent owned by the connected wallet (null when none is registered). */
export function useMyAgent(publicKey: string | null) {
  const { getContractClient } = useStellar();
  return useQuery<Agent | null>({
    queryKey: queryKeys.agents.byOwner(publicKey),
    queryFn: async () => {
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      return client.getAgentByOwner(publicKey as string);
    },
    enabled: !!publicKey,
    staleTime: STALE_TIME.detail,
  });
}

/** Paginated jobs published by an agent's owner. */
export function useAgentJobs(agent: Agent | undefined, agentId: number) {
  const { getContractClient } = useStellar();
  return useInfiniteQuery<Job[]>({
    queryKey: queryKeys.agents.jobs(agentId),
    queryFn: async ({ pageParam }) => {
      const offset = (pageParam as number) ?? 0;
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      if (!agent) return [];
      const ids: number[] = await client.getJobsByAgent(agent.owner, offset, PAGE_SIZE);
      return Promise.all(ids.map((id: number) => client.getJob(id)));
    },
    initialPageParam: 0,
    enabled: !!agentId && !!agent,
    getNextPageParam,
  });
}

/** Paginated reviews received by an agent. */
export function useAgentReviews(agentId: number) {
  const { getContractClient } = useStellar();
  return useInfiniteQuery<Review[]>({
    queryKey: queryKeys.agents.reviews(agentId),
    queryFn: async ({ pageParam }) => {
      const offset = (pageParam as number) ?? 0;
      const client = getContractClient();
      if (!client) throw new Error(NOT_CONNECTED);
      const agent = await client.getAgent(agentId);
      if (!agent) return [];
      const ids: number[] = await client.getReviewsByReviewee(agent.owner, offset, PAGE_SIZE);
      return Promise.all(ids.map((id: number) => client.getReview(id)));
    },
    initialPageParam: 0,
    enabled: !!agentId,
    getNextPageParam,
  });
}
