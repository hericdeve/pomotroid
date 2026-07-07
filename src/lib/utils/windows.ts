import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { isMac } from './platform';

export async function openSettingsWindow() {
  const existing = await WebviewWindow.getByLabel('settings');
  if (existing) {
    await existing.show();
    await existing.setFocus();
    return;
  }
  new WebviewWindow('settings', {
    url: '/settings',
    title: 'Pomotroid — Settings',
    width: 720,
    height: 520,
    // On macOS: native decorations + overlay titlebar for rounded corners and
    // traffic light buttons. On other platforms: custom decorations-free window.
    decorations: isMac,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    titleBarStyle: isMac ? ('Overlay' as any) : undefined,
    hiddenTitle: isMac ? true : undefined,
    resizable: false,
    visible: false,
  });
}

export async function openStatsWindow() {
  const existing = await WebviewWindow.getByLabel('stats');
  if (existing) {
    await existing.show();
    await existing.setFocus();
    return;
  }
  new WebviewWindow('stats', {
    url: '/stats',
    title: 'Pomotroid — Statistics',
    width: 840,
    height: 520,
    minWidth: 600,
    minHeight: 400,
    decorations: isMac,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    titleBarStyle: isMac ? ('Overlay' as any) : undefined,
    hiddenTitle: isMac ? true : undefined,
    resizable: true,
    visible: false,
  });
}
