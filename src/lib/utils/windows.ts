import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { isMac } from './platform';

export async function openSettingsWindow(section?: string) {
  const existing = await WebviewWindow.getByLabel('settings');
  if (existing) {
    await existing.show();
    await existing.setFocus();
    if (section) {
      await existing.emit('settings:navigate', section);
    }
    return;
  }
  const url = section ? `/settings?section=${encodeURIComponent(section)}` : '/settings';
  new WebviewWindow('settings', {
    url,
    title: 'Pomotroid — Settings',
    width: 720,
    height: 520,
    minWidth: 600,
    minHeight: 400,
    // On macOS: native decorations + overlay titlebar for rounded corners and
    // traffic light buttons. On other platforms: custom decorations-free window.
    decorations: isMac,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    titleBarStyle: isMac ? ('Overlay' as any) : undefined,
    hiddenTitle: isMac ? true : undefined,
    resizable: true,
    visible: false,
  });
}

export async function openPlannerWindow() {
  const existing = await WebviewWindow.getByLabel('planner');
  if (existing) {
    await existing.show();
    await existing.setFocus();
    return;
  }
  new WebviewWindow('planner', {
    url: '/planner',
    title: 'Pomotroid — Planner',
    width: 960,
    height: 640,
    minWidth: 680,
    minHeight: 480,
    decorations: isMac,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    titleBarStyle: isMac ? ('Overlay' as any) : undefined,
    hiddenTitle: isMac ? true : undefined,
    resizable: true,
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
