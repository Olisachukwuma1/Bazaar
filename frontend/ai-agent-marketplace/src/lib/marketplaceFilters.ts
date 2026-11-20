/**
 * Typed codec for the Marketplace filter state stored in the URL.
 *
 * The URL is the single source of truth for search, skills, sort, view mode,
 * tab and page so that reloads restore state and back/forward navigate filter
 * history. Every parser is total: invalid or unknown values fall back to the
 * defaults instead of throwing.
 */

export const MARKETPLACE_SORTS = ['reputation', 'price', 'jobs', 'recent'] as const;
export type MarketplaceSort = (typeof MARKETPLACE_SORTS)[number];

export const MARKETPLACE_VIEWS = ['grid', 'list'] as const;
export type MarketplaceView = (typeof MARKETPLACE_VIEWS)[number];

export const MARKETPLACE_TABS = ['agents', 'jobs'] as const;
export type MarketplaceTab = (typeof MARKETPLACE_TABS)[number];

export interface MarketplaceFilters {
  search: string;
  skills: string[];
  sort: MarketplaceSort;
  view: MarketplaceView;
  tab: MarketplaceTab;
  page: number;
}

export const DEFAULT_MARKETPLACE_FILTERS: MarketplaceFilters = {
  search: '',
  skills: [],
  sort: 'reputation',
  view: 'grid',
  tab: 'agents',
  page: 1,
};

/** Maximum page accepted from the URL; guards against absurd offsets. */
export const MAX_MARKETPLACE_PAGE = 1000;

const isSort = (value: string): value is MarketplaceSort =>
  (MARKETPLACE_SORTS as readonly string[]).includes(value);

const isView = (value: string): value is MarketplaceView =>
  (MARKETPLACE_VIEWS as readonly string[]).includes(value);

const isTab = (value: string): value is MarketplaceTab =>
  (MARKETPLACE_TABS as readonly string[]).includes(value);

/** Parse a positive integer page, falling back to the default page. */
export const parsePage = (raw: string | null): number => {
  if (!raw) return DEFAULT_MARKETPLACE_FILTERS.page;
  const parsed = Number(raw);
  if (!Number.isInteger(parsed) || parsed < 1 || parsed > MAX_MARKETPLACE_PAGE) {
    return DEFAULT_MARKETPLACE_FILTERS.page;
  }
  return parsed;
};

/** Parse a comma-separated skill list, dropping blanks and duplicates. */
export const parseSkills = (raw: string | null): string[] => {
  if (!raw) return [];
  const seen = new Set<string>();
  for (const part of raw.split(',')) {
    const skill = part.trim().toLowerCase();
    if (skill) seen.add(skill);
  }
  return [...seen];
};

/** Decode URL search params into a complete, validated filter state. */
export const decodeMarketplaceFilters = (
  params: URLSearchParams,
): MarketplaceFilters => {
  const sort = params.get('sort') ?? '';
  const view = params.get('view') ?? '';
  const tab = params.get('tab') ?? '';
  return {
    search: params.get('q') ?? '',
    skills: parseSkills(params.get('skills')),
    sort: isSort(sort) ? sort : DEFAULT_MARKETPLACE_FILTERS.sort,
    view: isView(view) ? view : DEFAULT_MARKETPLACE_FILTERS.view,
    tab: isTab(tab) ? tab : DEFAULT_MARKETPLACE_FILTERS.tab,
    page: parsePage(params.get('page')),
  };
};

/**
 * Encode a filter state into URL search params, omitting values that equal the
 * defaults so the canonical URL stays short and shareable.
 */
export const encodeMarketplaceFilters = (
  filters: MarketplaceFilters,
): URLSearchParams => {
  const params = new URLSearchParams();
  if (filters.search) params.set('q', filters.search);
  if (filters.skills.length > 0) params.set('skills', filters.skills.join(','));
  if (filters.sort !== DEFAULT_MARKETPLACE_FILTERS.sort) params.set('sort', filters.sort);
  if (filters.view !== DEFAULT_MARKETPLACE_FILTERS.view) params.set('view', filters.view);
  if (filters.tab !== DEFAULT_MARKETPLACE_FILTERS.tab) params.set('tab', filters.tab);
  if (filters.page !== DEFAULT_MARKETPLACE_FILTERS.page) params.set('page', String(filters.page));
  return params;
};

/** True when any filter (search or skills) is active. */
export const hasActiveMarketplaceFilters = (filters: MarketplaceFilters): boolean =>
  filters.search.length > 0 || filters.skills.length > 0;
