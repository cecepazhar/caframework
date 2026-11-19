import { describe, it, expect } from 'vitest';
import { appCommandFor, isTypingTarget, isTerminalTarget } from '$lib/shortcuts';

describe('Centralized keyboard shortcuts (P16.7)', () => {
  it('Ctrl+K or Ctrl+Shift+P resolves to palette command', () => {
    const kEvent = new KeyboardEvent('keydown', { key: 'k', ctrlKey: true });
    expect(appCommandFor(kEvent)).toEqual({ type: 'palette' });

    const pEvent = new KeyboardEvent('keydown', { key: 'P', ctrlKey: true, shiftKey: true });
    expect(appCommandFor(pEvent)).toEqual({ type: 'palette' });
  });

  it('Ctrl+, resolves to settings command', () => {
    const settingsEvent = new KeyboardEvent('keydown', { key: ',', ctrlKey: true });
    expect(appCommandFor(settingsEvent)).toEqual({ type: 'settings' });
  });

  it('Ctrl+Shift+L resolves to lock command', () => {
    const lockEvent = new KeyboardEvent('keydown', { key: 'L', ctrlKey: true, shiftKey: true });
    expect(appCommandFor(lockEvent)).toEqual({ type: 'lock' });
  });

  it('ignores plain typing when target is text input', () => {
    const input = document.createElement('input');
    expect(isTypingTarget(input)).toBe(true);
    expect(isTerminalTarget(input)).toBe(false);
  });
});
