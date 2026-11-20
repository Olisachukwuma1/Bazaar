import React from 'react';
import { AlertCircle } from 'lucide-react';
import type { EnvConfigError } from '../config/env';

/**
 * Full-screen, typed error shown when the VITE_* environment configuration is
 * missing or invalid. Rendered before any provider/router work so the app
 * fails fast with an actionable message instead of a blank page.
 */
export const EnvErrorScreen: React.FC<{ error: EnvConfigError }> = ({ error }) => (
  <div className="flex min-h-screen items-center justify-center bg-background p-6">
    <div className="w-full max-w-xl rounded-xl border border-destructive/40 bg-card p-8">
      <div className="mb-4 flex items-center gap-3">
        <AlertCircle className="h-8 w-8 text-destructive" />
        <h1 className="text-xl font-bold text-foreground">Configuration error</h1>
      </div>
      <p className="mb-4 text-sm text-muted-foreground">
        The app cannot start because its environment variables are missing or invalid.
        Copy <code className="font-mono">.env.example</code> to{' '}
        <code className="font-mono">.env.local</code>, fill in the values, and restart the dev server.
      </p>
      <ul className="space-y-1 rounded-lg bg-muted/40 p-4 font-mono text-xs text-destructive">
        {error.issues.map((issue) => (
          <li key={issue}>{issue}</li>
        ))}
      </ul>
    </div>
  </div>
);
