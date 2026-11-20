import React from 'react';
import { StatCard } from '@/components/ui/StatCard';

/** A single stat rendered by {@link StatGrid}. */
export interface StatGridItem {
  label: string;
  value: React.ReactNode;
  icon: React.ComponentType<{ className?: string }>;
  color?: string;
}

export interface StatGridProps {
  items: StatGridItem[];
  /** Tailwind grid classes; defaults to a 2/4 column responsive grid. */
  className?: string;
}

/**
 * Shared responsive grid of stat cards used by the client and agent dashboards.
 */
export const StatGrid: React.FC<StatGridProps> = ({
  items,
  className = 'grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8',
}) => (
  <div className={className}>
    {items.map((item) => (
      <StatCard
        key={item.label}
        label={item.label}
        value={item.value}
        icon={item.icon}
        color={item.color}
      />
    ))}
  </div>
);

export default StatGrid;
