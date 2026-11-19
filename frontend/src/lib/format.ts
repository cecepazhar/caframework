import { intlLocale } from './i18n/index.svelte';

export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(intlLocale(), options).format(value);
}

export function formatCurrency(value: number, currency = 'USD', options?: Intl.NumberFormatOptions): string {
  const isIdr = currency.toUpperCase() === 'IDR';
  const defaultFractionDigits = isIdr ? 0 : 2;
  return new Intl.NumberFormat(intlLocale(), {
    style: 'currency',
    currency,
    minimumFractionDigits: options?.minimumFractionDigits ?? defaultFractionDigits,
    maximumFractionDigits: options?.maximumFractionDigits ?? defaultFractionDigits,
    ...options
  }).format(value);
}

export function formatDate(date: Date | number | string, options?: Intl.DateTimeFormatOptions): string {
  const d = typeof date === 'string' || typeof date === 'number' ? new Date(date) : date;
  return new Intl.DateTimeFormat(intlLocale(), {
    dateStyle: 'medium',
    ...options
  }).format(d);
}

export function formatDateTime(date: Date | number | string, options?: Intl.DateTimeFormatOptions): string {
  const d = typeof date === 'string' || typeof date === 'number' ? new Date(date) : date;
  return new Intl.DateTimeFormat(intlLocale(), {
    dateStyle: 'medium',
    timeStyle: 'short',
    ...options
  }).format(d);
}

export function formatRelative(timestamp: number | Date): string {
  const now = Date.now();
  const time = typeof timestamp === 'number' ? timestamp : timestamp.getTime();
  const diffSec = Math.round((time - now) / 1000);

  const rtf = new Intl.RelativeTimeFormat(intlLocale(), { numeric: 'auto' });

  if (Math.abs(diffSec) < 60) {
    return rtf.format(diffSec, 'second');
  }
  const diffMin = Math.round(diffSec / 60);
  if (Math.abs(diffMin) < 60) {
    return rtf.format(diffMin, 'minute');
  }
  const diffHour = Math.round(diffMin / 60);
  if (Math.abs(diffHour) < 24) {
    return rtf.format(diffHour, 'hour');
  }
  const diffDay = Math.round(diffHour / 24);
  return rtf.format(diffDay, 'day');
}

/**
 * Formats a phone number for display.
 * For Indonesian phone numbers ('id' locale or numbers starting with 08, +628, 628):
 * - Groups domestic mobile numbers as '0852 2069 6117' (4-4-4).
 * - Formats international numbers cleanly e.g. '+62 852 2069 6117'.
 */
export function formatPhone(phone: string, localeOverride?: string): string {
  if (!phone) return '';
  const cleaned = phone.trim();
  const digits = cleaned.replace(/\D/g, '');
  const loc = localeOverride ?? intlLocale();

  if (loc.startsWith('id') || cleaned.startsWith('08') || cleaned.startsWith('+62') || cleaned.startsWith('62')) {
    if (digits.startsWith('08')) {
      if (digits.length <= 4) return digits;
      if (digits.length <= 8) return `${digits.slice(0, 4)} ${digits.slice(4)}`;
      if (digits.length <= 12) return `${digits.slice(0, 4)} ${digits.slice(4, 8)} ${digits.slice(8)}`;
      return `${digits.slice(0, 4)} ${digits.slice(4, 8)} ${digits.slice(8, 12)} ${digits.slice(12)}`;
    }
    if (digits.startsWith('628')) {
      const rest = digits.slice(2);
      if (rest.length <= 3) return `+62 ${rest}`;
      if (rest.length <= 7) return `+62 ${rest.slice(0, 3)} ${rest.slice(3)}`;
      return `+62 ${rest.slice(0, 3)} ${rest.slice(3, 7)} ${rest.slice(7)}`;
    }
    if (digits.startsWith('0')) {
      if (digits.length <= 3) return digits;
      if (digits.length <= 7) return `${digits.slice(0, 3)} ${digits.slice(3)}`;
      return `${digits.slice(0, 3)} ${digits.slice(3, 7)} ${digits.slice(7)}`;
    }
  }

  if (cleaned.startsWith('+1') && digits.length === 11) {
    return `+1 ${digits.slice(1, 4)} ${digits.slice(4, 7)}${digits.slice(7)}`;
  }

  if (cleaned.startsWith('+')) {
    if (digits.length > 10) {
      return `+${digits.slice(0, 2)} ${digits.slice(2, 6)} ${digits.slice(6)}`;
    }
    return `+${digits}`;
  }

  return phone;
}
