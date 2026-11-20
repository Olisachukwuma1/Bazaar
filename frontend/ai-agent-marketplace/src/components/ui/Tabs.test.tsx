import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Tabs, TabsList, TabsTrigger, TabsContent } from './Tabs';

const renderTabs = (onValueChange = vi.fn()) =>
  render(
    <Tabs value="one" onValueChange={onValueChange}>
      <TabsList>
        <TabsTrigger value="one">One</TabsTrigger>
        <TabsTrigger value="two">Two</TabsTrigger>
      </TabsList>
      <TabsContent value="one">First panel</TabsContent>
      <TabsContent value="two">Second panel</TabsContent>
    </Tabs>
  );

describe('Tabs', () => {
  it('renders triggers and content', () => {
    renderTabs();
    expect(screen.getByRole('button', { name: 'One' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Two' })).toBeInTheDocument();
    expect(screen.getByText('First panel')).toBeInTheDocument();
    expect(screen.getByText('Second panel')).toBeInTheDocument();
  });

  it('calls onValueChange with the clicked trigger value', async () => {
    const onValueChange = vi.fn();
    renderTabs(onValueChange);
    await userEvent.click(screen.getByRole('button', { name: 'Two' }));
    expect(onValueChange).toHaveBeenCalledWith('two');
  });

  it('does not fire for disabled triggers', async () => {
    const onValueChange = vi.fn();
    render(
      <Tabs value="one" onValueChange={onValueChange}>
        <TabsList>
          <TabsTrigger value="two" disabled>
            Two
          </TabsTrigger>
        </TabsList>
      </Tabs>
    );
    await userEvent.click(screen.getByRole('button', { name: 'Two' }));
    expect(onValueChange).not.toHaveBeenCalled();
  });
});
