<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { isMac } from '$lib/utils/platform';
  import * as m from '$paraglide/messages.js';

  let maximized = $state(false);

  onMount(() => {
    const win = getCurrentWebviewWindow();
    win.isMaximized().then((v) => {
      maximized = v;
    });
    const unlisten = win.onResized(async () => {
      maximized = await win.isMaximized();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  function minimize() {
    getCurrentWebviewWindow().minimize();
  }

  function toggleMaximize() {
    getCurrentWebviewWindow().toggleMaximize();
  }

  function handleTitlebarDblClick(e: MouseEvent) {
    if ((e.target as HTMLElement).closest('button')) return;
    toggleMaximize();
  }

  function close() {
    getCurrentWebviewWindow().close();
  }
</script>

<nav class="titlebar" class:macos={isMac} data-tauri-drag-region ondblclick={handleTitlebarDblClick}>
  <h1 class="title">{m.settings_title()}</h1>
  <!-- Hidden on macOS; the native traffic light buttons handle this. -->
  {#if !isMac}
    <div class="controls">
      <button class="btn-ctrl" onclick={minimize} aria-label="Minimize">
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
        class="btn-ctrl"
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
      <button class="btn-ctrl btn-close" onclick={close} aria-label="Close">
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
    </div>
  {/if}
</nav>

<style>
  .titlebar {
    height: 40px;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    flex-shrink: 0;
    border-bottom: 1px solid var(--color-separator);
  }

  /* Shift the centered title right of the traffic lights on macOS. */
  .macos {
    padding-left: 72px;
  }

  .title {
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-foreground-darker, var(--color-foreground));
    pointer-events: none;
  }

  .controls {
    position: absolute;
    right: 8px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .btn-ctrl {
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

  .btn-ctrl:hover {
    color: var(--color-foreground);
    background: var(--color-hover);
  }

  .btn-ctrl.btn-close:hover {
    color: var(--color-background);
    background: var(--color-focus-round);
  }
</style>
