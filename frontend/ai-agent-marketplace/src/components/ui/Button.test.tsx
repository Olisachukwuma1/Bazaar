import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Button } from './Button';

describe('Button', () => {
  it('renders its children', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByRole('button', { name: 'Click me' })).toBeInTheDocument();
  });

  it('calls onClick when clicked', async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Go</Button>);
    await userEvent.click(screen.getByRole('button', { name: 'Go' }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('is disabled and not clickable while loading', async () => {
    const onClick = vi.fn();
    render(
      <Button isLoading onClick={onClick}>
        Saving
      </Button>
    );
    const button = screen.getByRole('button', { name: /saving/i });
    expect(button).toBeDisabled();
    await userEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it('applies variant and size classes', () => {
    render(
      <Button variant="destructive" size="lg">
        Delete
      </Button>
    );
    const button = screen.getByRole('button', { name: 'Delete' });
    expect(button.className).toContain('bg-destructive');
    expect(button.className).toContain('h-11');
  });
});
