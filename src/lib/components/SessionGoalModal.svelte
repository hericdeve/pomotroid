<script lang="ts">
  import { setSetting, subjectGetWeeklyProgress, studySessionUpdate } from '$lib/ipc';
  import { settings } from '$lib/stores/settings';
  import { sessionGoalRounds, getMaxSessionRounds } from '$lib/stores/sessionGoal';
  import { pendingTags } from '$lib/stores/pendingTags';
  import type { TimerState, SubjectWeeklyProgress } from '$lib/types';

  interface Props {
    snap: TimerState;
    onClose: () => void;
  }

  let { snap, onClose }: Props = $props();

  // Local editable goal value — starts from the active store
  let goalInput = $state<number | null>($sessionGoalRounds);
  const activeGoal = $derived(goalInput || 1);

  const maxRounds = $derived(getMaxSessionRounds($settings));

  // Apply goal change immediately to the live store if valid
  function applyGoal() {
    if (goalInput !== null) {
      const clamped = Math.max(1, Math.min(maxRounds, Math.round(goalInput)));
      sessionGoalRounds.set(clamped);
      if (snap.active_study_session_id) {
        studySessionUpdate(snap.active_study_session_id, { goal_rounds: clamped }).catch(console.error);
      }
    }
  }

  function handleBlur() {
    if (goalInput === null) {
      goalInput = $sessionGoalRounds;
    } else {
      goalInput = Math.max(1, Math.min(maxRounds, Math.round(goalInput)));
      sessionGoalRounds.set(goalInput);
      if (snap.active_study_session_id) {
        studySessionUpdate(snap.active_study_session_id, { goal_rounds: goalInput }).catch(console.error);
      }
    }
  }

  async function setAsDefault() {
    await setSetting('session_goal_rounds', activeGoal.toString());
  }

  // ── Derived stats ──────────────────────────────────────────────────────────

  // Only count a round as completed once it is finished (on a break or the next work round started)
  const completedRounds = $derived(
    snap.round_type === 'work'
      ? Math.max(0, snap.session_work_count - 1)
      : snap.session_work_count
  );

  const remainingRounds = $derived(Math.max(0, activeGoal - completedRounds));

  // How many long/short breaks span the whole goal (for total time display rows)
  function computeBreaks(goal: number) {
    const interval = $settings.long_break_interval;
    if (!$settings.long_breaks_enabled && !$settings.short_breaks_enabled) {
      return { longBreaks: 0, shortBreaks: 0 };
    }
    if (!$settings.long_breaks_enabled) {
      return { longBreaks: 0, shortBreaks: Math.max(0, goal - 1) };
    }
    const longBreaks = Math.floor((goal - 1) / interval);
    const shortBreaks = $settings.short_breaks_enabled ? Math.max(0, goal - 1 - longBreaks) : 0;
    return { longBreaks, shortBreaks };
  }

  function totalBreakSecs(goal: number): number {
    const { longBreaks, shortBreaks } = computeBreaks(goal);
    return (
      longBreaks * $settings.time_long_break_secs +
      shortBreaks * $settings.time_short_break_secs
    );
  }

  function totalStudySecs(goal: number): number {
    return goal * $settings.time_work_secs;
  }

  // ── Future Future Simulation ───────────────────────────────────────────────

  // Simulates the exact duration of future breaks by walking through the backend's
  // cycle state (`work_round_number`) instead of using stateless modulo math.
  function simulateFutureBreaks(startCycle: number, breaksToTake: number): number {
    let totalSecs = 0;
    let cycle = startCycle;
    
    for (let i = 0; i < breaksToTake; i++) {
      if (cycle >= snap.work_rounds_total) {
        if ($settings.long_breaks_enabled) {
          totalSecs += $settings.time_long_break_secs;
        } else if ($settings.short_breaks_enabled) {
          totalSecs += $settings.time_short_break_secs;
        }
        cycle = 1; // Backend resets cycle to 1 after a long break
      } else {
        if ($settings.short_breaks_enabled) {
          totalSecs += $settings.time_short_break_secs;
        }
        cycle++;
      }
    }
    return totalSecs;
  }

  // ── Finish-time clock ──────────────────────────────────────────────────────
  
  let nowMs = $state(Date.now());

  $effect(() => {
    const id = setInterval(() => {
      nowMs = Date.now();
    }, 1000);
    return () => clearInterval(id);
  });

  // Remaining milliseconds from nowMs, correctly accounting for the
  // already-elapsed portion of the current active round or break.
  let isGoalReached = $derived(completedRounds >= activeGoal);

  function computeRemainingMs(): number {
    if (isGoalReached) return 0;

    const currentRoundRemSecs = Math.max(0, snap.total_secs - snap.elapsed_secs);
    const breaksToTake = Math.max(0, remainingRounds - 1);

    if (snap.round_type === 'work') {
      // Current round remainder + future work rounds + future simulated breaks
      const futureWorkSecs = Math.max(0, remainingRounds - 1) * $settings.time_work_secs;
      const futBreakSecs = simulateFutureBreaks(snap.work_round_number, breaksToTake);
      return (currentRoundRemSecs + futureWorkSecs + futBreakSecs) * 1000;
    } else {
      // On a break
      const futureWorkSecs = remainingRounds * $settings.time_work_secs;
      // The break we are currently on was triggered by the round that just finished.
      // The next cycle starts at the *next* work round index.
      const nextCycle = snap.work_round_number >= snap.work_rounds_total ? 1 : snap.work_round_number + 1;
      const futBreakSecs = simulateFutureBreaks(nextCycle, breaksToTake);
      return (currentRoundRemSecs + futureWorkSecs + futBreakSecs) * 1000;
    }
  }

  // ── Format helpers ─────────────────────────────────────────────────────────

  function formatDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  }

  // ── Reactive stat rows ─────────────────────────────────────────────────────
  const studyTotal = $derived(totalStudySecs(activeGoal));
  const breakTotal = $derived(totalBreakSecs(activeGoal));
  const sessionTotal = $derived(studyTotal + breakTotal);
  const progressPct = $derived(activeGoal > 0 ? Math.min(100, (completedRounds / activeGoal) * 100) : 0);

  // Finish time: stable while running, drifts forward while paused or idle.
  const finishTime = $derived(() => {
    const finishEpoch = nowMs + computeRemainingMs();
    return new Date(finishEpoch).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  });

  // Current sub-cycle info
  const roundsUntilLongBreak = $derived(
    $settings.long_breaks_enabled
      ? snap.work_rounds_total - snap.work_round_number + 1
      : null
  );

  // Weekly Subject Progress
  let weeklyProgress = $state<SubjectWeeklyProgress | null>(null);

  $effect(() => {
    if ($pendingTags.subject) {
      subjectGetWeeklyProgress($pendingTags.subject).then(prog => {
        weeklyProgress = prog;
      }).catch(err => {
        console.error("Failed to load weekly progress:", err);
      });
    } else {
      weeklyProgress = null;
    }
  });

