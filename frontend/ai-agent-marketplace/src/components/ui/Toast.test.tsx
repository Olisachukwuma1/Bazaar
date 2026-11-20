import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Toast } from './Toast';

describe('Toast', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders title and message', () => {
    render(<Toast id="1" type="success" title="Saved" message="All good" onClose={vi.fn()} />);
    expect(screen.getByText('Saved')).toBeInTheDocument();
    expect(screen.getByText('All good')).toBeInTheDocument();
  });

  it('calls onClose with the id when the close button is clicked', async () => {
    const onClose = vi.fn();
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(<Toast id="42" type="error" title="Failed" onClose={onClose} />);
    await user.click(screen.getByRole('button', { name: '✕' }));
    expect(onClose).toHaveBeenCalledWith('42');
  });

  it('auto-dismisses after the duration', () => {
    const onClose = vi.fn();
    render(<Toast id="7" type="info" title="Info" onClose={onClose} duration={1000} />);
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(onClose).toHaveBeenCalledWith('7');
  });
});
