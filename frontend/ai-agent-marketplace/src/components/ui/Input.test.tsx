import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Input } from './Input';

describe('Input', () => {
  it('renders a labelled input', () => {
    render(<Input label="Email" />);
    expect(screen.getByLabelText('Email')).toBeInTheDocument();
  });

  it('accepts typed text', async () => {
    const onChange = vi.fn();
    render(<Input label="Name" onChange={onChange} />);
    await userEvent.type(screen.getByLabelText('Name'), 'Ada');
    expect(screen.getByLabelText('Name')).toHaveValue('Ada');
    expect(onChange).toHaveBeenCalled();
  });

  it('shows an error message and marks the input invalid', () => {
    render(<Input label="Email" error="Required" />);
    const input = screen.getByLabelText('Email');
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(input).toHaveAttribute('aria-describedby', 'email-error');
    expect(screen.getByRole('alert')).toHaveTextContent('Required');
  });

  it('forwards refs to the input element', () => {
    const ref = { current: null as HTMLInputElement | null };
    render(<Input label="Ref" ref={ref} />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });
});
