<script lang="ts">
  import { onMount } from 'svelte';
  import { statsGetInsights } from '$lib/ipc';
  import type { InsightsStats, SessionFilter } from '$lib/types';
  import * as m from '$paraglide/messages.js';

  let insights = $state<InsightsStats | null>(null);
  let loading = $state(false);

  let dateFromStr = $state<string>('');
  let dateToStr = $state<string>('');
  let quickMode = $state<'day' | 'week' | 'year' | 'all'>('all');

  function fmtTime(secs: number): string {
    const mins = Math.floor(secs / 60);
    if (mins < 60) return `${mins}m`;
    const h = Math.floor(mins / 60);
    const m2 = mins % 60;
    return m2 === 0 ? `${h}h` : `${h}h ${m2}m`;
  }

  async function loadInsights() {
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
        subject: null,
        subject_topic: null,
        study_type: null,
        date_from,
        date_to,
        show_breaks: false // Insights are for work only
      };

      insights = await statsGetInsights(filter);
    } catch (e) {
      console.error('Failed to load insights', e);
    } finally {
      loading = false;
    }
  }

  function setDateStr(d: Date, field: 'from' | 'to') {
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
      const day = start.getDay();
      const diff = start.getDate() - day + (day === 0 ? -6 : 1);
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

  $effect(() => {
    // Triggers when dates change
    const d1 = dateFromStr;
    const d2 = dateToStr;
    loadInsights();
  });

  onMount(() => {
    loadInsights();
  });

  const maxSubjectTime = $derived(insights && insights.top_subjects.length > 0 ? insights.top_subjects[0].focus_secs : 1);
  const maxTopicTime = $derived(insights && insights.top_topics.length > 0 ? insights.top_topics[0].focus_secs : 1);
  const maxStudyTypeTime = $derived(insights && insights.top_study_types.length > 0 ? insights.top_study_types[0].focus_secs : 1);

</script>

<div class="view insights-view">
  
  <!-- Date Filter Bar -->
  <div class="filter-bar">
    <div class="date-controls">
      <div class="quick-modes">
        <button class="btn btn-small" class:active={quickMode === 'day'} onclick={() => applyQuickMode('day')}>Day</button>
        <button class="btn btn-small" class:active={quickMode === 'week'} onclick={() => applyQuickMode('week')}>Week</button>
        <button class="btn btn-small" class:active={quickMode === 'year'} onclick={() => applyQuickMode('year')}>Year</button>
        <button class="btn btn-small" class:active={quickMode === 'all'} onclick={() => applyQuickMode('all')}>All Time</button>
      </div>

      {#if quickMode !== 'all'}
        <div class="date-shifters">
          <button class="btn btn-icon btn-small" aria-label="Previous Period" onclick={() => shiftPeriod(-1)}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 18l-6-6 6-6" /></svg>
          </button>
          
          <div class="custom-dates">
            <input type="date" bind:value={dateFromStr} class="date-input" aria-label="From Date" />
            <span class="date-sep">to</span>
            <input type="date" bind:value={dateToStr} class="date-input" aria-label="To Date" />
          </div>

          <button class="btn btn-icon btn-small" aria-label="Next Period" onclick={() => shiftPeriod(1)}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18l6-6-6-6" /></svg>
          </button>
        </div>
      {/if}
    </div>
  </div>

  {#if loading && !insights}
    <div class="loading">Loading...</div>
  {:else if insights}
    <div class="insights-grid">
      <!-- Subjects -->
      <div class="insight-card">
        <h3>Time by Subject</h3>
        {#if insights.top_subjects.length === 0}
          <div class="empty">No data in this period</div>
        {:else}
          <div class="bar-list">
            {#each insights.top_subjects as s, i}
              {@const pct = (s.focus_secs / maxSubjectTime) * 100}
              <div class="bar-item" style="--delay: {i * 50}ms">
                <div class="bar-label">
                  <span class="name" title={s.subject}>{s.subject}</span>
                  <span class="val">{fmtTime(s.focus_secs)}</span>
                </div>
                <div class="bar-track">
                  <div class="bar-fill" style="width: {Math.max(2, pct)}%"></div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Topics -->
      <div class="insight-card">
        <h3>Time by Topic</h3>
        {#if insights.top_topics.length === 0}
          <div class="empty">No data in this period</div>
        {:else}
          <div class="bar-list">
            {#each insights.top_topics as s, i}
              {@const pct = (s.focus_secs / maxTopicTime) * 100}
              <div class="bar-item" style="--delay: {i * 50}ms">
                <div class="bar-label">
                  <span class="name" title={s.topic}>{s.topic}</span>
                  <span class="val">{fmtTime(s.focus_secs)}</span>
                </div>
                <div class="bar-track">
                  <div class="bar-fill" style="width: {Math.max(2, pct)}%"></div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Study Types -->
      <div class="insight-card">
        <h3>Time by Study Type</h3>
        {#if insights.top_study_types.length === 0}
          <div class="empty">No data in this period</div>
        {:else}
          <div class="bar-list">
            {#each insights.top_study_types as s, i}
              {@const pct = (s.focus_secs / maxStudyTypeTime) * 100}
              <div class="bar-item" style="--delay: {i * 50}ms">
                <div class="bar-label">
                  <span class="name" title={s.study_type}>{s.study_type}</span>
                  <span class="val">{fmtTime(s.focus_secs)}</span>
                </div>
                <div class="bar-track">
                  <div class="bar-fill" style="width: {Math.max(2, pct)}%"></div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

    </div>
  {/if}
</div>

<style>
  .insights-view {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding-bottom: 40px;
  }
  
  .filter-bar {
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: var(--color-bg-secondary);
    padding: 12px 16px;
    border-radius: 8px;
  }
  .date-controls {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }
  .quick-modes {
    display: flex;
    gap: 4px;
    background: var(--color-bg);
    padding: 4px;
    border-radius: 6px;
  }
  .btn-small {
    padding: 4px 10px;
    font-size: 0.85rem;
    border: none;
    background: transparent;
    color: var(--color-text-secondary);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
  }
  .btn-small:hover {
    color: var(--color-text);
    background: var(--color-bg-hover);
  }
  .btn-small.active {
    background: var(--color-primary);
    color: var(--color-bg);
    font-weight: 600;
  }
  .btn-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
  }
  .btn-icon svg {
    width: 16px;
    height: 16px;
  }
  .date-shifters {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--color-bg);
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
    border: 1px solid var(--color-border);
    color: var(--color-text);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.85rem;
  }
  .date-sep {
    font-size: 0.85rem;
    color: var(--color-text-secondary);
  }

  .insights-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 20px;
  }
  
  .insight-card {
    background: var(--color-bg-secondary);
    border-radius: 12px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .insight-card h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--color-text);
    font-weight: 600;
  }
  .empty {
    color: var(--color-text-secondary);
    font-size: 0.9rem;
    text-align: center;
    padding: 20px 0;
  }
  
  .bar-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  
  .bar-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
    animation: barFadeIn 0.4s ease forwards;
    animation-delay: var(--delay, 0ms);
    opacity: 0;
  }
  
  @keyframes barFadeIn {
    from { opacity: 0; transform: translateY(5px); }
    to { opacity: 1; transform: translateY(0); }
  }
  
  .bar-label {
    display: flex;
    justify-content: space-between;
    font-size: 0.9rem;
  }
  .name {
    color: var(--color-text);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 70%;
  }
  .val {
    color: var(--color-text-secondary);
  }
  
  .bar-track {
    width: 100%;
    height: 8px;
    background: var(--color-bg);
    border-radius: 4px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: var(--color-primary);
    border-radius: 4px;
    transform-origin: left;
    animation: barGrow 0.6s cubic-bezier(0.2, 0.8, 0.2, 1) forwards;
    animation-delay: calc(var(--delay, 0ms) + 100ms);
    transform: scaleX(0);
  }
  
  @keyframes barGrow {
    to { transform: scaleX(1); }
  }
</style>
