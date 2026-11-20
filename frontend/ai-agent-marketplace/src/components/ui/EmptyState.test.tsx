import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { EmptyState } from './EmptyState';

describe('EmptyState', () => {
  it('renders title, description and default icon', () => {
    render(<EmptyState title="Nothing here" description="Try again later" />);
    expect(screen.getByText('Nothing here')).toBeInTheDocument();
    expect(screen.getByText('Try again later')).toBeInTheDocument();
    expect(screen.getByText('🔍')).toBeInTheDocument();
  });

  it('renders the action button and calls onAction', async () => {
    const onAction = vi.fn();
    render(
      <EmptyState
        title="Empty"
        description="No items"
        actionLabel="Create"
        onAction={onAction}
      />
    );
    await userEvent.click(screen.getByRole('button', { name: 'Create' }));
    expect(onAction).toHaveBeenCalledTimes(1);
  });

  it('renders a custom action node instead of the button', () => {
    render(
      <EmptyState
        title="Empty"
        description="No items"
        actionLabel="Create"
        onAction={vi.fn()}
        action={<a href="/new">Custom action</a>}
      />
    );
    expect(screen.getByText('Custom action')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Create' })).not.toBeInTheDocument();
  });
});
