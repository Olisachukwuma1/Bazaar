import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Badge } from './Badge';

describe('Badge', () => {
  it('renders its content', () => {
    render(<Badge>New</Badge>);
    expect(screen.getByText('New')).toBeInTheDocument();
  });

  it('applies the requested variant classes', () => {
    render(<Badge variant="success">Done</Badge>);
    expect(screen.getByText('Done').className).toContain('bg-green-500');
  });

  it('falls back to default variant for unknown values', () => {
    render(<Badge variant="unknown">Fallback</Badge>);
    expect(screen.getByText('Fallback').className).toContain('bg-primary');
  });

  it('merges custom className', () => {
    render(<Badge className="custom-class">Custom</Badge>);
    expect(screen.getByText('Custom').className).toContain('custom-class');
  });
});
