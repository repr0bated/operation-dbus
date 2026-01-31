import type { ReactNode } from 'react';
import { useAuthStore } from '@/stores';

interface Props {
  roles?: string[];
  children: ReactNode;
  fallback?: ReactNode;
}

export function RBACGate({ roles, children, fallback = null }: Props) {
  const user = useAuthStore((s) => s.user);
  if (!user) return fallback;
  // For now, just check if logged in. Extend with role checking as needed.
  return <>{children}</>;
}
