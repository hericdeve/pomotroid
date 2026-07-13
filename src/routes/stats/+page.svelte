<script lang="ts">
  import '../../app.css';
  import { onMount } from 'svelte';
  import {
    getSettings,
    getThemes,
    onSettingsChanged,
    onThemesChanged,
    onRoundChange,
    onSessionsCleared,
    statsGetDetailed,
    statsGetHeatmap,
  } from '$lib/ipc';
  import { settings } from '$lib/stores/settings';
  import { applyTheme } from '$lib/stores/theme';
  import { setLocale } from '$lib/locale.svelte.js';
  import { resolveThemeName } from '$lib/utils/theme';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { isMac } from '$lib/utils/platform';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import type { DetailedStats, HeatmapStats } from '$lib/types';
  import * as m from '$paraglide/messages.js';
  import { info, error as logError } from '@tauri-apps/plugin-log';

  import DailyView from '$lib/components/stats/DailyView.svelte';
  import WeeklyView from '$lib/components/stats/WeeklyView.svelte';
  import YearlyView from '$lib/components/stats/YearlyView.svelte';
  import HistoryView from '$lib/components/stats/HistoryView.svelte';
  import InsightsView from '$lib/components/stats/InsightsView.svelte';
  import ManualEntryModal from '$lib/components/ManualEntryModal.svelte';
  import SessionTagModal from '$lib/components/SessionTagModal.svelte';
  import SessionsListModal from '$lib/components/stats/SessionsListModal.svelte';
  import SubjectsView from '$lib/components/stats/SubjectsView.svelte';
  import PlanningView from '$lib/components/stats/PlanningView.svelte';
  import ComparisonsView from '$lib/components/stats/ComparisonsView.svelte';

  type Tab = 'today' | 'week' | 'alltime' | 'history' | 'insights' | 'subjects' | 'planning' | 'comparisons';

  let activeTab = $state<Tab>('today');
  let detailed = $state<DetailedStats | null>(null);
  let heatmap = $state<HeatmapStats | null>(null);
  let heatmapLoaded = $state(false);
  let showManualEntry = $state(false);
  let editingSessionId = $state<number | null>(null);
  let editingStudySessionId = $state<number | null>(null);
  let refreshTrigger = $state(0);
  
  let listModalTimeRange = $state<{ start: number, end: number, label: string } | null>(null);

  async function loadData() {
    detailed = await statsGetDetailed();
    if (heatmapLoaded) heatmap = await statsGetHeatmap();
  }

  async function switchTab(tab: Tab) {
    activeTab = tab;
    if ((tab === 'alltime' || tab === 'comparisons') && !heatmapLoaded) {
      try {
        heatmap = await statsGetHeatmap();
        heatmapLoaded = true;
      } catch (e) {
        await logError(`[stats] failed to load heatmap: ${e}`);
      }
    }
  }

  function close() {
    getCurrentWebviewWindow().close();
  }

  onMount(() => {
    const cleanups: UnlistenFn[] = [];

    (async () => {
      try {
        const s = await getSettings();
        settings.set(s);
        setLocale(s.language);
        await info(`[stats] settings loaded, locale=${s.language}`);

        const themes = await getThemes();
        const osDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        const activeTheme = themes.find((t) => t.name === resolveThemeName(s, osDark)) ?? themes[0];
        if (activeTheme) applyTheme(activeTheme);

        // Show window immediately after theme is applied
        await getCurrentWebviewWindow().show();

        detailed = await statsGetDetailed();
        await info(`[stats] initialized, theme=${activeTheme?.name ?? 'none'}`);
      } catch (e) {
        await logError(`[stats] initialization failed: ${e}`);
        throw e;
      }

      cleanups.push(
        await onRoundChange(async () => {
          try {
            detailed = await statsGetDetailed();
            if (heatmapLoaded) heatmap = await statsGetHeatmap();
          } catch (e) {
            await logError(`[stats] failed to refresh stats after round change: ${e}`);
          }
        }),
        await onSessionsCleared(async () => {
          try {
            detailed = await statsGetDetailed();
            if (heatmapLoaded) heatmap = await statsGetHeatmap();
          } catch (e) {
            await logError(`[stats] failed to refresh stats after session clear: ${e}`);
          }
        }),
        await onSettingsChanged(async (updated) => {
          const prev = {
            mode: $settings.theme_mode,
            light: $settings.theme_light,
            dark: $settings.theme_dark,
            language: $settings.language,
          };
          settings.set(updated);
          if (updated.language !== prev.language) {
            setLocale(updated.language);
          }
          if (
            updated.theme_mode !== prev.mode ||
            updated.theme_light !== prev.light ||
            updated.theme_dark !== prev.dark
          ) {
            const allThemes = await getThemes();
            const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
            const t = allThemes.find((th) => th.name === resolveThemeName(updated, dark));
            if (t) applyTheme(t);
          }
        }),
        await onThemesChanged((updated) => {
          const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
          const current =
            updated.find((t) => t.name === resolveThemeName($settings, dark)) ?? updated[0];
          if (current) applyTheme(current);
        })
      );
    })();

    return () => {
      for (const fn of cleanups) fn();
    };
  });
</script>

