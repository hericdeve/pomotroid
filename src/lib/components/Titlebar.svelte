<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWebviewWindow, WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { openSettingsWindow, openStatsWindow } from '$lib/utils/windows';
  import { setWindowVisibility } from '$lib/ipc';
  import { settings } from '$lib/stores/settings';
  import { isMac } from '$lib/utils/platform';
  import Tooltip from './Tooltip.svelte';
  import * as m from '$paraglide/messages.js';
  import { timerState } from '$lib/stores/timer';
  import { setSetting } from '$lib/ipc';

  let maximized = $state(false);
  let suppressTitlebarHover = $state(false);

  let showVolume = $state(false);
  let localVolume = $state($settings.volume);
  $effect(() => {
    localVolume = $settings.volume;
  });
  let premuteVolume = $state<number | null>(null);

  function handleVolumeChange(e: Event) {
    const val = (e.target as HTMLInputElement).valueAsNumber;
    localVolume = val;
    setSetting('volume', String(Math.round(val * 100)));
  }

  function toggleMute() {
    if (localVolume === 0) {
      const restore = premuteVolume ?? 1.0;
      premuteVolume = null;
      localVolume = restore;
      setSetting('volume', String(Math.round(restore * 100)));
    } else {
      premuteVolume = localVolume;
      localVolume = 0;
      setSetting('volume', '0');
    }
  }

  function blurTitlebarControl() {
    const active = document.activeElement;
    if (active instanceof HTMLElement && active.closest('.titlebar')) active.blur();
  }

  function suppressRestoredTitlebarState() {
    suppressTitlebarHover = true;
    blurTitlebarControl();
  }

  onMount(() => {
    const win = getCurrentWebviewWindow();
    win.isMaximized().then((v) => {
      maximized = v;
    });
    const unlisten = win.onResized(async () => {
      maximized = await win.isMaximized();
    });
    const clearRestoredTitlebarFocus = () => {
      if (suppressTitlebarHover) requestAnimationFrame(blurTitlebarControl);
    };
    const clearSuppressedTitlebarHover = () => {
      suppressTitlebarHover = false;
    };
    window.addEventListener('focus', clearRestoredTitlebarFocus);
    document.addEventListener('pointermove', clearSuppressedTitlebarHover);
    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener('focus', clearRestoredTitlebarFocus);
      document.removeEventListener('pointermove', clearSuppressedTitlebarHover);
    };
  });

  async function openSettings() {
    await openSettingsWindow();
  }

  async function openStats() {
    await openStatsWindow();
  }

  async function minimize() {
    suppressRestoredTitlebarState();
    if ($settings.min_to_tray) {
      await setWindowVisibility(false);
    } else {
      await getCurrentWebviewWindow().minimize();
    }
  }

  function toggleMaximize() {
    getCurrentWebviewWindow().toggleMaximize();
  }

  async function close() {
    suppressRestoredTitlebarState();
    await getCurrentWebviewWindow().close();
  }
</script>

