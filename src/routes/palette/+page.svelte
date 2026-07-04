<script lang="ts">
  import '../../app.css';
  import { onMount } from 'svelte';
  import { getSettings, getThemes, onSettingsChanged, onThemesChanged } from '$lib/ipc';
  import { settings } from '$lib/stores/settings';
  import { applyTheme } from '$lib/stores/theme';
  import { resolveThemeName } from '$lib/utils/theme';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import CommandPalette from '$lib/components/CommandPalette.svelte';

  onMount(() => {
    const cleanups: UnlistenFn[] = [];

    (async () => {
      const s = await getSettings();
      settings.set(s);

      const themes = await getThemes();
      const osDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      const activeTheme = themes.find((t) => t.name === resolveThemeName(s, osDark)) ?? themes[0];
      if (activeTheme) applyTheme(activeTheme);

      await getCurrentWebviewWindow().show();

      cleanups.push(
        await onSettingsChanged(async (newSettings) => {
          settings.set(newSettings);
          const osDark2 = window.matchMedia('(prefers-color-scheme: dark)').matches;
          const theme = themes.find((t) => t.name === resolveThemeName(newSettings, osDark2));
          if (theme) applyTheme(theme);
        }),
        await onThemesChanged(async (newThemes) => {
          const osDark2 = window.matchMedia('(prefers-color-scheme: dark)').matches;
          const theme = newThemes.find((t) => t.name === resolveThemeName(s, osDark2));
          if (theme) applyTheme(theme);
        }),
      );
    })();

    return () => cleanups.forEach((fn) => fn());
  });
</script>

<CommandPalette />

<style>
  :global(body) {
    background-color: transparent !important;
  }
</style>
