import React from 'react';
import { Link } from 'react-router-dom';
import { EmptyState } from '@/components/ui/EmptyState';

export const NotFound: React.FC = () => {
  return (
    <div className="mx-auto max-w-3xl px-4 py-16">
      <EmptyState
        icon="🧭"
        title="404 — Page not found"
        description="The page you are looking for does not exist or has moved."
        action={
          <Link
            to="/marketplace"
            className="inline-flex items-center justify-center rounded-lg border border-input bg-background h-10 px-4 py-2 text-sm font-medium hover:bg-accent hover:text-accent-foreground"
          >
            Back to marketplace
          </Link>
        }
      />
    </div>
  );
};
