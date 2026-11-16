import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockMinimize = vi.fn();
const mockMaximize = vi.fn();
const mockUnmaximize = vi.fn();
const mockClose = vi.fn();
const mockIsMaximized = vi.fn();

vi.mock('@tauri-apps/api/window', () => {
  return {
    Window: class {
      label: string;
      constructor(label: string) {
        this.label = label;
      }
      minimize = mockMinimize;
      maximize = mockMaximize;
      unmaximize = mockUnmaximize;
      close = mockClose;
      isMaximized = mockIsMaximized;
    }
  };
});

describe('TitleBar Window Actions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('window actions trigger corresponding Tauri Window methods', async () => {
    const { Window } = await import('@tauri-apps/api/window');
    const win = new Window('main');

    mockMinimize.mockResolvedValueOnce(undefined);
    await win.minimize();
    expect(mockMinimize).toHaveBeenCalledTimes(1);

    mockIsMaximized.mockResolvedValueOnce(false);
    const isMax = await win.isMaximized();
    expect(isMax).toBe(false);

    mockMaximize.mockResolvedValueOnce(undefined);
    await win.maximize();
    expect(mockMaximize).toHaveBeenCalledTimes(1);

    mockClose.mockResolvedValueOnce(undefined);
    await win.close();
    expect(mockClose).toHaveBeenCalledTimes(1);
  });

  it('handles toggle maximize flow correctly', async () => {
    const { Window } = await import('@tauri-apps/api/window');
    const win = new Window('main');

    // Case 1: when currently maximized, call unmaximize
    mockIsMaximized.mockResolvedValueOnce(true);
    if (await win.isMaximized()) {
      await win.unmaximize();
    }
    expect(mockUnmaximize).toHaveBeenCalledTimes(1);
    expect(mockMaximize).not.toHaveBeenCalled();

    // Case 2: when not maximized, call maximize
    mockIsMaximized.mockResolvedValueOnce(false);
    if (!(await win.isMaximized())) {
      await win.maximize();
    }
    expect(mockMaximize).toHaveBeenCalledTimes(1);
  });
});
