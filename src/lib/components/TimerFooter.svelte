<script lang="ts">
  // Round counter, reset/skip buttons, and volume slider.
  import type { TimerState } from '$lib/types';
  import { timerReset, setSetting } from '$lib/ipc';
  import { settings } from '$lib/stores/settings';
  import * as m from '$paraglide/messages.js';
  import Tooltip from './Tooltip.svelte';
  import { showTagModal, pendingTags } from '$lib/stores/pendingTags';
  import { updateSession } from '$lib/ipc';
  import { sessionGoalRounds, showGoalModal } from '$lib/stores/sessionGoal';

  interface Props {
    snap: TimerState;
  }

  let { snap }: Props = $props();

  let hasTags = $derived(!!($pendingTags.subject || $pendingTags.subject_topic || $pendingTags.study_type || $pendingTags.notes));

  const completedRounds = $derived(
    snap.round_type === 'work'
      ? Math.max(0, snap.session_work_count - 1)
      : snap.session_work_count
  );

  async function handleReset() {
    if (snap.active_session_id !== null) {
      updateSession(snap.active_session_id, {
        subject: null,
        subject_topic: null,
        study_type: null,
        notes: null,
      }).catch(console.error);
    }
    pendingTags.set({
      subject: '',
      subject_topic: '',
      study_type: '',
      notes: '',
    });
    // Reset goal to the settings default
    sessionGoalRounds.set($settings.session_goal_rounds);
    timerReset();
  }
</script>

<!-- Session goal counter: X / N — click to open goal details dialog -->
<Tooltip text="Session goal — click for details">
  <button class="rounds btn-goal" onclick={() => ($showGoalModal = true)} aria-label="Session goal progress">
    {completedRounds} / {$sessionGoalRounds}
  </button>
</Tooltip>

<!-- Reset -->
<Tooltip text={m.tooltip_reset()}>
  <button class="btn-text" onclick={handleReset} aria-label={m.timer_reset()}>
    {m.timer_reset()}
  </button>
</Tooltip>

<!-- Tag Button -->
<Tooltip text="Tag Active Session">
  <button class="btn-icon" class:active={hasTags} onclick={() => ($showTagModal = true)} aria-label="Tag Session">
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
      <path d="M14.5,2.5 L10,2.5 C9.73,2.5 9.48,2.61 9.29,2.8 L2.8,9.29 C2.41,9.68 2.41,10.31 2.8,10.7 L7.3,15.2 C7.69,15.59 8.31,15.59 8.7,15.2 L15.2,8.7 C15.39,8.51 15.5,8.27 15.5,8 L15.5,3.5 C15.5,2.95 15.05,2.5 14.5,2.5 Z M12.5,5.5 C11.5,5.5 11.5,5.05 11.5,4.5 C11.5,3.95 11.95,3.5 12.5,3.5 C13.05,3.5 13.5,3.95 13.5,4.5 C13.5,5.05 13.05,5.5 12.5,5.5 Z"/>
    </svg>
  </button>
</Tooltip>



<style>
  .rounds {
    font-size: 0.8rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    min-width: 48px;
    text-align: center;
  }

  .btn-goal {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 4px;
    transition: color 0.15s, background 0.15s;
  }

  .btn-goal:hover {
    color: var(--color-foreground);
    background: var(--color-hover);
  }

  .btn-text {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-foreground-darker, var(--color-foreground));
    font-size: 0.8rem;
    padding: 4px 8px;
    border-radius: 4px;
    transition:
      color 0.15s,
      background 0.15s;
  }

  .btn-text:hover {
    color: var(--color-foreground);
    background: var(--color-hover);
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

  .btn-icon:hover {
    color: var(--color-foreground);
    background: var(--color-hover);
  }

  .btn-icon.active {
    color: var(--color-focus-round);
  }
</style>
