<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { getSessionSubjects, getSessionTopics, getSessionStudyTypes, sessionsGetHistory } from '$lib/ipc';
  import DropdownSelect from '$lib/components/DropdownSelect.svelte';
  import type { SessionRow, SessionFilter, SessionHistoryPage } from '$lib/types';
  import * as m from '$paraglide/messages.js';
  import { settings } from '$lib/stores/settings';

  let { onEditSession, onEditStudySession }: { onEditSession: (id: number) => void, onEditStudySession: (id: number) => void } = $props();

  let limit = 50;
  let offset = $state(0);

  let subjects = $state<string[]>([]);
  let topics = $state<string[]>([]);
  
  let filterSubject = $state<string>('');
  let filterTopic = $state<string>('');
  let filterStudyType = $state<string>('');
  
  let dateFromStr = $state<string>('');
  let dateToStr = $state<string>('');
  
  let quickMode = $state<'day' | 'week' | 'year' | 'all'>('all');

  let history = $state<SessionHistoryPage>({ sessions: [], total: 0, total_work_rounds: 0, total_focus_secs: 0, longest_streak: 0 });
  let loading = $state(false);

  const DEFAULT_STUDY_TYPES = [
    'None / Uncategorized'
  ];
  let studyTypes = $state<string[]>([...DEFAULT_STUDY_TYPES]);

  function formatUnix(ts: number) {
    const d = new Date(ts * 1000);
    return `${d.toLocaleDateString()} ${d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
  }

  function formatDuration(secs: number) {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    if (h > 0) {
      return `${h}h ${m}m ${s}s`;
    }
    return `${m}m ${s}s`;
  }

  function fmtDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return `${h.toLocaleString()}h ${m}m`;
  }

  async function loadSubjectsAndTopics() {
    subjects = await getSessionSubjects();
    if (filterSubject) {
      topics = await getSessionTopics(filterSubject);
    } else {
      topics = await getSessionTopics();
    }
    
    const fetchedTypes = await getSessionStudyTypes();
    if (fetchedTypes && fetchedTypes.length > 0) {
      const unique = new Set([...DEFAULT_STUDY_TYPES, ...fetchedTypes]);
      studyTypes = Array.from(unique);
    }
  }

  async function loadHistory() {
    loading = true;
    try {
      let date_from: number | null = null;
      let date_to: number | null = null;

      if (dateFromStr) {
        const d = new Date(dateFromStr + 'T00:00:00');
        d.setHours(0, 0, 0, 0);
        date_from = Math.floor(d.getTime() / 1000);
      }
      if (dateToStr) {
        const d = new Date(dateToStr + 'T00:00:00');
        d.setHours(23, 59, 59, 999);
        date_to = Math.floor(d.getTime() / 1000);
      }

      const filter: SessionFilter = {
        subject: filterSubject || null,
        subject_topic: filterTopic || null,
        study_type: filterStudyType || null,
        date_from,
        date_to,
        show_breaks: $settings.history_show_breaks
      };

      history = await sessionsGetHistory(limit, offset, filter);
    } catch (e) {
      console.error('Failed to load history', e);
    } finally {
      loading = false;
    }
  }

  function setDateStr(d: Date, field: 'from' | 'to') {
    // Format to YYYY-MM-DD
    const pad = (n: number) => n.toString().padStart(2, '0');
    const str = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    if (field === 'from') dateFromStr = str;
    else dateToStr = str;
  }

  function applyQuickMode(mode: 'day' | 'week' | 'year' | 'all', referenceDate: Date = new Date()) {
    quickMode = mode;
    if (mode === 'all') {
      dateFromStr = '';
      dateToStr = '';
      return;
    }

    const start = new Date(referenceDate);
    const end = new Date(referenceDate);

    if (mode === 'day') {
      setDateStr(start, 'from');
      setDateStr(end, 'to');
    } else if (mode === 'week') {
      const day = start.getDay(); // 0 is Sunday
      const diff = start.getDate() - day + (day === 0 ? -6 : 1); // Adjust when day is sunday
      start.setDate(diff);
      end.setDate(diff + 6);
      setDateStr(start, 'from');
      setDateStr(end, 'to');
    } else if (mode === 'year') {
      start.setMonth(0, 1);
      end.setMonth(11, 31);
      setDateStr(start, 'from');
      setDateStr(end, 'to');
    }
  }

  function shiftPeriod(dir: -1 | 1) {
    if (quickMode === 'all') return;
    if (!dateFromStr) return;
    // ensure date parses relative to local correctly by appending T00:00:00
    const ref = new Date(dateFromStr + 'T00:00:00');
    
    if (quickMode === 'day') {
      ref.setDate(ref.getDate() + dir);
    } else if (quickMode === 'week') {
      ref.setDate(ref.getDate() + (dir * 7));
    } else if (quickMode === 'year') {
      ref.setFullYear(ref.getFullYear() + dir);
    }
    applyQuickMode(quickMode, ref);
  }

  // Reactive effect for filters
  $effect(() => {
    // Any change to these variables triggers a history reload and page reset
    const _f1 = filterSubject;
    const _f2 = filterTopic;
    const _f3 = filterStudyType;
    const _f4 = dateFromStr;
    const _f5 = dateToStr;
    
    untrack(() => {
      offset = 0; // reset to first page when filters change
      loadHistory();
      if (_f1 || _f1 === '') {
          // Need to reload topics if subject changes
          loadSubjectsAndTopics();
      }
    });
  });

  $effect(() => {
    // Only load history when offset changes without resetting to 0
    const _o = offset;
    untrack(() => {
      loadHistory();
    });
  });

  onMount(() => {
    applyQuickMode('all');
    loadSubjectsAndTopics();
  });
</script>

<div class="history-view">
  <div class="controls">
    <div class="filter-row">
      <DropdownSelect bind:value={filterSubject} options={subjects} placeholder="All Subjects" />
      <DropdownSelect bind:value={filterTopic} options={topics} placeholder="All Topics" />
      <DropdownSelect bind:value={filterStudyType} options={studyTypes} placeholder="All Types" />
    </div>

    <div class="filter-row date-controls">
      <div class="quick-modes">
        <button class="btn btn-small" class:active={quickMode === 'day'} onclick={() => applyQuickMode('day')}>Day</button>
        <button class="btn btn-small" class:active={quickMode === 'week'} onclick={() => applyQuickMode('week')}>Week</button>
        <button class="btn btn-small" class:active={quickMode === 'year'} onclick={() => applyQuickMode('year')}>Year</button>
        <button class="btn btn-small" class:active={quickMode === 'all'} onclick={() => applyQuickMode('all')}>All Time</button>
      </div>

      <div class="date-shifters" style:visibility={quickMode === 'all' ? 'hidden' : 'visible'}>
        <button class="btn btn-icon btn-small" aria-label="Previous Period" onclick={() => shiftPeriod(-1)}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 18l-6-6 6-6" /></svg>
        </button>
        <div class="custom-dates">
          <input type="date" bind:value={dateFromStr} class="date-input" aria-label="From Date" />
          <span class="date-sep">to</span>
          <input type="date" bind:value={dateToStr} class="date-input" aria-label="To Date" />
        </div>
        <button class="btn btn-icon btn-small" aria-label="Next Period" onclick={() => shiftPeriod(1)}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18l6-6-6-6" /></svg>
        </button>
      </div>

      <div class="pagination">
        <button disabled={offset === 0} onclick={() => offset = Math.max(0, offset - limit)}>&#9664; Prev</button>
        <span class="page-info">
          {history.total === 0 ? '0-0' : Math.min(offset + 1, history.total)} - {Math.min(offset + limit, history.total)} of {history.total}
        </span>
        <button disabled={offset + limit >= history.total} onclick={() => offset += limit}>Next &#9654;</button>
      </div>
    </div>
  </div>

  <div class="list-container">
    {#if loading}
      <div class="msg">Loading...</div>
    {:else if history.sessions.length === 0}
      <div class="msg">No sessions found.</div>
    {:else}
      <div class="history-grid">
        <div class="grid-header">
          <div>Date</div>
          <div>Type</div>
          <div class="text-center">Status</div>
          <div>Duration</div>
          <div>Subject</div>
          <div>Topic</div>
          <div>Study Type</div>
        </div>
        
        <div class="grid-body">
          {#each history.sessions as session}
            <div class="session-card">
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="session-row grid-row" onclick={() => onEditStudySession(session.id)}>
                <div class="font-bold">{formatUnix(session.started_at)}</div>
                <div class="font-bold">Session</div>
                <div class="font-bold text-center">Goal: {session.goal_rounds}</div>
                <div class="font-bold">
                  {formatDuration(session.rounds.reduce((acc, r) => acc + r.duration_secs, 0))}
                </div>
                <div class="font-bold">{session.subject || '-'}</div>
                <div class="font-bold">{session.subject_topic || '-'}</div>
                <div class="font-bold">{session.study_type || '-'}</div>
              </div>
              
              <div class="rounds-container">
                {#each session.rounds as row}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div class="round-row grid-row" onclick={() => onEditSession(row.id)}>
                    <div class="pl-6 text-sm opacity-70 flex items-center gap-2">
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="opacity-50"><polyline points="9 10 4 15 9 20"></polyline><path d="M20 4v7a4 4 0 0 1-4 4H4"></path></svg>
                      {formatUnix(row.started_at)}
                    </div>
                    <div class="text-sm opacity-70">{row.round_type === 'work' ? 'Work' : row.round_type === 'short-break' ? 'Short Break' : 'Long Break'}</div>
                    <div class="text-sm opacity-70 text-center">
                      {#if row.round_type === 'work'}
                        {#if row.completed}
                          <span class="status-badge complete" title="Completed">✓</span>
                        {:else}
                          <span class="status-badge incomplete" title="Incomplete">✕</span>
                        {/if}
                      {/if}
                    </div>
                    <div class="text-sm opacity-70">{formatDuration(row.duration_secs)}</div>
                    <div class="text-sm opacity-70">{row.subject || '-'}</div>
                    <div class="text-sm opacity-70">{row.subject_topic || '-'}</div>
                    <div class="text-sm opacity-70">{row.study_type || '-'}</div>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <div class="totals">
    <div class="total-card" style="--delay: 0ms">
      <span class="total-label">{m.stats_total_rounds()}</span>
      <span class="total-value">{Number(history.total_work_rounds).toLocaleString(undefined, { maximumFractionDigits: 1 })}</span>
    </div>
    <div class="total-divider"></div>
    <div class="total-card" style="--delay: 60ms">
      <span class="total-label">{m.stats_focus_time()}</span>
      <span class="total-value">{fmtDuration(history.total_focus_secs)}</span>
    </div>
    <div class="total-divider"></div>
    <div class="total-card" style="--delay: 120ms">
      <span class="total-label">{m.stats_best_streak()}</span>
      <span class="total-value">
        {history.longest_streak > 0 ? history.longest_streak : '—'}
        {#if history.longest_streak > 0}
          <span class="total-unit">{m.stats_days()}</span>
        {/if}
      </span>
    </div>
  </div>
</div>

<style>
  .history-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
    box-sizing: border-box;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 16px 0 16px;
  }

  .filter-row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .date-input {
    flex: none;
    min-width: 115px;
    width: 115px;
  }

  .pagination {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-left: auto;
  }

  .pagination button {
    background: var(--color-hover);
    color: var(--color-foreground);
    border: 1px solid var(--color-separator);
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: background 0.1s ease;
  }

  .pagination button:hover:not(:disabled) {
    background: var(--color-focus-round);
    color: var(--color-background);
    border-color: var(--color-focus-round);
  }

  .pagination button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .page-info {
    font-size: 0.85rem;
    color: var(--color-foreground-darker);
    font-variant-numeric: tabular-nums;
    min-width: 85px;
    text-align: center;
    display: inline-block;
  }

  /* ── Lifetime totals ─────────────────────────────────────── */
  .totals {
    display: flex;
    align-items: stretch;
    border-top: 1px solid var(--color-separator);
    flex-shrink: 0;
  }

  .total-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 18px 24px;
    animation: card-rise 0.35s cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--delay, 0ms);
  }

  @keyframes card-rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .total-label {
    font-size: 0.62rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-foreground-darker);
  }

  .total-value {
    font-size: 1.8rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
    color: var(--color-foreground);
    line-height: 1;
    display: flex;
    align-items: baseline;
    gap: 4px;
  }

  .total-unit {
    font-size: 0.85rem;
    font-weight: 400;
    color: var(--color-foreground-darker);
  }

  .total-divider {
    width: 1px;
    background: var(--color-separator);
    align-self: stretch;
    margin: 10px 0;
  }

  .date-controls {
    flex: 1;
    justify-content: space-between;
    flex-wrap: nowrap;
    overflow-x: auto;
  }
  .quick-modes {
    display: flex;
    gap: 4px;
    background: var(--color-background);
    padding: 4px;
    border-radius: 6px;
  }
  .btn-small {
    padding: 4px 10px;
    font-size: 0.85rem;
    border: none;
    background: transparent;
    color: var(--color-foreground-darker);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
  }
  .btn-small:hover {
    color: var(--color-foreground);
    background: var(--color-hover);
  }
  .btn-small.active {
    background: var(--color-focus-round);
    color: var(--color-background);
    font-weight: 600;
  }
  .btn-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
  }
  .date-shifters {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--color-background);
    padding: 4px 8px;
    border-radius: 6px;
  }
  .custom-dates {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .date-input {
    background: transparent;
    border: 1px solid var(--color-separator);
    color: var(--color-foreground);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.85rem;
  }
  .date-sep {
    font-size: 0.85rem;
    color: var(--color-foreground-darker);
  }

  .list-container {
    flex: 1;
    overflow-y: auto;
    background: transparent;
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    margin: 16px;
  }

  .msg {
    padding: 24px;
    text-align: center;
    color: var(--color-foreground-darker);
  }

  .history-grid {
    display: flex;
    flex-direction: column;
    font-size: 0.85rem;
  }
  
  .grid-header, .grid-row {
    display: grid;
    grid-template-columns: 18% 12% 8% 12% 18% 17% 15%;
    align-items: center;
  }

  .grid-header {
    background: var(--color-background);
    color: var(--color-foreground-darker);
    font-weight: 600;
    position: sticky;
    top: 0;
    z-index: 10;
    padding: 10px 16px;
    border-bottom: 1px solid var(--color-separator);
  }

  .grid-header > div, .grid-row > div {
    padding-right: 8px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .grid-body {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .session-card {
    border: 1px solid var(--color-separator);
    border-radius: 8px;
    overflow: hidden;
    background: var(--color-background);
  }

  .session-row {
    padding: 12px 16px;
    background: var(--color-hover);
    cursor: pointer;
    transition: background 0.1s;
    border-bottom: 1px solid var(--color-separator);
  }

  .session-row:hover {
    filter: brightness(1.1);
  }

  .rounds-container {
    display: flex;
    flex-direction: column;
  }

  .round-row {
    padding: 10px 16px;
    cursor: pointer;
    transition: background 0.1s;
    border-bottom: 1px solid var(--color-separator);
  }
  
  .round-row:last-child {
    border-bottom: none;
  }

  .round-row:hover {
    background: var(--color-hover);
  }
  
  .pl-6 { padding-left: 24px; }
  .opacity-70 { opacity: 0.7; }
  .opacity-50 { opacity: 0.5; }
  .text-sm { font-size: 0.8rem; }
  .flex { display: flex; }
  .items-center { align-items: center; }
  .gap-2 { gap: 8px; }
  .font-bold { font-weight: 600; }
  .text-center { text-align: center; }
</style>
