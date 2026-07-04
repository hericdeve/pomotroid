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
      <button class="btn-discard" onclick={stopAndDiscard}>Discard</button>
      <button class="btn-add" onclick={stopAndAdd}>Add</button>
    </div>
  </div>
{/if}

<style>
  .extra-timer-container {
    position: absolute;
    bottom: 4rem;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    z-index: 10;
    animation: fade-in 0.3s ease-out;
  }

  @keyframes fade-in {
    from { opacity: 0; transform: translate(-50%, 10px); }
    to { opacity: 1; transform: translate(-50%, 0); }
  }

  .time {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--color-accent);
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    gap: 0.75rem;
  }

  button {
    background: transparent;
    border: none;
    font-size: 0.8rem;
    padding: 0.3rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
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
