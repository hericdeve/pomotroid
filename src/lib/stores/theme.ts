// Theme store.
// Applies theme colors to CSS custom properties on :root.

import type { Theme } from '$lib/types';

/** Apply a theme's colors to the document root CSS custom properties.
 *  Theme keys already include the `--` prefix (e.g. "--color-background"). */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(theme.colors)) {
    root.style.setProperty(key, value);
  }

  // Set color-scheme based on background luminance to fix native controls (select, date picker, scrollbars)
  const bg = theme.colors['--color-background'];
  if (bg && bg.startsWith('#')) {
    const hex = bg.replace('#', '');
    if (hex.length === 6) {
      const r = parseInt(hex.substring(0, 2), 16);
      const g = parseInt(hex.substring(2, 4), 16);
      const b = parseInt(hex.substring(4, 6), 16);
      const luminance = 0.299 * r + 0.587 * g + 0.114 * b;
      root.style.setProperty('color-scheme', luminance < 128 ? 'dark' : 'light');
    }
  }
}
