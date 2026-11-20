import React from 'react';
import { Button } from './ui/Button';

interface RouteErrorBoundaryProps {
  children: React.ReactNode;
  /** Optional label used in the fallback and the report payload (e.g. the route path). */
  routeName?: string;
}

interface RouteErrorBoundaryState {
  error: Error | null;
}

/**
 * Catches render errors thrown by a single lazy route so one broken page does not
 * blank the whole app. The fallback offers a reset (re-render the route) and a
 * report action that surfaces the error to the console/error tracker.
 */
export class RouteErrorBoundary extends React.Component<
  RouteErrorBoundaryProps,
  RouteErrorBoundaryState
> {
  state: RouteErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): RouteErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error(`Route error${this.props.routeName ? ` in ${this.props.routeName}` : ''}:`, error, info);
  }

  private handleReset = () => {
    this.setState({ error: null });
  };

  private handleReport = () => {
    const { error } = this.state;
    if (!error) return;
    console.error('Reported route error:', {
      route: this.props.routeName ?? window.location.pathname,
      message: error.message,
      stack: error.stack,
    });
  };

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div
        role="alert"
        className="mx-auto max-w-xl px-4 py-16 text-center"
      >
        <div className="text-5xl mb-4">⚠️</div>
        <h1 className="text-xl font-bold text-foreground">Something went wrong</h1>
        <p className="text-sm text-muted-foreground mt-2 mb-6">
          This page failed to render. You can retry, or report the problem so we can fix it.
        </p>
        <div className="flex items-center justify-center gap-3">
          <Button onClick={this.handleReset}>Try again</Button>
          <Button variant="outline" onClick={this.handleReport}>
            Report problem
          </Button>
        </div>
      </div>
    );
  }
}
