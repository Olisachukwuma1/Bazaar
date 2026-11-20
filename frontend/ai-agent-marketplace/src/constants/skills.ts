/**
 * Single source of truth for the skills accepted by the marketplace contract.
 *
 * Slugs must match the contract-valid format: lowercase ASCII alphanumerics
 * separated by single hyphens (e.g. `machine-learning`).
 */

export const SKILL_SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export const SKILL_CATEGORIES = {
  Languages: ['python', 'javascript', 'typescript', 'rust', 'go', 'java', 'cpp'],
  'AI/ML': ['machine-learning', 'deep-learning', 'nlp', 'computer-vision', 'data-science', 'pytorch', 'tensorflow', 'jax', 'huggingface', 'langchain', 'openai-api'],
  Blockchain: ['smart-contracts', 'soroban', 'stellar', 'blockchain', 'web3', 'defi'],
  'Web/Backend': ['react', 'vue', 'nextjs', 'nodejs', 'express', 'fastapi', 'django'],
  Infrastructure: ['postgresql', 'mongodb', 'redis', 'docker', 'kubernetes', 'aws', 'gcp'],
  Finance: ['trading', 'quantitative-analysis', 'risk-management', 'portfolio-optimization'],
} as const satisfies Record<string, readonly string[]>;

export type SkillCategory = keyof typeof SKILL_CATEGORIES;

export type Skill = (typeof SKILL_CATEGORIES)[SkillCategory][number];

export const ALL_SKILLS: readonly Skill[] = Object.values(SKILL_CATEGORIES).flat() as Skill[];

const SKILL_SET: ReadonlySet<string> = new Set(ALL_SKILLS);

/** True when `value` is one of the contract-valid skills. */
export function isKnownSkill(value: string): value is Skill {
  return SKILL_SET.has(value);
}

/** True when `value` matches the contract-valid slug format. */
export function isValidSkillSlug(value: string): boolean {
  return SKILL_SLUG_PATTERN.test(value);
}

/** Human-readable label for a skill slug, e.g. `machine-learning` -> `Machine Learning`. */
export function skillLabel(skill: string): string {
  return skill
    .split('-')
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

/** Tailwind classes used to colour a skill badge. */
export function getSkillColor(skill: string): string {
  const colors = [
    'bg-primary/10 text-primary',
    'bg-stellar/10 text-stellar',
    'bg-green-500/10 text-green-500',
    'bg-orange-500/10 text-orange-500',
    'bg-pink-500/10 text-pink-500',
    'bg-cyan-500/10 text-cyan-500',
  ];
  const hash = skill.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return colors[hash % colors.length];
}
