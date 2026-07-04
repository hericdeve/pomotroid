<script lang="ts">
  import { onMount } from 'svelte';
  import { statsGetInsights } from '$lib/ipc';
  import type { InsightsStats, SessionFilter } from '$lib/types';

  let insights = $state<InsightsStats | null>(null);
  let loading = $state(false);

  let dateFromStr = $state<string>('');
  let dateToStr = $state<string>('');
  let quickMode = $state<'day' | 'week' | 'year' | 'all'>('all');

  let subjectGraphType = $state<'donut' | 'bar'>('donut');

  const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  const colors = [
    '#f43f5e', '#ec4899', '#d946ef', '#a855f7', '#8b5cf6', '#6366f1', 
    '#3b82f6', '#0ea5e9', '#06b6d4', '#14b8a6', '#10b981', '#22c55e', 
    '#84cc16', '#eab308', '#f59e0b', '#f97316', '#ef4444'
  ];

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
        show_breaks: false
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

  // Derived state for rendering
  const maxSubjectTime = $derived(insights && insights.top_subjects.length > 0 ? insights.top_subjects[0].focus_secs : 1);
  const totalSubjectTime = $derived(insights ? insights.top_subjects.reduce((s, x) => s + x.focus_secs, 0) : 1);
  
  const maxDayTime = $derived(insights ? Math.max(1, ...insights.by_day_of_week) : 1);
  const maxHourTime = $derived(insights ? Math.max(1, ...insights.by_hour_of_day) : 1);

  // SVG parameters
  const CHART_H = 160;
  const r = 64; // Donut radius
  const c = 2 * Math.PI * r;

</script>

