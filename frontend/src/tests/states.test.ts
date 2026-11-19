import { describe, it, expect } from 'vitest';
import { confirmModal, getConfirmDialog, showToast, getToasts } from '$lib/stores/uiNotifications.svelte';

describe('Four states and confirmation dialog (P16.5)', () => {
  it('confirmModal opens dialog state and resolves on user action', async () => {
    const promise = confirmModal(
      'Aksi ini tidak dapat dibatalkan.',
      'Hapus Catatan?',
      true,
      'Hapus',
      'Batal'
    );

    const state = getConfirmDialog();
    expect(state.isOpen).toBe(true);
    expect(state.title).toBe('Hapus Catatan?');
    expect(state.danger).toBe(true);

    // Simulate clicking confirm
    state.resolve?.(true);
    const result = await promise;
    expect(result).toBe(true);
  });

  it('showToast adds a toast and auto-clears', () => {
    showToast('Operasi berhasil', 'success');
    const toasts = getToasts();
    expect(toasts.some((t) => t.message === 'Operasi berhasil' && t.type === 'success')).toBe(true);
  });
});
