/** Stable server/browser formatting; no locale-dependent hydration changes. */
export function shortDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.valueOf())
    ? '—'
    : new Intl.DateTimeFormat('en-US', {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
        timeZone: 'UTC',
      }).format(date);
}
export function fileSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

export function money(
  value: string | null | undefined,
  currency = 'USD',
  compact = true,
) {
  if (value == null || value === '') return '—';
  const amount = Number(value);
  if (!Number.isFinite(amount)) return '—';
  try {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency || 'USD',
      notation: compact && Math.abs(amount) >= 10000 ? 'compact' : 'standard',
      maximumFractionDigits: compact && Math.abs(amount) >= 10000 ? 1 : 2,
    }).format(amount);
  } catch {
    return `${amount.toLocaleString('en-US', { maximumFractionDigits: 2 })} ${currency}`;
  }
}
export const frequency = (value?: string) =>
  ({
    one_time: 'one-time',
    monthly: '/ month',
    quarterly: '/ quarter',
    annually: '/ year',
  })[value ?? 'one_time'] ?? 'one-time';
export function messageTime(value: string) {
  return (
    new Intl.DateTimeFormat('en-US', {
      hour: 'numeric',
      minute: '2-digit',
      timeZone: 'UTC',
    }).format(new Date(value)) + ' UTC'
  );
}
