<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { getSessionSubjects, getSessionTopics, sessionsGetHistory } from '$lib/ipc';
  import type { SessionRow, SessionFilter, SessionHistoryPage } from '$lib/types';
  import * as m from '$paraglide/messages.js';

  let { onEditSession }: { onEditSession: (id: number) => void } = $props();

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

  let history = $state<SessionHistoryPage>({ sessions: [], total: 0 });
  let loading = $state(false);

  const studyTypes = [
    'None / Uncategorized',
    'Exercise',
    'Reading',
    'Review',
    'Classroom',
    'Video',
    'Flash Cards'
  ];

  function formatUnix(ts: number) {
    const d = new Date(ts * 1000);
    return `${d.toLocaleDateString()} ${d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
  }

  function formatDuration(secs: number) {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}m ${s}s`;
  }

  async function loadSubjectsAndTopics() {
    subjects = await getSessionSubjects();
    if (filterSubject) {
      topics = await getSessionTopics(filterSubject);
    } else {
      topics = await getSessionTopics();
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
        date_to
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
      <select bind:value={filterSubject} class="custom-select">
        <option value="">All Subjects</option>
        {#each subjects as s}
          <option value={s}>{s}</option>
        {/each}
      </select>

      <select bind:value={filterTopic} class="custom-select">
        <option value="">All Topics</option>
        {#each topics as t}
          <option value={t}>{t}</option>
        {/each}
      </select>

      <select bind:value={filterStudyType} class="custom-select">
        <option value="">All Types</option>
        {#each studyTypes as type}
          <option value={type}>{type}</option>
        {/each}
      </select>
    </div>

    <div class="filter-row">
      <div class="quick-select">
        <button class:active={quickMode === 'day'} onclick={() => applyQuickMode('day')}>Day</button>
        <button class:active={quickMode === 'week'} onclick={() => applyQuickMode('week')}>Week</button>
        <button class:active={quickMode === 'year'} onclick={() => applyQuickMode('year')}>Year</button>
        <button class:active={quickMode === 'all'} onclick={() => applyQuickMode('all')}>All Time</button>
      </div>

      <div class="date-range">
        <button class="nav-btn" disabled={quickMode === 'all'} onclick={() => shiftPeriod(-1)}>&#9664;</button>
        <input type="date" bind:value={dateFromStr} class="custom-input date-input" />
        <span>to</span>
        <input type="date" bind:value={dateToStr} class="custom-input date-input" />
        <button class="nav-btn" disabled={quickMode === 'all'} onclick={() => shiftPeriod(1)}>&#9654;</button>
      </div>
    </div>
  </div>

  <div class="list-container">
    {#if loading}
      <div class="msg">Loading...</div>
    {:else if history.sessions.length === 0}
      <div class="msg">No sessions found.</div>
    {:else}
      <table class="table">
        <thead>
          <tr>
            <th>Date</th>
            <th>Type</th>
            <th>Duration</th>
            <th>Subject</th>
            <th>Topic</th>
            <th>Study Type</th>
          </tr>
        </thead>
        <tbody>
          {#each history.sessions as row}
            <tr onclick={() => onEditSession(row.id)}>
              <td>{formatUnix(row.started_at)}</td>
              <td>{row.round_type === 'work' ? 'Work' : row.round_type === 'short-break' ? 'Short Break' : 'Long Break'}</td>
              <td>{formatDuration(row.duration_secs)}</td>
              <td>{row.subject || '-'}</td>
              <td>{row.subject_topic || '-'}</td>
              <td>{row.study_type || '-'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  <div class="pagination">
    <button disabled={offset === 0} onclick={() => offset = Math.max(0, offset - limit)}>Previous</button>
    <span class="page-info">
      Showing {Math.min(offset + 1, history.total)} - {Math.min(offset + limit, history.total)} of {history.total}
    </span>
    <button disabled={offset + limit >= history.total} onclick={() => offset += limit}>Next</button>
  </div>
</div>

<style>
  .history-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
    gap: 16px;
    box-sizing: border-box;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .filter-row {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }

  .custom-input {
    background: var(--color-hover);
    color: var(--color-foreground);
    border: 1px solid var(--color-separator);
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 0.85rem;
    outline: none;
    flex: 1;
    min-width: 120px;
  }

  .custom-select {
    appearance: none;
    -webkit-appearance: none;
    background-color: var(--color-hover);
    background-image: url("/src/lib/assets/dropdown.svg");
    background-repeat: no-repeat;
    background-position: right 10px top 50%;
    background-size: 10px auto;
    color: var(--color-foreground);
    border: 1px solid var(--color-separator);
    padding: 6px 30px 6px 12px;
    border-radius: 4px;
    font-size: 0.85rem;
    outline: none;
    flex: 1;
    min-width: 120px;
  }

  .custom-select option {
    background: var(--color-background);
    color: var(--color-foreground);
  }

  .date-input {
    flex: none;
    min-width: 130px;
    width: 130px;
  }

  .quick-select {
    display: flex;
    gap: 4px;
  }

  .quick-select button {
    background: var(--color-hover);
    color: var(--color-foreground);
    border: 1px solid var(--color-separator);
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
  }

  .quick-select button.active {
    background: var(--color-focus-round);
    color: var(--color-background);
    border-color: var(--color-focus-round);
  }

  .date-range {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
    color: var(--color-foreground-darker);
  }

  .nav-btn {
    background: none;
    border: none;
    color: var(--color-foreground-darker);
    cursor: pointer;
    padding: 4px;
  }

  .nav-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .list-container {
    flex: 1;
    overflow-y: auto;
    background: transparent;
    border: 1px solid var(--color-separator);
    border-radius: 6px;
  }

  .msg {
    padding: 24px;
    text-align: center;
    color: var(--color-foreground-darker);
  }

  .table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .table th, .table td {
    padding: 10px 16px;
    text-align: left;
    border-bottom: 1px solid var(--color-separator);
  }

  .table th {
    background: var(--color-background);
    color: var(--color-foreground-darker);
    font-weight: 600;
    position: sticky;
    top: 0;
  }

  .table tbody tr {
    cursor: pointer;
    transition: background 0.1s;
  }

  .table tbody tr:hover {
    background: var(--color-hover);
  }

  .pagination {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.85rem;
    color: var(--color-foreground-darker);
  }

  .pagination button {
    background: var(--color-hover);
    color: var(--color-foreground);
    border: 1px solid var(--color-separator);
    padding: 6px 16px;
    border-radius: 4px;
    cursor: pointer;
  }

  .pagination button:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