<div class="window">
  <!-- Titlebar -->
  <nav class="titlebar" class:macos={isMac} data-tauri-drag-region>
    <span class="titlebar-label">{m.stats_title()}</span>
    {#if !isMac}
      <button class="btn-close" onclick={close} aria-label="Close">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <line
            x1="1"
            y1="1"
            x2="11"
            y2="11"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
          <line
            x1="11"
            y1="1"
            x2="1"
            y2="11"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      </button>
    {/if}
  </nav>

  <div class="tabs-container">
    <div class="tabs">
      <button class="tab" class:active={activeTab === 'today'} onclick={() => switchTab('today')}
        >{m.stats_tab_today()}</button
      >
      <button class="tab" class:active={activeTab === 'week'} onclick={() => switchTab('week')}
        >{m.stats_tab_week()}</button
      >
      <button class="tab" class:active={activeTab === 'alltime'} onclick={() => switchTab('alltime')}
        >{m.stats_tab_alltime()}</button
      >
      <button class="tab" class:active={activeTab === 'comparisons'} onclick={() => switchTab('comparisons')}
        >Comparisons</button
      >
      <button class="tab" class:active={activeTab === 'planning'} onclick={() => switchTab('planning')}
        >Planning</button
      >
      <button class="tab" class:active={activeTab === 'subjects'} onclick={() => switchTab('subjects')}
        >Subjects</button
      >
      <button class="tab" class:active={activeTab === 'history'} onclick={() => switchTab('history')}
        >History</button
      >
      <button class="tab" class:active={activeTab === 'insights'} onclick={() => switchTab('insights')}
        >Insights</button
      >
    </div>
    <button class="btn-manual" onclick={() => showManualEntry = true} title="Manual Entry" aria-label="Manual Entry">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M5 12h14"/>
        <path d="M12 5v14"/>
      </svg>
    </button>
  </div>

  <!-- Content -->
  <div class="content">
    {#key refreshTrigger}
      {#if activeTab === 'today'}
        <DailyView today={detailed?.today ?? null} onBarClick={(r) => listModalTimeRange = r} />
      {:else if activeTab === 'week'}
        <WeeklyView week={detailed?.week ?? null} streak={detailed?.streak ?? null} onBarClick={(r) => listModalTimeRange = r} />
      {:else if activeTab === 'alltime'}
        <YearlyView {heatmap} onBarClick={(r) => listModalTimeRange = r} />
      {:else if activeTab === 'comparisons'}
        <ComparisonsView {heatmap} />
      {:else if activeTab === 'history'}
        <HistoryView 
          onEditSession={(id) => editingSessionId = id} 
          onEditStudySession={(id) => editingStudySessionId = id} 
        />
      {:else if activeTab === 'insights'}
        <InsightsView />
      {:else if activeTab === 'subjects'}
        <SubjectsView />
      {:else}
        <PlanningView />
      {/if}
    {/key}
  </div>
</div>

{#if showManualEntry}
  <ManualEntryModal onclose={() => {
    showManualEntry = false;
    refreshTrigger++;
    loadData();
  }} />
{/if}

{#if listModalTimeRange !== null}
  {#key refreshTrigger}
    <SessionsListModal
      dateFrom={listModalTimeRange.start}
      dateTo={listModalTimeRange.end}
      label={listModalTimeRange.label}
      onClose={() => listModalTimeRange = null}
      onEditSession={(id) => {
        listModalTimeRange = null;
        editingSessionId = id;
      }}
    />
  {/key}
{/if}

{#if editingSessionId !== null}
  <SessionTagModal 
    sessionId={editingSessionId} 
    allowDelete={true}
    onClose={() => {
      editingSessionId = null;
      refreshTrigger++;
      loadData();
    }} 
    onDeleted={() => {
      editingSessionId = null;
      refreshTrigger++;
      loadData();
    }}
  />
{/if}

{#if editingStudySessionId !== null}
  <SessionTagModal 
    sessionId={null}
    studySessionId={editingStudySessionId} 
    allowDelete={true}
    onClose={() => {
      editingStudySessionId = null;
      refreshTrigger++;
      loadData();
    }} 
    onDeleted={() => {
      editingStudySessionId = null;
      refreshTrigger++;
      loadData();
    }}
  />
{/if}

<style>
  .window {
    display: flex;
    flex-direction: column;
    height: 100dvh;
    background: var(--color-background);
    color: var(--color-foreground);
    animation: app-fade-in 0.18s ease;
    overflow: hidden;
    cursor: default;
  }

  /* ── Titlebar ──────────────────────────────────────────── */
  .titlebar {
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    flex-shrink: 0;
    border-bottom: 1px solid var(--color-separator);
  }

  .macos {
    padding-left: 72px;
  }

  .titlebar-label {
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-foreground-darker);
    pointer-events: none;
  }

  .btn-close {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-foreground-darker);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 4px;
    transition:
      color 0.15s,
      background 0.15s;
  }

  .btn-close:hover {
    color: var(--color-background);
    background: var(--color-focus-round);
  }

  /* ── Tabs ──────────────────────────────────────────────── */
  .tabs-container {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--color-separator);
    flex-shrink: 0;
    padding: 0 24px;
  }

  .tabs {
    display: flex;
    gap: 0;
  }

  .tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    padding: 10px 20px;
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-foreground-darker);
    cursor: pointer;
    transition:
      color 0.15s,
      border-color 0.15s;
  }

  .tab:hover {
    color: var(--color-foreground);
  }

  .tab.active {
    color: var(--color-focus-round);
    border-bottom-color: var(--color-focus-round);
  }

  .btn-manual {
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
    border: none;
    color: var(--color-foreground);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 4px;
    cursor: pointer;
    transition: var(--transition-default);
  }

  .btn-manual:hover {
    background: color-mix(in oklch, var(--color-foreground) 15%, transparent);
  }

  /* ── Content ───────────────────────────────────────────── */
  .content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .content > :global(*) {
    width: 100%;
    max-width: 900px;
    flex: 1;
    min-height: 0;
  }
</style>
