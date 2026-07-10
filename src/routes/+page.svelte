<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import Titlebar from '$lib/components/Titlebar.svelte';
  import Timer from '$lib/components/Timer.svelte';
  import SessionTagModal from '$lib/components/SessionTagModal.svelte';
  import SessionGoalModal from '$lib/components/SessionGoalModal.svelte';
  import { showTagModal, pendingTags } from '$lib/stores/pendingTags';
  import { showGoalModal, sessionGoalRounds } from '$lib/stores/sessionGoal';
  import { timerState } from '$lib/stores/timer';
  import { getSettings, getThemes, onSettingsChanged, onThemesChanged, timerToggle, timerSkip, timerRestartRound, scheduleGetAll, studySessionUpdate } from '$lib/ipc';
  import type { ScheduledBlock } from '$lib/types';
  import { settings } from '$lib/stores/settings';
  import { applyTheme } from '$lib/stores/theme';
  import { resolveThemeName } from '$lib/utils/theme';
  import { isMac } from '$lib/utils/platform';
  import { setLocale } from '$lib/locale.svelte.js';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { get } from 'svelte/store';
  import { info, error as logError } from '@tauri-apps/plugin-log';
  import { createLocalShortcutHandler } from '$lib/utils/localShortcuts';

  // Local shortcut state — volume and fullscreen tracked separately so the
  // handler can read current values without waiting for settings:changed round-trip.
  let localVolume = $state(1.0);
  let preMuteVolume = $state(0.5);
  let isFullscreen = $state(false);

  // Base window dimensions (natural/default size).
  const BASE_W = 360;
  const BASE_H = 478;
  const TITLEBAR_H = 40;

  // Compact mode: when either dimension drops below this threshold,
  // hide non-essential elements (footer, label, play/pause) to show
  // only the timer dial — like an Apple Watch face.
  const COMPACT_THRESHOLD = 300;

  let uiScale = $state(1.0);
  let isCompact = $state(false);

  // Extra bottom padding added to <main> in compact mode.  Shifts the
  // dial upward so the whitespace sits at the bottom rather than being
  // split equally — compensates for the visual weight of the titlebar.
  const COMPACT_BOTTOM_PAD = 48;

  // Smart Schedule Detection state
  let scheduleBlocks = $state<ScheduledBlock[]>([]);
  let activeScheduledSubject = $state<string | null>(null);

  function checkSchedule() {
    const now = new Date();
    // JS getDay(): 0=Sun, 1=Mon, ..., 6=Sat
    // Our DB format: 0=Mon, ..., 6=Sun
    const jsDay = now.getDay();
    const currentDay = jsDay === 0 ? 6 : jsDay - 1;
    const currentMinute = now.getHours() * 60 + now.getMinutes();

    const activeBlock = scheduleBlocks.find(b => 
      b.day_of_week === currentDay && 
      currentMinute >= b.start_minute && 
      currentMinute < b.end_minute
    );
    activeScheduledSubject = activeBlock ? activeBlock.subject : null;
  }

  function handleStartScheduled() {
    if (activeScheduledSubject) {
      pendingTags.set({
        subject: activeScheduledSubject,
        subject_topic: '',
        study_type: '',
        notes: ''
      });
      if (!$timerState.is_running) {
        timerToggle();
      }
    }
  }

  $effect(() => {
    function update() {
      const w = window.innerWidth;
      const h = window.innerHeight;
      isCompact = w < COMPACT_THRESHOLD || h < COMPACT_THRESHOLD;
      if (isCompact) {
        // Scale so the dial fills the available space, reserving
        // COMPACT_BOTTOM_PAD px for the intentional bottom whitespace.
        const available = Math.min(w - 16, h - TITLEBAR_H - 16 - COMPACT_BOTTOM_PAD);
        uiScale = Math.max(0.4, Math.min(available / 220, 4));
      } else {
        // Scale proportionally to the base window dimensions.
        uiScale = Math.max(0.5, Math.min(w / BASE_W, (h - TITLEBAR_H) / (BASE_H - TITLEBAR_H), 4));
      }
    }
    update();
    window.addEventListener('resize', update);
    return () => window.removeEventListener('resize', update);
  });

  let prevActiveStudySessionId = $state<number | null>(null);

  $effect(() => {
    const currentId = $timerState.active_study_session_id;
    if (currentId !== prevActiveStudySessionId) {
      if (currentId !== null && prevActiveStudySessionId === null) {
        // Session just started, push pending tags and goal to backend
        studySessionUpdate(currentId, {
          subject: $pendingTags.subject || undefined,
          subject_topic: $pendingTags.subject_topic || undefined,
          study_type: $pendingTags.study_type || undefined,
          notes: $pendingTags.notes || undefined,
          goal_rounds: $sessionGoalRounds,
        }).catch(console.error);
      }
      prevActiveStudySessionId = currentId;
    }
  });

  async function startResize(direction: string) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await getCurrentWebviewWindow().startResizeDragging(direction as any);
  }

  onMount(() => {
    const cleanups: UnlistenFn[] = [];

    // Mount local keyboard shortcut handler.
    const shortcutHandler = createLocalShortcutHandler({
      getSettings: () => $settings,
      getVolume: () => localVolume,
      setVolume: (v) => {
        localVolume = v;
      },
      getPreMuteVolume: () => preMuteVolume,
      setPreMuteVolume: (v) => {
        preMuteVolume = v;
      },
      getFullscreen: () => isFullscreen,
      setFullscreen: (v) => {
        isFullscreen = v;
      },
    });
    document.addEventListener('keydown', shortcutHandler);
    cleanups.push(() => document.removeEventListener('keydown', shortcutHandler));

    (async () => {
      try {
        // Load settings from backend.
        const s = await getSettings();
        settings.set(s);
        localVolume = s.volume;

        // Apply the stored locale on mount.
        setLocale(s.language);
        await info(`[main] settings loaded, locale=${s.language}`);

        // Load and apply the active theme using OS color scheme.
        const themes = await getThemes();
        const osDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        const active = themes.find((t) => t.name === resolveThemeName(s, osDark)) ?? themes[0];
        if (active) applyTheme(active);
        
        // Load schedule and start interval checking
        scheduleBlocks = await scheduleGetAll();
        checkSchedule();
        const intervalId = setInterval(() => {
          checkSchedule();
          // Optionally refresh schedule every minute in case Stats updated it
          scheduleGetAll().then(blocks => {
            scheduleBlocks = blocks;
            checkSchedule();
          }).catch(console.error);
        }, 60_000);
        cleanups.push(() => clearInterval(intervalId));

        await getCurrentWebviewWindow().show();
        await info(`[main] initialized, theme=${active?.name ?? 'none'}`);
      } catch (e) {
        await logError(`[main] initialization failed: ${e}`);
        throw e;
      }

      // Live OS color scheme changes — re-resolve only in auto mode.
      const mq = window.matchMedia('(prefers-color-scheme: dark)');
      const mqListener = async (e: MediaQueryListEvent) => {
        if ($settings.theme_mode !== 'auto') return;
        const allThemes = await getThemes();
        const t = allThemes.find((th) => th.name === resolveThemeName($settings, e.matches));
        if (t) applyTheme(t);
      };
      mq.addEventListener('change', mqListener);
      cleanups.push(() => mq.removeEventListener('change', mqListener));

      // Keep settings store in sync with backend changes.
      cleanups.push(
        await onSettingsChanged(async (updated) => {
          const prevMode = $settings.theme_mode;
          const prevLight = $settings.theme_light;
          const prevDark = $settings.theme_dark;
          const prevLanguage = $settings.language;
          settings.set(updated);
          localVolume = updated.volume;
          if (updated.language !== prevLanguage) {
            setLocale(updated.language);
          }
          if (
            updated.theme_mode !== prevMode ||
            updated.theme_light !== prevLight ||
            updated.theme_dark !== prevDark
          ) {
            const allThemes = await getThemes();
            const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
            const t = allThemes.find((th) => th.name === resolveThemeName(updated, dark));
            if (t) applyTheme(t);
          }
        }),
        // Re-apply theme when custom themes are hot-reloaded.
        await onThemesChanged((updated) => {
          const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
          const current =
            updated.find((t) => t.name === resolveThemeName($settings, dark)) ?? updated[0];
          if (current) applyTheme(current);
        })
      );
      // Listen for palette:start events from the command palette window.
      cleanups.push(
        await listen<{ subject: string; subject_topic: string; study_type: string; notes: string }>(
          'palette:start',
          async (event) => {
            const p = event.payload;
            pendingTags.set({
              subject: p.subject || '',
              subject_topic: p.subject_topic || '',
              study_type: p.study_type || '',
              notes: p.notes || '',
            });

            const state = get(timerState);
            const isBreak = state.round_type === 'short-break' || state.round_type === 'long-break';

            if (isBreak) {
              await timerSkip();
              setTimeout(() => {
                if (!get(timerState).is_running) {
                  timerToggle();
                }
              }, 50);
            } else {
              if (state.elapsed_secs > 0) {
                await timerRestartRound();
              }
              // timerRestartRound stops the timer. We want it to start ticking immediately, 
              // or if it was already stopped at 0:00, we want it to start ticking.
              setTimeout(() => {
                if (!get(timerState).is_running) {
                  timerToggle();
                }
              }, 50);
            }
          }
        )
      );
    })();

    return () => {
      for (const fn of cleanups) fn();
    };
  });