</script>

<div class="modal-overlay" role="presentation" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>

    <!-- Header -->
    <div class="header">
      <h2>Session Goal</h2>
      <button class="close-btn" onclick={onClose} aria-label="Close">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="content">

      <!-- Progress section -->
      <div class="progress-section">
        <div class="progress-label">
          <span class="progress-text">
            <span class="progress-completed">{completedRounds}</span>
            <span class="progress-sep"> / </span>
            <span class="progress-goal">{activeGoal}</span>
            <span class="progress-unit"> rounds</span>
          </span>
          <span class="progress-remaining">
            {remainingRounds} remaining
          </span>
        </div>
        <div class="progress-track">
          <div class="progress-fill" style="width: {progressPct}%"></div>
        </div>
      </div>

      <!-- Weekly Progress (Only if subject is active and has a goal) -->
      {#if weeklyProgress && weeklyProgress.goal}
        <div class="weekly-section">
          <div class="progress-label weekly-label">
            <span class="progress-text">
              <span class="weekly-subject">{$pendingTags.subject}</span> Weekly Goal:
              <span class="progress-completed">{weeklyProgress.completed}</span>
              <span class="progress-sep"> / </span>
              <span class="progress-goal">{weeklyProgress.goal}</span>
              <span class="progress-unit"> rounds</span>
            </span>
          </div>
          <div class="progress-track weekly-track">
            <div class="progress-fill weekly-fill" style="width: {Math.min(100, (weeklyProgress.completed / weeklyProgress.goal) * 100)}%"></div>
          </div>
        </div>
      {/if}

      <!-- Stat table -->
      <div class="stats-section">
        <div class="stat-row">
          <span class="stat-label">Total study time</span>
          <span class="stat-value">{formatDuration(studyTotal)}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Total break time</span>
          <span class="stat-value">{formatDuration(breakTotal)}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Total session time</span>
          <span class="stat-value highlight">{formatDuration(sessionTotal)}</span>
        </div>
        <div class="stat-divider"></div>
        <div class="stat-row">
          <span class="stat-label">Estimated finish</span>
          <span class="stat-value highlight">{finishTime()}</span>
        </div>
        {#if roundsUntilLongBreak !== null}
          <div class="stat-row">
            <span class="stat-label">Rounds until long break</span>
            <span class="stat-value">{roundsUntilLongBreak}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Current cycle</span>
            <span class="stat-value">{snap.work_round_number} / {snap.work_rounds_total}</span>
          </div>
        {/if}
      </div>

      <!-- Goal editor -->
      <div class="goal-editor">
        <label class="goal-label" for="goal-input">
          Goal
        </label>
        <div class="goal-input-row">
          <input
            id="goal-input"
            type="number"
            min="1"
            max={maxRounds}
            bind:value={goalInput}
            oninput={applyGoal}
            onchange={applyGoal}
            onblur={handleBlur}
          />
          <span class="goal-unit">rounds</span>
          <button class="btn-default" onclick={setAsDefault} title="Save as default in settings">
            Set as default
          </button>
        </div>
      </div>

    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(4px);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    animation: fade-in 0.18s ease-out;
  }

  .modal {
    background: var(--color-background);
    border-radius: 12px;
    width: 100%;
    max-width: 340px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-background-light, rgba(255,255,255,0.08));
  }

  .header h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-foreground);
  }

  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-foreground-darker, var(--color-foreground));
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: background 0.15s;
  }

  .close-btn:hover {
    background: var(--color-hover);
    color: var(--color-foreground);
  }

  .content {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  /* ── Progress ──────────────────────────────────── */
  .progress-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .progress-label {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .progress-text {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-foreground);
  }

  .progress-completed {
    color: var(--color-focus-round);
    font-size: 1.25rem;
  }

  .progress-sep,
  .progress-goal,
  .progress-unit {
    color: var(--color-foreground-darker, var(--color-foreground));
    font-size: 0.95rem;
  }

  .progress-remaining {
    font-size: 0.8rem;
    color: var(--color-foreground-darker, var(--color-foreground));
  }

  .progress-track {
    height: 5px;
    background: var(--color-hover, rgba(255,255,255,0.08));
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-focus-round);
    border-radius: 3px;
    transition: width 0.4s ease;
  }

  /* ── Weekly Progress ───────────────────────────── */
  .weekly-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    background: var(--color-background-light, rgba(255,255,255,0.03));
    border: 1px dashed var(--color-subtext, rgba(255,255,255,0.1));
    border-radius: 6px;
  }

  .weekly-label .progress-text {
    font-size: 0.85rem;
    font-weight: 500;
  }

  .weekly-subject {
    font-weight: 600;
    color: var(--color-focus-round);
  }

  .weekly-track {
    height: 4px;
  }

  .weekly-fill {
    background: var(--color-foreground-darker, rgba(255,255,255,0.4));
  }

  /* ── Stats table ───────────────────────────────── */
  .stats-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 3px 0;
  }

  .stat-label {
    font-size: 0.82rem;
    color: var(--color-foreground-darker, var(--color-foreground));
  }

  .stat-value {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--color-foreground);
  }

  .stat-value.highlight {
    color: var(--color-focus-round);
    font-weight: 600;
  }

  .stat-divider {
    height: 1px;
    background: var(--color-background-light, rgba(255,255,255,0.08));
    margin: 4px 0;
  }

  /* ── Goal editor ───────────────────────────────── */
  .goal-editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 4px;
    border-top: 1px solid var(--color-background-light, rgba(255,255,255,0.08));
  }

  .goal-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--color-foreground-darker, var(--color-foreground));
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .goal-input-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .goal-input-row input[type='number'] {
    width: 54px;
    background: var(--color-background-light, rgba(255,255,255,0.06));
    border: 1px solid var(--color-foreground-darker, rgba(255,255,255,0.2));
    color: var(--color-foreground);
    border-radius: 5px;
    padding: 4px 8px;
    font-size: 0.9rem;
    font-weight: 600;
    text-align: center;
    transition: border-color 0.15s;
    /* hide browser spin arrows */
    -moz-appearance: textfield;
    appearance: textfield;
  }

  .goal-input-row input[type='number']::-webkit-inner-spin-button,
  .goal-input-row input[type='number']::-webkit-outer-spin-button {
    -webkit-appearance: none;
  }

  .goal-input-row input[type='number']:focus {
    outline: none;
    border-color: var(--color-focus-round);
  }

  .goal-unit {
    font-size: 0.82rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    flex: 1;
  }

  .btn-default {
    background: none;
    border: 1px solid var(--color-foreground-darker, rgba(255,255,255,0.2));
    color: var(--color-foreground-darker, var(--color-foreground));
    border-radius: 5px;
    padding: 4px 10px;
    font-size: 0.78rem;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
    white-space: nowrap;
  }

  .btn-default:hover {
    border-color: var(--color-focus-round);
    color: var(--color-focus-round);
    background: var(--color-hover);
  }

  @keyframes fade-in {
    from { opacity: 0; transform: scale(0.97); }
    to   { opacity: 1; transform: scale(1); }
  }
</style>