<div class="view insights-view">
  
  <div class="filter-bar">
    <div class="date-controls">
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
    </div>
  </div>

  {#if loading && !insights}
    <div class="loading">Loading...</div>
  {:else if insights}
    <div class="insights-dashboard">
      
      <!-- Subjects Card -->
      <div class="card card-wide">
        <div class="card-header">
          <h3>Focus Time by Subject</h3>
          <div class="toggles">
            <button class="btn-toggle" aria-label="Donut Chart View" class:active={subjectGraphType === 'donut'} onclick={() => subjectGraphType = 'donut'}>
              <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2" fill="none"><circle cx="12" cy="12" r="10"/><path d="M12 2v10l7.5-7.5"/></svg>
            </button>
            <button class="btn-toggle" aria-label="Bar Chart View" class:active={subjectGraphType === 'bar'} onclick={() => subjectGraphType = 'bar'}>
              <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2" fill="none"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="15" y2="12"/><line x1="3" y1="18" x2="9" y2="18"/></svg>
            </button>
          </div>
        </div>
        
        {#if insights.top_subjects.length === 0}
          <div class="empty">No data in this period</div>
        {:else if subjectGraphType === 'donut'}
          <div class="donut-container">
            <svg class="donut-svg" viewBox="0 0 160 160">
              <!-- Background ring -->
              <circle cx="80" cy="80" r={r} fill="none" stroke="var(--color-background)" stroke-width="24" />
              <!-- Slices -->
              {#each insights.top_subjects as s, i}
                {@const prevOffset = insights.top_subjects.slice(0, i).reduce((sum, item) => sum + (item.focus_secs / totalSubjectTime) * c, 0)}
                {@const sliceLen = (s.focus_secs / totalSubjectTime) * c}
                <circle 
                  cx="80" 
                  cy="80" 
                  r={r} 
                  fill="none" 
                  stroke={colors[i % colors.length]} 
                  stroke-width="24" 
                  stroke-dasharray="{sliceLen} {c}"
                  stroke-dashoffset={-prevOffset}
                  transform="rotate(-90 80 80)"
                  class="donut-slice"
                />
              {/each}
            </svg>
            <div class="donut-legend">
              {#each insights.top_subjects as s, i}
                <div class="legend-item">
                  <span class="legend-dot" style="background: {colors[i % colors.length]}"></span>
                  <span class="legend-label" title={s.subject}>{s.subject}</span>
                  <span class="legend-val">{fmtTime(s.focus_secs)}</span>
                </div>
              {/each}
            </div>
          </div>
        {:else}
          <div class="bar-list">
            {#each insights.top_subjects as s, i}
              {@const pct = (s.focus_secs / maxSubjectTime) * 100}
              <div class="bar-item">
                <div class="bar-label">
                  <span class="name" title={s.subject}>{s.subject}</span>
                  <span class="val">{fmtTime(s.focus_secs)}</span>
                </div>
                <div class="bar-track">
                  <div class="bar-fill" style="width: {Math.max(1, pct)}%; background: {colors[i % colors.length]}"></div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Day of Week Card -->
      <div class="card">
        <div class="card-header">
          <h3>Time by Day of Week</h3>
        </div>
        <div class="vertical-chart-container">
          <svg width="280" height="{CHART_H + 40}" viewBox="0 0 280 {CHART_H + 40}" class="vertical-svg">
            {#each insights.by_day_of_week as secs, i}
              {@const h = Math.max(2, (secs / maxDayTime) * CHART_H)}
              {@const x = i * 40 + 10}
              {@const y = CHART_H - h}
              
              <!-- Hoverable Group -->
              <g class="chart-group">
                <rect {x} {y} width="20" height={h} fill={colors[i % colors.length]} rx="3" class="v-bar" />
                <text x={x + 10} y={CHART_H + 18} text-anchor="middle" class="axis-label">{days[i]}</text>
                
                <!-- Tooltip Overlay (CSS managed) -->
                <rect x={i * 40} y="0" width="40" height={CHART_H + 40} fill="transparent" class="hover-area" />
                <g class="tooltip">
                  <rect x={x + 10} y={y - 30} width="60" height="24" rx="4" fill="var(--color-background)" transform="translate(-30, 0)" />
                  <text x={x + 10} y={y - 14} text-anchor="middle" class="tooltip-text">{fmtTime(secs)}</text>
                </g>
              </g>
            {/each}
          </svg>
        </div>
      </div>

      <!-- Hour of Day Card -->
      <div class="card">
        <div class="card-header">
          <h3>Time by Hour of Day</h3>
        </div>
        <div class="vertical-chart-container hour-chart-wrap">
          <svg width="528" height="{CHART_H + 40}" viewBox="0 0 528 {CHART_H + 40}" class="vertical-svg">
            {#each insights.by_hour_of_day as secs, i}
              {@const h = Math.max(2, (secs / maxHourTime) * CHART_H)}
              {@const x = i * 22 + 4}
              {@const y = CHART_H - h}
              
              <g class="chart-group">
                <rect {x} {y} width="14" height={h} fill={colors[(i + 4) % colors.length]} rx="2" class="v-bar" />
                {#if i % 4 === 0}
                  <text x={x + 7} y={CHART_H + 18} text-anchor="middle" class="axis-label">
                    {i === 0 ? '12A' : i < 12 ? `${i}A` : i === 12 ? '12P' : `${i-12}P`}
                  </text>
                {/if}
                
                <rect x={i * 22} y="0" width="22" height={CHART_H + 40} fill="transparent" class="hover-area" />
                <g class="tooltip">
                  <rect x={x + 7} y={y - 30} width="50" height="24" rx="4" fill="var(--color-background)" transform="translate(-25, 0)" />
                  <text x={x + 7} y={y - 14} text-anchor="middle" class="tooltip-text">{fmtTime(secs)}</text>
                </g>
              </g>
            {/each}
          </svg>
        </div>
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
    background: var(--color-background);
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

  .insights-dashboard {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 20px;
  }
  
  .card {
    background: var(--color-background);
    border-radius: 12px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .card-wide {
    grid-column: 1 / -1;
  }
  
  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .card-header h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--color-foreground);
    font-weight: 600;
  }
  
  .toggles {
    display: flex;
    gap: 4px;
    background: var(--color-background);
    border-radius: 6px;
    padding: 4px;
  }
  .btn-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px 12px;
    border: none;
    background: transparent;
    border-radius: 4px;
    color: var(--color-foreground-darker);
    cursor: pointer;
  }
  .btn-toggle:hover {
    color: var(--color-foreground);
  }
  .btn-toggle.active {
    background: var(--color-focus-round);
    color: var(--color-background);
  }
  
  .empty {
    color: var(--color-foreground-darker);
    font-size: 0.9rem;
    text-align: center;
    padding: 20px 0;
  }
  
  /* Horizontal Bar List */
  .bar-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: 240px;
    overflow-y: auto;
    padding-right: 10px;
  }
  .bar-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
    animation: barFadeIn 0.4s ease forwards;
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
    color: var(--color-foreground);
    font-weight: 500;
  }
  .val {
    color: var(--color-foreground-darker);
  }
  .bar-track {
    width: 100%;
    height: 8px;
    background: var(--color-background);
    border-radius: 4px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    border-radius: 4px;
    transform-origin: left;
    animation: barGrow 0.6s cubic-bezier(0.2, 0.8, 0.2, 1) forwards;
  }
  @keyframes barGrow {
    from { transform: scaleX(0); }
    to { transform: scaleX(1); }
  }

  /* Donut Chart */
  .donut-container {
    display: flex;
    align-items: center;
    gap: 40px;
    padding: 20px 0;
  }
  .donut-svg {
    width: 200px;
    height: 200px;
    flex-shrink: 0;
    overflow: visible;
  }
  .donut-slice {
    transition: stroke-dasharray 0.6s ease, stroke-dashoffset 0.6s ease, transform 0.2s ease;
  }
  .donut-slice:hover {
    stroke-width: 28;
    cursor: pointer;
  }
  
  .donut-legend {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex-grow: 1;
    max-height: 200px;
    overflow-y: auto;
    padding-right: 10px;
  }
  .legend-item {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 0.95rem;
  }
  .legend-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .legend-label {
    color: var(--color-foreground);
    flex-grow: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .legend-val {
    color: var(--color-foreground-darker);
    font-weight: 500;
  }

  /* Vertical SVGs */
  .vertical-chart-container {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: flex-end;
    overflow-x: auto;
    padding-top: 36px;
  }
  .vertical-svg {
    display: block;
    overflow: visible;
  }
  .hour-chart-wrap .vertical-svg {
    /* Auto horizontal scroll driven by actual pixel width */
  }
  
  .v-bar {
    transition: y 0.6s ease, height 0.6s ease, filter 0.2s;
  }
  .chart-group {
    cursor: pointer;
  }
  .chart-group:hover .v-bar {
    filter: brightness(1.2);
  }
  
  .axis-label {
    font-size: 11px;
    fill: var(--color-foreground-darker);
    user-select: none;
  }
  
  /* Tooltips */
  .tooltip {
    opacity: 0;
    transition: opacity 0.15s;
    pointer-events: none;
  }
  .tooltip-text {
    font-size: 11px;
    font-weight: 600;
    fill: var(--color-foreground);
  }
  .chart-group:hover .tooltip {
    opacity: 1;
  }

  @media (max-width: 800px) {
    .insights-dashboard {
      grid-template-columns: 1fr;
    }
    .donut-container {
      flex-direction: column;
      gap: 20px;
    }
    .donut-svg {
      width: 160px;
      height: 160px;
    }
  }
</style>