</script>

<!-- Resize handles — invisible edge/corner strips for decorations-free windows.
     Not needed on macOS where native resizing is provided by decorations:true. -->
{#if !isMac}
  <!-- N -->
  <div class="rh rh-n" onmousedown={() => startResize('North')} role="none"></div>
  <!-- S -->
  <div class="rh rh-s" onmousedown={() => startResize('South')} role="none"></div>
  <!-- E -->
  <div class="rh rh-e" onmousedown={() => startResize('East')} role="none"></div>
  <!-- W -->
  <div class="rh rh-w" onmousedown={() => startResize('West')} role="none"></div>
  <!-- NE -->
  <div class="rh rh-ne" onmousedown={() => startResize('NorthEast')} role="none"></div>
  <!-- NW -->
  <div class="rh rh-nw" onmousedown={() => startResize('NorthWest')} role="none"></div>
  <!-- SE -->
  <div class="rh rh-se" onmousedown={() => startResize('SouthEast')} role="none"></div>
  <!-- SW -->
  <div class="rh rh-sw" onmousedown={() => startResize('SouthWest')} role="none"></div>
{/if}

<div class="app">
  <Titlebar />
  <!-- Smart Suggestion -->
  {#if activeScheduledSubject && !$timerState.is_running && !$timerState.active_session_id && !isCompact}
    <div class="smart-suggestion">
      <button class="ghost-btn" onclick={handleStartScheduled}>
        Start {activeScheduledSubject} session
      </button>
    </div>
  {/if}
  <main class:compact={isCompact}>
    <Timer {isCompact} {uiScale} />
  </main>
  {#if $showTagModal}
    <SessionTagModal 
      onClose={() => ($showTagModal = false)} 
      sessionId={$timerState.active_session_id} 
      studySessionId={$timerState.active_study_session_id}
    />
  {/if}
  {#if $showGoalModal}
    <SessionGoalModal
      snap={$timerState}
      onClose={() => ($showGoalModal = false)}
    />
  {/if}
</div>

<style>
  .app {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: app-fade-in 0.4s var(--transition-slow) both;
  }

  main {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  main.compact {
    /* Bottom padding provides breathing room below the mini controls. */
    padding-bottom: 8px;
  }

  /* ---------------------------------------------------------------------------
     Resize handles — positioned outside/over the window edges so the user can
     grab them to resize a decoration-free window (needed on Linux/Wayland and
     GNOME with undecorated windows).
     --------------------------------------------------------------------------- */
  :global(.rh) {
    position: fixed;
    z-index: 9999;
  }

  /* Edge handles */
  :global(.rh-n) {
    top: 0;
    left: 6px;
    right: 6px;
    height: 5px;
    cursor: n-resize;
  }
  :global(.rh-s) {
    bottom: 0;
    left: 6px;
    right: 6px;
    height: 5px;
    cursor: s-resize;
  }
  :global(.rh-e) {
    right: 0;
    top: 6px;
    bottom: 6px;
    width: 5px;
    cursor: e-resize;
  }
  :global(.rh-w) {
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 5px;
    cursor: w-resize;
  }

  /* Corner handles (larger for easier grabbing) */
  :global(.rh-ne) {
    top: 0;
    right: 0;
    width: 10px;
    height: 10px;
    cursor: ne-resize;
  }
  :global(.rh-nw) {
    top: 0;
    left: 0;
    width: 10px;
    height: 10px;
    cursor: nw-resize;
  }
  :global(.rh-se) {
    bottom: 0;
    right: 0;
    width: 10px;
    height: 10px;
    cursor: se-resize;
  }
  :global(.rh-sw) {
    bottom: 0;
    left: 0;
    width: 10px;
    height: 10px;
    cursor: sw-resize;
  }

  .smart-suggestion {
    position: absolute;
    top: 48px;
    left: 0;
    width: 100%;
    display: flex;
    justify-content: center;
    z-index: 50;
    pointer-events: none;
  }

  .ghost-btn {
    pointer-events: auto;
    background: transparent;
    color: var(--color-subtext);
    border: 1px solid transparent;
    padding: 0.5rem 1rem;
    font-size: 0.85rem;
    font-weight: 500;
    border-radius: 20px;
    cursor: pointer;
    transition: all 0.2s ease-in-out;
    animation: fadeIn 0.5s ease-out;
  }

  .ghost-btn:hover {
    color: var(--color-text);
    border-color: var(--color-subtext);
    background: rgba(255, 255, 255, 0.05);
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(-10px); }
    to { opacity: 1; transform: translateY(0); }
  }
</style>
