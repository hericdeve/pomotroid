<script lang="ts">
  import { timerState } from '$lib/stores/timer';
  import { settings } from '$lib/stores/settings';
  import { sessionAddExtraTime } from '$lib/ipc';

  let extraSeconds = $state(0);
  let intervalId: number | null = null;
  let manuallyDismissed = $state(false);

  let isIdle = $derived(!$timerState.is_running && $timerState.elapsed_secs === 0);
  let canRunExtra = $derived(isIdle && $settings.enable_extra_timer && $timerState.last_completed_session_id !== null);

  $effect(() => {
    if (canRunExtra && !manuallyDismissed) {
      if (intervalId === null) {
        extraSeconds = 0;
        intervalId = window.setInterval(() => {
          extraSeconds += 1;
        }, 1000);
      }
    } else {
      if (intervalId !== null) {
        window.clearInterval(intervalId);
        intervalId = null;
      }
    }
  });

  $effect(() => {
    if (!isIdle) {
      manuallyDismissed = false;
      extraSeconds = 0;
    }
  });

  function stopAndDiscard() {
    manuallyDismissed = true;
  }

  async function stopAndAdd() {
    const secs = extraSeconds;
    manuallyDismissed = true; 

    if ($timerState.last_completed_session_id !== null && secs > 0) {
      try {
        await sessionAddExtraTime($timerState.last_completed_session_id, secs);
      } catch (e) {
        console.error("Failed to add extra time:", e);
      }
    }
  }

  function formatTime(s: number) {
    const mins = Math.floor(s / 60).toString().padStart(2, '0');
    const secs = (s % 60).toString().padStart(2, '0');
    return `+${mins}:${secs}`;
  }
</script>

{#if canRunExtra && !manuallyDismissed}
  <div class="extra-timer-container">
    <div class="time">{formatTime(extraSeconds)}</div>
    <div class="actions">
      <button class="btn-discard" onclick={stopAndDiscard} aria-label="Discard">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
      <button class="btn-add" onclick={stopAndAdd} aria-label="Add extra time">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <line x1="12" y1="5" x2="12" y2="19"></line>
          <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
      </button>
    </div>
  </div>
{/if}

<style>
  .extra-timer-container {
    position: absolute;
    bottom: 3rem;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    z-index: 10;
    animation: fade-in 0.3s ease-out;
  }

  @keyframes fade-in {
    from { opacity: 0; transform: translate(-50%, 10px); }
    to { opacity: 1; transform: translate(-50%, 0); }
  }

  .time {
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--color-accent);
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  button {
    background: transparent;
    border: none;
    padding: 0.25rem;
    border-radius: 50%;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .btn-discard {
    color: var(--color-foreground-subtle);
    background: var(--color-background-modifier-hover);
  }

  .btn-discard:hover {
    color: var(--color-foreground);
    background: var(--color-background-modifier-active);
  }

  .btn-add {
    color: var(--color-background);
    background: var(--color-accent);
    box-shadow: 0 2px 8px var(--color-accent-transparent);
  }

  .btn-add:hover {
    filter: brightness(1.1);
    transform: translateY(-1px);
  }
</style>
