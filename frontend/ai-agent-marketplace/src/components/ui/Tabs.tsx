import * as React from 'react';
import { clsx } from 'clsx';

interface TabsContextValue {
  value: string;
  onValueChange: (value: string) => void;
  baseId: string;
}

const TabsContext = React.createContext<TabsContextValue | null>(null);

const useTabsContext = (component: string) => {
  const ctx = React.useContext(TabsContext);
  if (!ctx) throw new Error(`${component} must be used within <Tabs>`);
  return ctx;
};

const tabId = (baseId: string, value: string) => `${baseId}-tab-${value}`;
const panelId = (baseId: string, value: string) => `${baseId}-panel-${value}`;

interface TabsProps {
  value: string;
  onValueChange: (value: string) => void;
  children: React.ReactNode;
  className?: string;
}

export const Tabs = ({ value, onValueChange, children, className }: TabsProps) => {
  const baseId = React.useId();
  const ctx = React.useMemo(() => ({ value, onValueChange, baseId }), [value, onValueChange, baseId]);

  return (
    <TabsContext.Provider value={ctx}>
      <div className={clsx('space-y-4', className)}>{children}</div>
    </TabsContext.Provider>
  );
};

interface TabsListProps {
  children: React.ReactNode;
  className?: string;
  'aria-label'?: string;
}

export const TabsList = ({ children, className, 'aria-label': ariaLabel }: TabsListProps) => {
  const listRef = React.useRef<HTMLDivElement>(null);

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const keys = ['ArrowRight', 'ArrowLeft', 'Home', 'End'];
    if (!keys.includes(event.key)) return;

    const tabs = Array.from(
      listRef.current?.querySelectorAll<HTMLButtonElement>('[role="tab"]:not([disabled])') ?? []
    );
    if (tabs.length === 0) return;

    const currentIndex = tabs.findIndex(tab => tab === document.activeElement);
    let nextIndex = currentIndex;

    switch (event.key) {
      case 'ArrowRight':
        nextIndex = currentIndex < 0 ? 0 : (currentIndex + 1) % tabs.length;
        break;
      case 'ArrowLeft':
        nextIndex = currentIndex < 0 ? tabs.length - 1 : (currentIndex - 1 + tabs.length) % tabs.length;
        break;
      case 'Home':
        nextIndex = 0;
        break;
      case 'End':
        nextIndex = tabs.length - 1;
        break;
    }

    event.preventDefault();
    tabs[nextIndex]?.focus();
  };

  return (
    <div
      ref={listRef}
      role="tablist"
      aria-label={ariaLabel}
      aria-orientation="horizontal"
      onKeyDown={handleKeyDown}
      className={clsx('flex h-10 items-center justify-center rounded-md bg-muted p-1 text-muted-foreground', className)}
    >
      {children}
    </div>
  );
};

interface TabsTriggerProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'value'> {
  value: string;
  children: React.ReactNode;
  disabled?: boolean;
  className?: string;
}

export const TabsTrigger = React.forwardRef<HTMLButtonElement, TabsTriggerProps>(
  ({ value, children, disabled, className, onClick, ...props }, ref) => {
    const { value: selectedValue, onValueChange, baseId } = useTabsContext('TabsTrigger');
    const isSelected = selectedValue === value;

    return (
      <button
        ref={ref}
        type="button"
        role="tab"
        id={tabId(baseId, value)}
        aria-selected={isSelected}
        aria-controls={panelId(baseId, value)}
        tabIndex={isSelected ? 0 : -1}
        data-value={value}
        data-state={isSelected ? 'active' : 'inactive'}
        disabled={disabled}
        className={clsx(
          'inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm',
          className
        )}
        onClick={event => {
          onClick?.(event);
          if (!event.defaultPrevented) onValueChange(value);
        }}
        {...props}
      >
        {children}
      </button>
    );
  }
);
TabsTrigger.displayName = 'TabsTrigger';

interface TabsContentProps {
  value: string;
  children: React.ReactNode;
  className?: string;
  forceMount?: boolean;
}

export const TabsContent = ({ value, children, className, forceMount }: TabsContentProps) => {
  const { value: selectedValue, baseId } = useTabsContext('TabsContent');
  const isSelected = selectedValue === value;

  if (!forceMount && !isSelected) return null;

  return (
    <div
      role="tabpanel"
      id={panelId(baseId, value)}
      aria-labelledby={tabId(baseId, value)}
      tabIndex={0}
      data-value={value}
      data-state={isSelected ? 'active' : 'inactive'}
      hidden={!isSelected}
      className={clsx(
        'mt-2 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
        className
      )}
    >
      {children}
    </div>
  );
};

export default { Tabs, TabsList, TabsTrigger, TabsContent };