{#snippet settingsBtn()}
  <Tooltip text={m.tooltip_settings()}>
    <button class="btn-icon" onclick={openSettings} aria-label="Settings">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
        <line
          x1="2"
          y1="4"
          x2="14"
          y2="4"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
        <circle
          cx="5"
          cy="4"
          r="1.8"
          fill="var(--color-background)"
          stroke="currentColor"
          stroke-width="1.3"
        />
        <line
          x1="2"
          y1="8"
          x2="14"
          y2="8"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
        <circle
          cx="11"
          cy="8"
          r="1.8"
          fill="var(--color-background)"
          stroke="currentColor"
          stroke-width="1.3"
        />
        <line
          x1="2"
          y1="12"
          x2="14"
          y2="12"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
        <circle
          cx="7"
          cy="12"
          r="1.8"
          fill="var(--color-background)"
          stroke="currentColor"
          stroke-width="1.3"
        />
      </svg>
    </button>
  </Tooltip>
{/snippet}

{#snippet statsBtn()}
  <Tooltip text={m.tooltip_statistics()}>
    <button class="btn-icon" onclick={openStats} aria-label="Statistics">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
        <rect x="2" y="9" width="3" height="5" rx="0.5" fill="currentColor" opacity="0.6" />
        <rect x="6.5" y="5" width="3" height="9" rx="0.5" fill="currentColor" opacity="0.8" />
        <rect x="11" y="2" width="3" height="12" rx="0.5" fill="currentColor" />
      </svg>
    </button>
  </Tooltip>
{/snippet}

{#snippet volumeBtn()}
  <div class="volume-wrapper">
    <Tooltip text={localVolume === 0 ? m.tooltip_unmute() : m.tooltip_mute()}>
      <button
        class="btn-icon"
        onclick={toggleMute}
        aria-label={localVolume === 0 ? 'Unmute' : 'Mute'}
        onmouseenter={() => (showVolume = true)}
        onmouseleave={() => (showVolume = false)}
      >
        {#if localVolume === 0}
          <svg width="16" height="16" viewBox="0 0 16 16">
            <polygon points="1,5 5,5 10,1 10,15 5,11 1,11" fill="currentColor" />
            <line
              x1="12"
              y1="5"
              x2="16"
              y2="11"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
            <line
              x1="16"
              y1="5"
              x2="12"
              y2="11"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        {:else}
          <svg width="16" height="16" viewBox="0 0 16 16">
            <polygon points="1,5 5,5 10,1 10,15 5,11 1,11" fill="currentColor" />
            <path
              d="M12,5 Q15,8 12,11"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        {/if}
      </button>
    </Tooltip>

    {#if showVolume}
      <div class="volume-slider-wrapper" role="presentation" onmouseenter={() => (showVolume = true)} onmouseleave={() => (showVolume = false)}>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={localVolume}
          oninput={handleVolumeChange}
          class="volume-slider"
          aria-label="Volume"
        />
      </div>
    {/if}
  </div>
{/snippet}

<nav class="titlebar" class:suppress-hover={suppressTitlebarHover} data-tauri-drag-region>
  <!-- Left: settings + stats buttons on Linux/Windows. On macOS the traffic
       lights live here; the action buttons move to the right side instead. -->
  {#if !isMac}
    {@render settingsBtn()}
    {@render statsBtn()}
    {@render volumeBtn()}
  {/if}

  <!-- Right: settings + stats buttons on macOS, window controls on Linux/Windows. -->
  <div class="controls">
    {#if isMac}
      {@render volumeBtn()}
      {@render statsBtn()}
      {@render settingsBtn()}
    {:else}
      {#if $settings.enable_window_controls}
        <button class="btn-icon" onclick={minimize} aria-label="Minimize">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <line
              x1="1"
              y1="6"
              x2="11"
              y2="6"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
        <button
          class="btn-icon"
          onclick={toggleMaximize}
          aria-label={maximized ? 'Restore' : 'Maximize'}
        >
          {#if maximized}
            <svg width="12" height="12" viewBox="0 0 12 12">
              <rect
                x="3"
                y="1"
                width="8"
                height="8"
                rx="1"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
              />
              <path
                d="M1 4 L1 11 L8 11"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M11 4 L11 9 L8 9"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M4 1 L4 4 L8 4"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          {:else}
            <svg width="12" height="12" viewBox="0 0 12 12">
              <rect
                x="1"
                y="1"
                width="10"
                height="10"
                rx="1"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
              />
            </svg>
          {/if}
        </button>
        <button class="btn-icon close-btn" onclick={close} aria-label="Close">
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
    {/if}
  </div>
</nav>

<style>
  .titlebar {
    height: 40px;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px;
    position: relative;
    flex-shrink: 0;
  }


  .volume-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .volume-slider-wrapper {
    position: absolute;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px;
    background: var(--color-background-light);
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    width: 36px;
    height: 100px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.15);
  }

  .volume-slider {
    width: 80px;
    transform: rotate(-90deg);
    cursor: pointer;
    accent-color: var(--color-accent);
  }

  .controls {
    display: flex;
    gap: 4px;
    margin-left: auto;
  }

  .btn-icon {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-foreground-darker, var(--color-foreground));
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

  .btn-icon:focus {
    outline: none;
  }

  .btn-icon:focus-visible {
    outline: 2px solid color-mix(in oklch, var(--color-foreground) 45%, transparent);
    outline-offset: 2px;
  }

  .titlebar:not(.suppress-hover) .btn-icon:hover {
    background-color: var(--color-hover);
  }
</style>
