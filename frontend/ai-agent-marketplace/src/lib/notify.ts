import { toast } from 'sonner';

/**
 * Single entry point for user-facing notifications.
 * Backed by sonner (see the <Toaster /> in main.tsx).
 */
export const notify = {
  success(message: string, description?: string) {
    return toast.success(message, { description });
  },
  error(message: string, description?: string) {
    return toast.error(message, { description });
  },
  info(message: string, description?: string) {
    return toast(message, { description });
  },
  warning(message: string, description?: string) {
    return toast.warning(message, { description });
  },
  /** Report the outcome of a transaction (hash shown when available). */
  tx(message: string, hash?: string) {
    return toast.success(message, {
      description: hash ? `Tx: ${hash}` : undefined,
    });
  },
};

export default notify;
