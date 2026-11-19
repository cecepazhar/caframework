// Literal class map for theme accents.
// Guard requirement (P16.3): NO Tailwind class may be built from a template string (e.g. `bg-${accent}-500`).
// Every class must be written as a static literal string so the Tailwind compiler detects and includes it.

import { APP_ACCENT } from '$lib/appInfo';

export interface AccentTheme {
  name: string;
  primary: string;
  primaryHover: string;
  bgSubtle: string;
  text: string;
  textSubtle: string;
  border: string;
  borderSubtle: string;
  ring: string;
  dot: string;
}

export const ACCENT_MAP: Record<string, AccentTheme> = {
  sky: {
    name: 'Sky',
    primary: 'bg-sky-600 dark:bg-sky-500 text-white',
    primaryHover: 'hover:bg-sky-500 dark:hover:bg-sky-400',
    bgSubtle: 'bg-sky-500/10 text-sky-600 dark:text-sky-400',
    text: 'text-sky-600 dark:text-sky-400',
    textSubtle: 'text-sky-500/80 dark:text-sky-400/80',
    border: 'border-sky-500 dark:border-sky-400',
    borderSubtle: 'border-sky-500/20 dark:border-sky-400/20',
    ring: 'focus:ring-sky-500 focus:border-sky-500',
    dot: 'bg-sky-500'
  },
  emerald: {
    name: 'Emerald',
    primary: 'bg-emerald-600 dark:bg-emerald-500 text-white',
    primaryHover: 'hover:bg-emerald-500 dark:hover:bg-emerald-400',
    bgSubtle: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
    text: 'text-emerald-600 dark:text-emerald-400',
    textSubtle: 'text-emerald-500/80 dark:text-emerald-400/80',
    border: 'border-emerald-500 dark:border-emerald-400',
    borderSubtle: 'border-emerald-500/20 dark:border-emerald-400/20',
    ring: 'focus:ring-emerald-500 focus:border-emerald-500',
    dot: 'bg-emerald-500'
  },
  rose: {
    name: 'Rose',
    primary: 'bg-rose-600 dark:bg-rose-500 text-white',
    primaryHover: 'hover:bg-rose-500 dark:hover:bg-rose-400',
    bgSubtle: 'bg-rose-500/10 text-rose-600 dark:text-rose-400',
    text: 'text-rose-600 dark:text-rose-400',
    textSubtle: 'text-rose-500/80 dark:text-rose-400/80',
    border: 'border-rose-500 dark:border-rose-400',
    borderSubtle: 'border-rose-500/20 dark:border-rose-400/20',
    ring: 'focus:ring-rose-500 focus:border-rose-500',
    dot: 'bg-rose-500'
  },
  violet: {
    name: 'Violet',
    primary: 'bg-violet-600 dark:bg-violet-500 text-white',
    primaryHover: 'hover:bg-violet-500 dark:hover:bg-violet-400',
    bgSubtle: 'bg-violet-500/10 text-violet-600 dark:text-violet-400',
    text: 'text-violet-600 dark:text-violet-400',
    textSubtle: 'text-violet-500/80 dark:text-violet-400/80',
    border: 'border-violet-500 dark:border-violet-400',
    borderSubtle: 'border-violet-500/20 dark:border-violet-400/20',
    ring: 'focus:ring-violet-500 focus:border-violet-500',
    dot: 'bg-violet-500'
  },
  amber: {
    name: 'Amber',
    primary: 'bg-amber-600 dark:bg-amber-500 text-white',
    primaryHover: 'hover:bg-amber-500 dark:hover:bg-amber-400',
    bgSubtle: 'bg-amber-500/10 text-amber-600 dark:text-amber-400',
    text: 'text-amber-600 dark:text-amber-400',
    textSubtle: 'text-amber-500/80 dark:text-amber-400/80',
    border: 'border-amber-500 dark:border-amber-400',
    borderSubtle: 'border-amber-500/20 dark:border-amber-400/20',
    ring: 'focus:ring-amber-500 focus:border-amber-500',
    dot: 'bg-amber-500'
  },
  indigo: {
    name: 'Indigo',
    primary: 'bg-indigo-600 dark:bg-indigo-500 text-white',
    primaryHover: 'hover:bg-indigo-500 dark:hover:bg-indigo-400',
    bgSubtle: 'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400',
    text: 'text-indigo-600 dark:text-indigo-400',
    textSubtle: 'text-indigo-500/80 dark:text-indigo-400/80',
    border: 'border-indigo-500 dark:border-indigo-400',
    borderSubtle: 'border-indigo-500/20 dark:border-indigo-400/20',
    ring: 'focus:ring-indigo-500 focus:border-indigo-500',
    dot: 'bg-indigo-500'
  },
  neutral: {
    name: 'Neutral',
    primary: 'bg-neutral-900 dark:bg-white text-white dark:text-neutral-900',
    primaryHover: 'hover:bg-neutral-800 dark:hover:bg-neutral-100',
    bgSubtle: 'bg-neutral-500/10 text-neutral-900 dark:text-white',
    text: 'text-neutral-900 dark:text-white',
    textSubtle: 'text-neutral-500 dark:text-neutral-400',
    border: 'border-neutral-900 dark:border-white',
    borderSubtle: 'border-neutral-200 dark:border-neutral-800',
    ring: 'focus:ring-neutral-500 focus:border-neutral-500',
    dot: 'bg-neutral-500'
  }
};

/**
 * Returns the literal AccentTheme based on app configuration.
 * Fallback to 'sky' if the accent is unknown.
 */
export function getAppAccent(accentName: string = APP_ACCENT): AccentTheme {
  return ACCENT_MAP[accentName] ?? ACCENT_MAP.sky;
}

