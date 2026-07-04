<script lang="ts">
  import { onMount } from 'svelte';
  import { settings } from '$lib/stores/settings';
  import { setSetting, resetSettings, clearSessionHistory, traySupported, sessionsImportXlsx, sessionsExport, sessionsImport } from '$lib/ipc';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import SettingsToggle from '$lib/components/settings/SettingsToggle.svelte';
  import * as m from '$paraglide/messages.js';
  import { setLocale } from '$lib/locale.svelte.js';
  import { isMac, isLinux } from '$lib/utils/platform';

  // Language options: value stored in DB, label shown in native language.
  const LANGUAGES = [
    { value: 'auto', label: 'Auto' },
    { value: 'en', label: 'English' },
    { value: 'es', label: 'Español' },
    { value: 'fr', label: 'Français' },
    { value: 'de', label: 'Deutsch' },
    { value: 'ja', label: '日本語' },
    { value: 'zh', label: '中文' },
    { value: 'pt', label: 'Português' },
    { value: 'tr', label: 'Türkçe' },
  ];

  // On Linux, probe for libayatana-appindicator3 at runtime.  The tray section
  // is hidden entirely when the library is absent so users can't enable a
  // feature that would crash the app.  Non-Linux platforms always support tray.
  let trayAvailable = $state(!isLinux);
  onMount(async () => {
    if (isLinux) trayAvailable = await traySupported();
  });

  let localPort = $state(String($settings.websocket_port));
  $effect(() => {
    localPort = String($settings.websocket_port);
  });

  let langOpen = $state(false);
  let langEl: HTMLElement | undefined;

  const selectedLabel = $derived(
    LANGUAGES.find((l) => l.value === $settings.language)?.label ?? 'Auto'
  );

  async function selectLanguage(value: string) {
    langOpen = false;
    const updated = await setSetting('language', value);
    settings.set(updated);
    setLocale(value);
  }

  $effect(() => {
    if (!langOpen) return;
    function onOutside(e: MouseEvent) {
      if (langEl && !langEl.contains(e.target as Node)) langOpen = false;
    }
    window.addEventListener('mousedown', onOutside);
    return () => window.removeEventListener('mousedown', onOutside);
  });

  let confirmingReset = $state(false);
  let confirmingClear = $state(false);

  async function handleReset() {
    const updated = await resetSettings();
    settings.set(updated);
    confirmingReset = false;
  }

  async function handleClear() {
    await clearSessionHistory();
    confirmingClear = false;
  }

  async function handleExport() {
    const path = await save({
      filters: [{ name: 'Pomotroid JSON', extensions: ['json'] }],
      defaultPath: `pomotroid-export-${new Date().toISOString().slice(0, 10)}.json`
    });
    if (path) {
      try {
        const count = await sessionsExport(path);
        exportFeedback = `Exported ${count} session${count !== 1 ? 's' : ''}.`;
        setTimeout(() => (exportFeedback = ''), 4000);
      } catch (e) {
        console.error('Export failed', e);
        exportFeedback = `Export failed: ${e}`;
        setTimeout(() => (exportFeedback = ''), 6000);
      }
    }
  }

  async function handleJsonImport() {
    const file = await open({
      multiple: false,
      filters: [{ name: 'Pomotroid JSON', extensions: ['json'] }]
    });
    if (file) {
      try {
        const result = await sessionsImport(file as string);
        importFeedback = `Imported ${result.imported} session${result.imported !== 1 ? 's' : ''}${result.skipped > 0 ? `, skipped ${result.skipped} duplicate${result.skipped !== 1 ? 's' : ''}` : ''}.`;
        setTimeout(() => (importFeedback = ''), 5000);
      } catch (e) {
        console.error('Import failed', e);
        importFeedback = `Import failed: ${e}`;
        setTimeout(() => (importFeedback = ''), 6000);
      }
    }
  }

  async function handleImport() {
    const file = await open({
      multiple: false,
      filters: [{ name: 'Excel', extensions: ['xlsx'] }]
    });
    if (file) {
      try {
        const num = await sessionsImportXlsx(file as string);
        alert(`Successfully imported ${num} sessions!`);
      } catch (e) {
        console.error('Import failed', e);
        alert(`Failed to import: ${e}`);
      }
    }
  }

  let exportFeedback = $state('');
  let importFeedback = $state('');

  async function toggle(dbKey: string, current: boolean) {
    const updated = await setSetting(dbKey, current ? 'false' : 'true');
    settings.set(updated);
  }

  async function handlePortBlur() {
    const port = parseInt(localPort, 10);
    if (!isNaN(port) && port >= 1024 && port <= 65535) {
      const updated = await setSetting('websocket_port', String(port));
      settings.set(updated);
    } else {
      localPort = String($settings.websocket_port);
    }
  }
