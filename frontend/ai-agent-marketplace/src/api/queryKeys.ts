/**
 * Centralised react-query key factory.
 *
 * Every cache key used by the data layer lives here so that invalidation and
 * refetching can be expressed against a single source of truth.
 *
 * Stale / refetch behaviour (see `src/api/README.md`):
 *   - `agents` / `jobs` lists: `staleTime` 5 minutes (marketplace browsing).
 *   - `stats`: `staleTime` 10 minutes (platform aggregates change slowly).
 *   - `job` / `agent` detail: default `staleTime` (0) — always revalidated on
 *     mount so escrow state and statuses are fresh.
 *   - paginated lists use `useInfiniteQuery` with `PAGE_SIZE` pages.
 */
export const queryKeys = {
  all: ['bazaar'] as const,

  agents: {
    all: () => [...queryKeys.all, 'agents'] as const,
    list: (skills: readonly string[]) => [...queryKeys.agents.all(), 'list', skills] as const,
    detail: (agentId: number) => [...queryKeys.agents.all(), 'detail', agentId] as const,
    byOwner: (owner: string | null) => [...queryKeys.agents.all(), 'owner', owner] as const,
    jobs: (agentId: number) => [...queryKeys.agents.all(), 'jobs', agentId] as const,
    reviews: (agentId: number) => [...queryKeys.agents.all(), 'reviews', agentId] as const,
    metadata: (url: string | null) => [...queryKeys.agents.all(), 'metadata', url] as const,
  },

  jobs: {
    all: () => [...queryKeys.all, 'jobs'] as const,
    list: () => [...queryKeys.jobs.all(), 'list'] as const,
    detail: (jobId: number) => [...queryKeys.jobs.all(), 'detail', jobId] as const,
    byClient: (client: string | null) => [...queryKeys.jobs.all(), 'client', client] as const,
    bids: (jobId: number) => [...queryKeys.jobs.all(), 'bids', jobId] as const,
    reviews: (jobId: number) => [...queryKeys.jobs.all(), 'reviews', jobId] as const,
  },

  bids: {
    all: () => [...queryKeys.all, 'bids'] as const,
    byAgent: (agent: string | null) => [...queryKeys.bids.all(), 'agent', agent] as const,
  },

  reviews: {
    all: () => [...queryKeys.all, 'reviews'] as const,
    received: (publicKey: string | null) => [...queryKeys.reviews.all(), 'received', publicKey] as const,
  },

  stats: {
    all: () => [...queryKeys.all, 'stats'] as const,
    platform: () => [...queryKeys.stats.all(), 'platform'] as const,
  },

  payments: {
    all: () => [...queryKeys.all, 'payments'] as const,
    byClient: (client: string | null) => [...queryKeys.payments.all(), client] as const,
  },
} as const;

/** Shared stale times (milliseconds) for the data layer. */
export const STALE_TIME = {
  /** Marketplace lists (agents, jobs). */
  list: 1000 * 60 * 5,
  /** Platform-wide aggregates. */
  stats: 1000 * 60 * 10,
  /** Detail views are always revalidated on mount. */
  detail: 0,
} as const;
