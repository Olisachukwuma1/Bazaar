import { useCallback, useMemo, useState } from 'react';
import type { Agent, Job } from '@/lib/types';

export const ALL_SKILLS = [
  'python', 'javascript', 'typescript', 'rust', 'go', 'java', 'cpp',
  'machine-learning', 'deep-learning', 'nlp', 'computer-vision', 'data-science',
  'pytorch', 'tensorflow', 'jax', 'huggingface', 'langchain', 'openai-api',
  'smart-contracts', 'soroban', 'stellar', 'blockchain', 'web3', 'defi',
  'react', 'vue', 'nextjs', 'nodejs', 'express', 'fastapi', 'django',
  'postgresql', 'mongodb', 'redis', 'docker', 'kubernetes', 'aws', 'gcp',
  'trading', 'quantitative-analysis', 'risk-management', 'portfolio-optimization',
];

export type MarketplaceSortBy = 'reputation' | 'price' | 'jobs' | 'recent';
export type MarketplaceViewMode = 'grid' | 'list';
export type MarketplaceTab = 'agents' | 'jobs';

export interface UseMarketplaceFiltersResult {
  searchQuery: string;
  setSearchQuery: (value: string) => void;
  selectedSkills: string[];
  sortBy: MarketplaceSortBy;
  setSortBy: (value: MarketplaceSortBy) => void;
  viewMode: MarketplaceViewMode;
  setViewMode: (value: MarketplaceViewMode) => void;
  activeTab: MarketplaceTab;
  setActiveTab: (value: MarketplaceTab) => void;
  handleSkillToggle: (skill: string) => void;
  clearFilters: () => void;
  hasFilters: boolean;
  filteredAgents: Agent[];
  filteredJobs: Job[];
}

/**
 * Owns the Marketplace page's search/sort/view state and derives the
 * filtered agent and job lists. Behaviour is unchanged from the original
 * inline implementation in pages/Marketplace.tsx.
 */
export function useMarketplaceFilters(
  agents: Agent[],
  jobs: Job[],
): UseMarketplaceFiltersResult {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedSkills, setSelectedSkills] = useState<string[]>([]);
  const [sortBy, setSortBy] = useState<MarketplaceSortBy>('reputation');
  const [viewMode, setViewMode] = useState<MarketplaceViewMode>('grid');
  const [activeTab, setActiveTab] = useState<MarketplaceTab>('agents');

  const filteredAgents = useMemo(
    () =>
      agents
        .filter(agent => {
          if (!agent.isActive) return false;
          if (searchQuery) {
            const query = searchQuery.toLowerCase();
            if (
              !agent.name.toLowerCase().includes(query) &&
              !agent.description.toLowerCase().includes(query) &&
              !agent.skills.some(s => s.toLowerCase().includes(query))
            ) {
              return false;
            }
          }
          if (selectedSkills.length > 0) {
            if (!selectedSkills.every(skill => agent.skills.includes(skill))) {
              return false;
            }
          }
          return true;
        })
        .sort((a, b) => {
          switch (sortBy) {
            case 'reputation': return b.reputation - a.reputation;
            case 'price': return a.pricePerTask - b.pricePerTask;
            case 'jobs': return b.totalJobsCompleted - a.totalJobsCompleted;
            case 'recent': return b.createdAt - a.createdAt;
            default: return 0;
          }
        }),
    [agents, searchQuery, selectedSkills, sortBy],
  );

  const filteredJobs = useMemo(
    () =>
      jobs
        .filter(job => {
          if (job.status !== 'Open') return false;
          if (searchQuery) {
            const query = searchQuery.toLowerCase();
            if (
              !job.title.toLowerCase().includes(query) &&
              !job.description.toLowerCase().includes(query) &&
              !job.requiredSkills.some(s => s.toLowerCase().includes(query))
            ) {
              return false;
            }
          }
          if (selectedSkills.length > 0) {
            if (!selectedSkills.every(skill => job.requiredSkills.includes(skill))) {
              return false;
            }
          }
          return true;
        })
        .sort((a, b) => b.budget - a.budget),
    [jobs, searchQuery, selectedSkills],
  );

  const handleSkillToggle = useCallback((skill: string) => {
    setSelectedSkills(prev =>
      prev.includes(skill) ? prev.filter(s => s !== skill) : [...prev, skill],
    );
  }, []);

  const clearFilters = useCallback(() => {
    setSearchQuery('');
    setSelectedSkills([]);
  }, []);

  const hasFilters = Boolean(searchQuery) || selectedSkills.length > 0;

  return {
    searchQuery,
    setSearchQuery,
    selectedSkills,
    sortBy,
    setSortBy,
    viewMode,
    setViewMode,
    activeTab,
    setActiveTab,
    handleSkillToggle,
    clearFilters,
    hasFilters,
    filteredAgents,
    filteredJobs,
  };
}
