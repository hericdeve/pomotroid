<script lang="ts">
  import { onMount } from 'svelte';
  import { createManualSession, createManualRound, sessionsGetHistory } from '$lib/ipc';
  import type { 
    CreateManualSessionPayload, 
    CreateManualRoundPayload, 
    UpdateSessionPayload, 
    StudySessionRow,
    RoundType 
  } from '$lib/types';
  import { settings } from '$lib/stores/settings';
  import EntryDetail from './EntryDetail.svelte';

  interface Props {
    onclose: () => void;
    initialMode?: 'session' | 'round';
    targetStudySessionId?: number | null;
  }

  let { onclose, initialMode = 'session', targetStudySessionId = null }: Props = $props();

  let activeTab = $state<'session' | 'round'>('session');
  $effect(() => {
    activeTab = initialMode;
  });

  let timeMode = $state<'now' | 'custom'>('now');
  let customDate = $state('');

  // --- Session Mode State ---
  let sessionDurationMins = $state(25);
  let sessionBreakMins = $state(5);
  let sessionRoundsCount = $state(1);
  let sessionGoalRounds = $state(1);
  let sessionDetailPayload = $state<UpdateSessionPayload>({
    subject: '',
    subject_topic: '',
    study_type: 'None / Uncategorized',
    notes: ''
  });

  // --- Round Mode State ---
  let selectedSessionId = $state<number | null>(null);
  $effect(() => {
    selectedSessionId = targetStudySessionId;
  });

  let roundType = $state<RoundType>('work');
  let roundStatus = $state<'completed' | 'half' | 'incomplete'>('completed');
  let roundDurationMins = $state(25);
  let roundExcludeFromStats = $state(false);
  let roundDetailPayload = $state<UpdateSessionPayload>({
    subject: '',
    subject_topic: '',
    study_type: 'None / Uncategorized',
    notes: ''
  });

  let availableSessions = $state<StudySessionRow[]>([]);
  let isSaving = $state(false);

  function formatSessionOptionLabel(s: StudySessionRow): string {
    const d = new Date(s.started_at * 1000);
    const dateStr = `${d.getMonth() + 1}/${d.getDate()} ${d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
    const subj = s.subject || 'Untagged';
    const topic = s.subject_topic ? ` - ${s.subject_topic}` : '';
    const rounds = s.rounds ? ` (${s.rounds.length} rounds)` : '';
    return `${dateStr}: ${subj}${topic}${rounds}`;
  }

  onMount(async () => {
    // Defaults from settings
    const workMins = Math.max(1, Math.round(($settings.time_work_secs || 1500) / 60));
    const breakMins = Math.max(1, Math.round(($settings.time_short_break_secs || 300) / 60));
    sessionDurationMins = workMins;
    sessionBreakMins = breakMins;
    roundDurationMins = workMins;

    const now = new Date();
    now.setMinutes(now.getMinutes() - now.getTimezoneOffset());
    customDate = now.toISOString().slice(0, 16);

    try {
      // Fetch recent sessions for linking
      const historyPage = await sessionsGetHistory(30, 0, {});
      if (historyPage && historyPage.sessions) {
        availableSessions = historyPage.sessions;
        if (targetStudySessionId) {
          const match = availableSessions.find(s => s.id === targetStudySessionId);
          if (match) {
            roundDetailPayload.subject = match.subject || '';
            roundDetailPayload.subject_topic = match.subject_topic || '';
            roundDetailPayload.study_type = match.study_type || 'None / Uncategorized';
            roundDetailPayload.notes = match.notes || '';
          }
        }
      }
    } catch (e) {
      console.error('Failed to load recent sessions for manual entry:', e);
    }
  });

  function handleRoundTypeChange(type: RoundType) {
    roundType = type;
    if (type === 'work') {
      roundDurationMins = Math.max(1, Math.round(($settings.time_work_secs || 1500) / 60));
    } else if (type === 'short-break') {
      roundDurationMins = Math.max(1, Math.round(($settings.time_short_break_secs || 300) / 60));
    } else {
      roundDurationMins = Math.max(1, Math.round(($settings.time_long_break_secs || 900) / 60));
    }
  }

  function handleSessionSelect(e: Event) {
    const val = (e.target as HTMLSelectElement).value;
    if (val === '') {
      selectedSessionId = null;
    } else {
      const id = parseInt(val, 10);
      selectedSessionId = isNaN(id) ? null : id;
      const match = availableSessions.find(s => s.id === selectedSessionId);
      if (match) {
        roundDetailPayload.subject = match.subject || '';
        roundDetailPayload.subject_topic = match.subject_topic || '';
        roundDetailPayload.study_type = match.study_type || 'None / Uncategorized';
      }
    }
  }

  function getStartedAt(): number {
    if (timeMode === 'now') {
      return Math.floor(Date.now() / 1000);
    }
    const parsed = Math.floor(new Date(customDate).getTime() / 1000);
    return isNaN(parsed) ? Math.floor(Date.now() / 1000) : parsed;
  }

  async function handleSave() {
    if (isSaving) return;
    isSaving = true;

    const started_at = getStartedAt();

    try {
      if (activeTab === 'session') {
        const payload: CreateManualSessionPayload = {
          started_at,
          duration_secs: sessionDurationMins * 60,
          subject: sessionDetailPayload.subject || null,
          subject_topic: sessionDetailPayload.subject_topic || null,
          study_type: sessionDetailPayload.study_type === 'None / Uncategorized' ? null : sessionDetailPayload.study_type,
          notes: sessionDetailPayload.notes || null,
          goal_rounds: sessionGoalRounds,
          rounds_count: sessionRoundsCount,
          break_duration_secs: sessionBreakMins * 60,
        };
        await createManualSession(payload);
      } else {
        const payload: CreateManualRoundPayload = {
          study_session_id: selectedSessionId,
          started_at,
          duration_secs: roundDurationMins * 60,
          round_type: roundType,
          completed: roundStatus === 'completed',
          is_half_session: roundStatus === 'half',
          exclude_from_stats: roundExcludeFromStats,
          subject: roundDetailPayload.subject || null,
          subject_topic: roundDetailPayload.subject_topic || null,
          study_type: roundDetailPayload.study_type === 'None / Uncategorized' ? null : roundDetailPayload.study_type,
          notes: roundDetailPayload.notes || null,
        };
        await createManualRound(payload);
      }
      onclose();
    } catch (err) {
      console.error('Failed to create manual entry:', err);
    } finally {
      isSaving = false;
    }
  }
</script>

<div class="modal-backdrop" onclick={onclose} role="presentation">
  <div class="modal-content" onclick={(e) => e.stopPropagation()} role="presentation">
    <div class="modal-header">
      <div class="header-left">
        <h2>Manual Entry</h2>
        <div class="mode-tabs">
          <button 
            type="button" 
            class="tab-btn" 
            class:active={activeTab === 'session'} 
            onclick={() => activeTab = 'session'}
          >
            Session
          </button>
          <button 
            type="button" 
            class="tab-btn" 
            class:active={activeTab === 'round'} 
            onclick={() => activeTab = 'round'}
          >
            Round
          </button>
        </div>
      </div>
      <button class="close-btn" aria-label="Close" onclick={onclose}>×</button>
    </div>
    
    <div class="scrollable-body">
      <!-- Time Occurred (common to both) -->
      <div class="form-row">
        <!-- svelte-ignore a11y_label_has_associated_control -->
        <label>Time Occurred</label>
        <div class="toggle-group">
          <button class:active={timeMode === 'now'} onclick={() => timeMode = 'now'}>Right Now</button>
          <button class:active={timeMode === 'custom'} onclick={() => timeMode = 'custom'}>Exact Time</button>
        </div>
      </div>
      
      {#if timeMode === 'custom'}
        <div class="form-row">
          <input type="datetime-local" bind:value={customDate} class="custom-input" />
        </div>
      {/if}

      {#if activeTab === 'session'}
        <!-- ─── SESSION MODE ─── -->
        <div class="form-row-group">
          <div class="form-row">
            <label for="rounds-count-input">Work Rounds</label>
            <input 
              id="rounds-count-input" 
              type="number" 
              bind:value={sessionRoundsCount} 
              oninput={() => {
                if (sessionGoalRounds < sessionRoundsCount) {
                  sessionGoalRounds = sessionRoundsCount;
                }
              }}
              min="1" 
              max="16" 
              class="custom-input" 
            />
          </div>
          <div class="form-row">
            <label for="session-duration-input">Duration (mins)</label>
            <input id="session-duration-input" type="number" bind:value={sessionDurationMins} min="1" class="custom-input" />
          </div>
        </div>

        <div class="form-row-group">
          {#if sessionRoundsCount > 1}
            <div class="form-row">
              <label for="break-duration-input">Break (mins)</label>
              <input id="break-duration-input" type="number" bind:value={sessionBreakMins} min="1" class="custom-input" />
            </div>
          {/if}
          <div class="form-row">
            <label for="goal-rounds-input">Session Goal</label>
            <input id="goal-rounds-input" type="number" bind:value={sessionGoalRounds} min="1" max="24" class="custom-input" />
          </div>
        </div>

        <div class="divider"></div>
        <EntryDetail bind:payload={sessionDetailPayload} />

      {:else}
        <!-- ─── ROUND MODE ─── -->
        <div class="form-row">
          <label for="target-session-select">Study Session</label>
          <select 
            id="target-session-select" 
            class="custom-select" 
            value={selectedSessionId === null ? '' : selectedSessionId}
            onchange={handleSessionSelect}
          >
            <option value="">+ New Standalone Session</option>
            {#each availableSessions as s}
              <option value={s.id}>{formatSessionOptionLabel(s)}</option>
            {/each}
          </select>
        </div>

        <div class="form-row">
          <!-- svelte-ignore a11y_label_has_associated_control -->
          <label>Round Type</label>
          <div class="toggle-group">
            <button 
              type="button" 
              class:active={roundType === 'work'} 
              onclick={() => handleRoundTypeChange('work')}
            >
              Work
            </button>
            <button 
              type="button" 
              class:active={roundType === 'short-break'} 
              onclick={() => handleRoundTypeChange('short-break')}
            >
              Short Break
            </button>
            <button 
              type="button" 
              class:active={roundType === 'long-break'} 
              onclick={() => handleRoundTypeChange('long-break')}
            >
              Long Break
            </button>
          </div>
        </div>

        <div class="form-row">
          <!-- svelte-ignore a11y_label_has_associated_control -->
          <label>Round Status</label>
          <div class="toggle-group status-toggles">
            <button 
              type="button" 
              class:active={roundStatus === 'completed'} 
              onclick={() => roundStatus = 'completed'}
            >
              <span class="status-badge complete">✓</span> Completed
            </button>
            <button 
              type="button" 
              class:active={roundStatus === 'half'} 
              onclick={() => roundStatus = 'half'}
            >
              <span class="status-badge half">½</span> Half
            </button>
            <button 
              type="button" 
              class:active={roundStatus === 'incomplete'} 
              onclick={() => roundStatus = 'incomplete'}
            >
              <span class="status-badge incomplete">✕</span> Incomplete
            </button>
          </div>
        </div>

        <div class="form-row">
          <label for="round-duration-input">Duration (minutes)</label>
          <input id="round-duration-input" type="number" bind:value={roundDurationMins} min="1" class="custom-input" />
        </div>

        <label class="checkbox-row">
          <input type="checkbox" bind:checked={roundExcludeFromStats} />
          <span>Exclude from statistics</span>
        </label>

        <div class="divider"></div>
        <EntryDetail bind:payload={roundDetailPayload} />
      {/if}
    </div>

    <div class="actions">
      <button class="btn-cancel" onclick={onclose}>Cancel</button>
      <button class="btn-save" onclick={handleSave} disabled={isSaving}>
        {activeTab === 'session' ? 'Save Session' : 'Save Round'}
      </button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
    animation: fade-in 0.2s ease-out;
  }

  .modal-content {
    background: var(--color-background);
    width: 90%;
    max-width: 440px;
    border-radius: 8px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.3);
    display: flex;
    flex-direction: column;
    max-height: 90vh;
    animation: slide-up 0.2s ease-out;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 20px;
    border-bottom: 1px solid var(--color-separator);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--color-foreground);
    letter-spacing: 0.02em;
  }

  .mode-tabs {
    display: flex;
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
    padding: 2px;
    border-radius: 6px;
    gap: 2px;
  }

  .tab-btn {
    background: transparent;
    border: none;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 0.78rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-foreground-darker);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-btn:hover:not(.active) {
    color: var(--color-foreground);
  }

  .tab-btn.active {
    background: var(--color-focus-round, var(--color-accent));
    color: var(--color-background);
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.5rem;
    color: var(--color-foreground-darker);
    cursor: pointer;
    padding: 0;
    line-height: 1;
    transition: color 0.15s;
  }

  .close-btn:hover {
    color: var(--color-accent);
  }

  .scrollable-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-row-group {
    display: flex;
    gap: 12px;
  }

  .form-row-group .form-row {
    flex: 1;
  }

  label {
    font-size: 0.85rem;
    color: var(--color-foreground-darker);
    font-weight: 500;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
    color: var(--color-foreground);
    cursor: pointer;
    user-select: none;
  }

  .toggle-group {
    display: flex;
    gap: 8px;
  }

  .toggle-group button {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
    border: 1px solid transparent;
    color: var(--color-foreground);
    padding: 8px;
    border-radius: 4px;
    font-size: 0.85rem;
    cursor: pointer;
    transition: var(--transition-default);
  }

  .toggle-group button.active {
    background: var(--color-focus-round);
    color: var(--color-background);
  }

  .toggle-group button:hover:not(.active) {
    background: color-mix(in oklch, var(--color-foreground) 15%, transparent);
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    font-size: 9px;
    font-weight: bold;
    line-height: 1;
  }
  .status-badge.complete {
    background: rgba(34, 197, 94, 0.2);
    color: #4ade80;
  }
  .toggle-group button.active .status-badge.complete {
    background: rgba(255, 255, 255, 0.3);
    color: var(--color-background);
  }
  .status-badge.half {
    background: rgba(234, 179, 8, 0.2);
    color: #facc15;
  }
  .toggle-group button.active .status-badge.half {
    background: rgba(255, 255, 255, 0.3);
    color: var(--color-background);
  }
  .status-badge.incomplete {
    background: rgba(239, 68, 68, 0.2);
    color: #f87171;
  }
  .toggle-group button.active .status-badge.incomplete {
    background: rgba(255, 255, 255, 0.3);
    color: var(--color-background);
  }

  .custom-input,
  .custom-select {
    width: 100%;
    padding: 10px 14px;
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--color-foreground);
    font-size: 0.9rem;
    font-family: 'Mona Sans', system-ui, sans-serif;
    transition: var(--transition-default);
    outline: none;
  }

  .custom-select option {
    background: var(--color-background);
    color: var(--color-foreground);
  }

  .custom-input:focus,
  .custom-select:focus {
    border-color: var(--color-accent);
    background: color-mix(in oklch, var(--color-foreground) 15%, transparent);
  }

  .divider {
    height: 1px;
    background: var(--color-separator);
    margin: 4px 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--color-separator);
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid var(--color-foreground-darker);
    color: var(--color-foreground);
    padding: 8px 16px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.15s;
  }

  .btn-cancel:hover {
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
  }

  .btn-save {
    background: var(--color-focus-round);
    border: none;
    color: var(--color-background);
    padding: 8px 16px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 500;
    transition: filter 0.15s;
  }

  .btn-save:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-save:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes slide-up {
    from { transform: translateY(10px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
</style>