</script>

<div class="section">
  <div class="group-heading">Window Options</div>
  <SettingsToggle
    label="Window Controls (Minimize, Maximize, Close)"
    description="Show client side window decorations"
    checked={$settings.enable_window_controls}
    onclick={() => toggle('enable_window_controls', $settings.enable_window_controls)}
  />

  <div class="group-heading">{m.system_group_integrations()}</div>

  <SettingsToggle
    label={m.system_toggle_websocket()}
    description={m.system_toggle_websocket_desc({ port: $settings.websocket_port })}
    tooltip={m.tooltip_websocket()}
    checked={$settings.websocket_enabled}
    onclick={() => toggle('websocket_enabled', $settings.websocket_enabled)}
  />

  {#if $settings.websocket_enabled}
    <div class="row">
      <span class="label">{m.system_label_port()}</span>
      <input
        class="port-input"
        type="number"
        min="1024"
        max="65535"
        bind:value={localPort}
        onblur={handlePortBlur}
      />
    </div>
    <p class="note">
      Listens on <code>ws://127.0.0.1:{$settings.websocket_port}/ws</code>. Send
      <code>{'{ "type": "getState" }'}</code> to query the current timer state. Round changes are broadcast
      automatically.
    </p>
  {/if}

  <div class="group-heading">{m.system_group_language()}</div>

  <div class="lang-row">
    <div class="lang-dropdown" bind:this={langEl}>
      <button
        class="lang-trigger"
        class:open={langOpen}
        onclick={() => (langOpen = !langOpen)}
        onkeydown={(e) => {
          if (e.key === 'Escape') langOpen = false;
        }}
      >
        <span>{selectedLabel}</span>
        <svg
          class="chevron"
          class:open={langOpen}
          width="10"
          height="6"
          viewBox="0 0 10 6"
          aria-hidden="true"
        >
          <polyline
            points="0,0.5 5,5.5 10,0.5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>

      {#if langOpen}
        <ul class="lang-menu" role="listbox">
          {#each LANGUAGES as lang (lang.value)}
            <!-- svelte-ignore a11y_interactive_supports_focus -->
            <li
              class="lang-option"
              class:selected={$settings.language === lang.value}
              role="option"
              aria-selected={$settings.language === lang.value}
              onmousedown={() => selectLanguage(lang.value)}
            >
              {lang.label}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>

  <div class="group-heading">{m.system_group_updates()}</div>

  <SettingsToggle
    label={m.system_toggle_check_updates()}
    description={m.system_toggle_check_updates_desc()}
    checked={$settings.check_for_updates}
    onclick={() => toggle('check_for_updates', $settings.check_for_updates)}
  />

  <div class="group-heading">{m.advanced_group_logging()}</div>

  <SettingsToggle
    label={m.advanced_toggle_verbose_logging()}
    description={m.advanced_toggle_verbose_logging_desc()}
    tooltip={m.tooltip_verbose_logging()}
    checked={$settings.verbose_logging}
    onclick={() => toggle('verbose_logging', $settings.verbose_logging)}
  />

  {#if trayAvailable}
    <div class="group-heading">{m.system_group_tray()}</div>

    <!-- Show in System Tray: available on all platforms. -->
    <SettingsToggle
      label={m.system_toggle_show_tray()}
      description={m.system_toggle_show_tray_desc()}
      tooltip={isLinux ? m.system_tray_gnome_hint() : undefined}
      checked={$settings.tray_icon_enabled}
      onclick={() => toggle('tray_icon_enabled', $settings.tray_icon_enabled)}
    />

    {#if $settings.tray_icon_enabled}
      <!-- Minimize to Tray is Windows/Linux only: the macOS yellow traffic-light
           button routes to the Dock and cannot be intercepted by the app. -->
      {#if !isMac}
        <SettingsToggle
          label={m.system_toggle_min_tray()}
          description={m.system_toggle_min_tray_desc()}
          checked={$settings.min_to_tray}
          onclick={() => toggle('min_to_tray', $settings.min_to_tray)}
        />
      {/if}
      <!-- Close to Tray is available on all platforms: the CloseRequested event
           fires on macOS (red button / Cmd+W) and can be intercepted. -->
      <SettingsToggle
        label={m.system_toggle_close_tray()}
        description={m.system_toggle_close_tray_desc()}
        checked={$settings.min_to_tray_on_close}
        onclick={() => toggle('min_to_tray_on_close', $settings.min_to_tray_on_close)}
      />
    {/if}
  {/if}

  <div class="group-heading">{m.system_group_window()}</div>

  <SettingsToggle
    label={m.system_toggle_aot()}
    description={m.system_toggle_aot_desc()}
    checked={$settings.always_on_top}
    onclick={() => toggle('always_on_top', $settings.always_on_top)}
  />
  {#if $settings.always_on_top}
    <SettingsToggle
      label={m.system_toggle_break_aot()}
      description={m.system_toggle_break_aot_desc()}
      checked={$settings.break_always_on_top}
      onclick={() => toggle('break_always_on_top', $settings.break_always_on_top)}
    />
  {/if}

  <div class="group-heading">History</div>
  <SettingsToggle
    label="Show breaks in history"
    description="Include short and long breaks in your session history"
    checked={$settings.history_show_breaks}
    onclick={() => toggle('history_show_breaks', $settings.history_show_breaks)}
  />

  <div class="group-heading">{m.system_group_data()}</div>

  <div class="data-group">
    <!-- Export -->
    <div class="data-action-row">
      <div class="data-action-info">
        <span class="data-action-label">Export Sessions</span>
        <span class="data-action-desc">Save all sessions to a .pomotroid.json file</span>
      </div>
      <button class="data-action-btn" onclick={handleExport}>Export</button>
    </div>
    {#if exportFeedback}
      <p class="data-feedback success">{exportFeedback}</p>
    {/if}

    <!-- Import -->
    <div class="data-action-row">
      <div class="data-action-info">
        <span class="data-action-label">Import Sessions</span>
        <span class="data-action-desc">Restore from a .pomotroid.json export file</span>
      </div>
      <button class="data-action-btn" onclick={handleJsonImport}>Import</button>
    </div>
    {#if importFeedback}
      <p class="data-feedback success">{importFeedback}</p>
    {/if}

    <!-- Clear history -->
    {#if !confirmingClear}
      <button class="data-row" onclick={() => (confirmingClear = true)}>
        <span>Clear Session History</span>
      </button>
    {:else}
      <div class="confirm-row">
        <span class="confirm-label"
          >This will permanently delete all session history. This cannot be undone.</span
        >
        <div class="confirm-actions">
          <button class="confirm-cancel" onclick={() => (confirmingClear = false)}>Cancel</button>
          <button class="confirm-destructive" onclick={handleClear}>Clear</button>
        </div>
      </div>
    {/if}

    <!-- Reset settings -->
    {#if !confirmingReset}
      <button class="data-row" onclick={() => (confirmingReset = true)}>
        <span>{m.about_reset_all()}</span>
      </button>
    {:else}
      <div class="confirm-row">
        <span class="confirm-label">{m.about_reset_confirm()}</span>
        <div class="confirm-actions">
          <button class="confirm-cancel" onclick={() => (confirmingReset = false)}>Cancel</button>
          <button class="confirm-destructive" onclick={handleReset}>Reset</button>
        </div>
      </div>
    {/if}

    <!-- Legacy xlsx import -->
    <div class="legacy-divider">
      <span>Legacy</span>
    </div>
    <button class="data-row deprecated" onclick={handleImport}>
      <span>Import from spreadsheet (.xlsx)</span>
      <span class="deprecated-tag">Deprecated</span>
    </button>
  </div>
</div>

<style>
  .section {
    display: flex;
    flex-direction: column;
    padding-bottom: 20px;
  }

  .group-heading {
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.6;
    margin: 0;
    padding: 16px 20px 6px;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 20px;
    border-bottom: 1px solid var(--color-separator);
  }

  .label {
    font-size: 0.85rem;
    color: var(--color-foreground);
    letter-spacing: 0.02em;
  }

  .port-input {
    background: var(--color-hover);
    border: 1px solid color-mix(in oklch, var(--color-foreground) 18%, transparent);
    border-radius: 4px;
    color: var(--color-foreground);
    font-size: 0.85rem;
    font-family: monospace;
    padding: 4px 10px;
    width: 90px;
    text-align: right;
    outline: none;
    transition: border-color 0.15s;
  }

  .port-input:focus {
    border-color: var(--color-accent);
  }

  .note {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.65;
    padding: 16px 20px;
    line-height: 1.6;
  }

  code {
    font-family: monospace;
    font-size: 0.75em;
    background: color-mix(in oklch, var(--color-foreground) 12%, transparent);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .lang-row {
    padding: 10px 20px;
    border-bottom: 1px solid var(--color-separator);
  }

  .lang-dropdown {
    position: relative;
  }

  .lang-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: var(--color-hover);
    border: 1px solid color-mix(in oklch, var(--color-foreground) 18%, transparent);
    border-radius: 4px;
    color: var(--color-foreground);
    font-size: 0.85rem;
    padding: 6px 10px;
    cursor: pointer;
    outline: none;
    transition: border-color 0.15s;
    text-align: left;
  }

  .lang-trigger:hover,
  .lang-trigger:focus,
  .lang-trigger.open {
    border-color: var(--color-accent);
  }

  .chevron {
    flex-shrink: 0;
    margin-left: 6px;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.7;
    transition: transform 0.15s;
  }

  .chevron.open {
    transform: rotate(180deg);
  }

  .lang-menu {
    position: absolute;
    top: calc(100% + 3px);
    left: 0;
    right: 0;
    background: var(--color-background-light);
    border: 1px solid color-mix(in oklch, var(--color-foreground) 15%, transparent);
    border-radius: 4px;
    box-shadow: 0 4px 16px color-mix(in oklch, black 20%, transparent);
    z-index: 200;
    list-style: none;
    margin: 0;
    padding: 3px 0;
  }

  .lang-option {
    padding: 7px 12px;
    font-size: 0.85rem;
    color: var(--color-foreground);
    cursor: pointer;
    transition: background 0.1s;
  }

  .lang-option:hover {
    background: var(--color-hover);
  }

  .lang-option.selected {
    color: var(--color-accent);
    font-weight: 500;
  }

  .data-group {
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    overflow: hidden;
    margin: 6px 20px 0;
  }

  .data-row {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 12px 16px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--color-separator);
    cursor: pointer;
    color: var(--color-foreground-darker, var(--color-foreground));
    font-size: 0.85rem;
    letter-spacing: 0.02em;
    text-align: left;
    transition:
      background 0.12s,
      color 0.12s;
  }

  .data-row:last-child {
    border-bottom: none;
  }

  .data-row:hover {
    background: var(--color-hover);
    color: color-mix(in oklch, var(--color-focus-round) 80%, var(--color-foreground));
  }

  .confirm-row {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--color-separator);
  }

  .confirm-row:last-child {
    border-bottom: none;
  }

  .confirm-label {
    font-size: 0.8rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.8;
  }

  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .confirm-cancel,
  .confirm-destructive {
    background: none;
    border: 1px solid color-mix(in oklch, var(--color-foreground) 18%, transparent);
    border-radius: 4px;
    font-size: 0.8rem;
    padding: 5px 14px;
    cursor: pointer;
    transition:
      border-color 0.15s,
      color 0.15s;
  }

  .confirm-cancel {
    color: var(--color-foreground-darker, var(--color-foreground));
  }

  .confirm-cancel:hover {
    border-color: color-mix(in oklch, var(--color-foreground) 40%, transparent);
    color: var(--color-foreground);
  }

  .confirm-destructive {
    color: var(--color-accent);
    border-color: color-mix(in oklch, var(--color-accent) 40%, transparent);
  }

  .confirm-destructive:hover {
    background: color-mix(in oklch, var(--color-accent) 10%, transparent);
    border-color: var(--color-accent);
  }

  /* Export / Import action rows */
  .data-action-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 11px 16px;
    border-bottom: 1px solid var(--color-separator);
    gap: 12px;
  }

  .data-action-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .data-action-label {
    font-size: 0.85rem;
    color: var(--color-foreground);
    font-weight: 500;
    letter-spacing: 0.02em;
  }

  .data-action-desc {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.6;
  }

  .data-action-btn {
    flex-shrink: 0;
    background: color-mix(in oklch, var(--color-accent) 12%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-accent) 35%, transparent);
    border-radius: 5px;
    color: var(--color-accent);
    font-size: 0.8rem;
    font-weight: 500;
    padding: 5px 14px;
    cursor: pointer;
    letter-spacing: 0.03em;
    transition:
      background 0.15s,
      border-color 0.15s;
  }

  .data-action-btn:hover {
    background: color-mix(in oklch, var(--color-accent) 22%, transparent);
    border-color: var(--color-accent);
  }

  .data-feedback {
    font-size: 0.78rem;
    padding: 6px 16px 8px;
    border-bottom: 1px solid var(--color-separator);
    margin: 0;
  }

  .data-feedback.success {
    color: var(--color-short-round, #3baf82);
  }

  /* Legacy divider */
  .legacy-divider {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px 4px;
    border-top: 1px solid var(--color-separator);
  }

  .legacy-divider span {
    font-size: 0.65rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.45;
  }

  /* Deprecated row styling */
  .data-row.deprecated {
    justify-content: space-between;
    opacity: 0.65;
  }

  .deprecated-tag {
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: color-mix(in oklch, var(--color-foreground) 50%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-foreground) 20%, transparent);
    border-radius: 3px;
    padding: 1px 6px;
    flex-shrink: 0;
  }
</style>
