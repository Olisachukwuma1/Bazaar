import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
import { RouteErrorBoundary } from './RouteErrorBoundary';
import { NotFound } from '../pages/NotFound';

const Boom = () => {
  throw new Error('boom');
};

describe('RouteErrorBoundary', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the fallback when a child throws and resets on retry', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    let shouldThrow = true;
    const Flaky = () => {
      if (shouldThrow) throw new Error('boom');
      return <p>Recovered</p>;
    };

    render(
      <RouteErrorBoundary routeName="/flaky">
        <Flaky />
      </RouteErrorBoundary>
    );

    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    shouldThrow = false;
    await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(screen.getByText('Recovered')).toBeInTheDocument();
  });

  it('reports the error when the report button is clicked', async () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <RouteErrorBoundary routeName="/boom">
        <Boom />
      </RouteErrorBoundary>
    );

    await userEvent.click(screen.getByRole('button', { name: 'Report problem' }));
    expect(spy).toHaveBeenCalledWith(
      'Reported route error:',
      expect.objectContaining({ route: '/boom', message: 'boom' })
    );
  });
});

describe('NotFound route', () => {
  it('renders a 404 for unknown URLs instead of redirecting', () => {
    render(
      <MemoryRouter initialEntries={['/does-not-exist']}>
        <Routes>
          <Route path="/marketplace" element={<p>Marketplace</p>} />
          <Route path="*" element={<NotFound />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText(/404/)).toBeInTheDocument();
    expect(screen.queryByText('Marketplace')).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: /back to marketplace/i })).toHaveAttribute(
      'href',
      '/marketplace'
    );
  });
});
