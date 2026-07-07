<script lang="ts">
  import type { HeatmapStats, HeatmapEntry } from '$lib/types';
  import { settings } from '$lib/stores/settings';
  import * as m from '$paraglide/messages.js';

  interface Props {
    heatmap: HeatmapStats | null;
  }

  let { heatmap }: Props = $props();

  // ── Date Helpers (Strict UTC to avoid DST bugs) ────────────────────────────

  function getDaysInYear(year: number): number {
    return (year % 4 === 0 && year % 100 !== 0) || (year % 400 === 0) ? 366 : 365;
  }

  function getDaysInHalf(year: number, isH2: boolean): number {
    const h2StartMonth = ($settings.half_year_start_month || 7) - 1; // 0-indexed, default July
    const startOfH2 = new Date(Date.UTC(year, h2StartMonth, 1));
    const startOfNextYear = new Date(Date.UTC(year + 1, 0, 1));
    const startOfYear = new Date(Date.UTC(year, 0, 1));

    if (isH2) {
      return Math.round((startOfNextYear.getTime() - startOfH2.getTime()) / (1000 * 60 * 60 * 24));
    } else {
      return Math.round((startOfH2.getTime() - startOfYear.getTime()) / (1000 * 60 * 60 * 24));
    }
  }

  function getDayOfYear(dateString: string): number {
    const d = new Date(`${dateString}T00:00:00Z`);
    const start = new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
    const diff = d.getTime() - start.getTime();
    return Math.floor(diff / (1000 * 60 * 60 * 24)); // 0-indexed day
  }

  function getDayOfHalf(dateString: string): number {
    const d = new Date(`${dateString}T00:00:00Z`);
    const h2StartMonth = ($settings.half_year_start_month || 7) - 1; // 0-indexed, default July
    const isH2 = d.getUTCMonth() >= h2StartMonth;
    const start = isH2 ? new Date(Date.UTC(d.getUTCFullYear(), h2StartMonth, 1)) : new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
    const diff = d.getTime() - start.getTime();
    return Math.floor(diff / (1000 * 60 * 60 * 24)); // 0-indexed day
  }

  function getHalfStr(dateString: string): string {
    const d = new Date(`${dateString}T00:00:00Z`);
    const h2StartMonth = ($settings.half_year_start_month || 7) - 1; // 0-indexed, default July
    return `${d.getUTCFullYear()}-H${d.getUTCMonth() >= h2StartMonth ? '2' : '1'}`;
  }

  function formatDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return h > 0 ? `${h}h ${m}m` : `${m}m`;
  }

  // ── Data Processing ────────────────────────────────────────────────────────

  interface PeriodStats {
    totalSecs: number;
    totalRounds: number;
    activeDays: number;
    daysInPeriod: number;
  }

  const yearlyData = $derived(() => {
    if (!heatmap?.entries) return new Map<number, HeatmapEntry[]>();
    const map = new Map<number, HeatmapEntry[]>();
    for (const e of heatmap.entries) {
      const y = parseInt(e.date.split('-')[0], 10);
      if (!map.has(y)) map.set(y, []);
      map.get(y)!.push(e);
    }
    return map;
  });

  const halfData = $derived(() => {
    if (!heatmap?.entries) return new Map<string, HeatmapEntry[]>();
    const map = new Map<string, HeatmapEntry[]>();
    for (const e of heatmap.entries) {
      const h = getHalfStr(e.date);
      if (!map.has(h)) map.set(h, []);
      map.get(h)!.push(e);
    }
    return map;
  });

  // Current states
  const todayStr = new Date().toLocaleDateString('en-CA'); // Local time bounds our current context!
  const currentYear = parseInt(todayStr.split('-')[0], 10);
  const currentHalf = getHalfStr(todayStr);
  const currentDayOfYear = getDayOfYear(todayStr);
  const currentDayOfHalf = getDayOfHalf(todayStr);
  
  const curDaysInYear = getDaysInYear(currentYear);
  const curDaysInHalf = getDaysInHalf(currentYear, currentHalf.endsWith('H2'));
  const curYearPct = currentDayOfYear / curDaysInYear;
  const curHalfPct = currentDayOfHalf / curDaysInHalf;

  function calculateStats(entries: HeatmapEntry[] | undefined, cutoffPct: number, getDaysInPeriod: number, getDayOfPeriod: (d: string) => number): PeriodStats {
    if (!entries) return { totalSecs: 0, totalRounds: 0, activeDays: 0, daysInPeriod: getDaysInPeriod };
    let secs = 0, rounds = 0, days = 0;
    for (const e of entries) {
      const dayIdx = getDayOfPeriod(e.date);
      const pct = dayIdx / getDaysInPeriod;
      if (pct <= cutoffPct) {
        secs += e.focus_secs;
        rounds += e.count;
        if (e.focus_secs > 0) days++;
      }
    }
    return { totalSecs: secs, totalRounds: rounds, activeDays: days, daysInPeriod: getDaysInPeriod };
  }

  // Calculate year stats up to today's YEAR PERCENTAGE
  const yearStatsPaced = $derived(() => {
    const stats = new Map<number, PeriodStats>();
    for (const [y, entries] of yearlyData().entries()) {
      stats.set(y, calculateStats(entries, curYearPct, getDaysInYear(y), getDayOfYear));
    }
    return stats;
  });

  // Calculate half stats up to today's HALF PERCENTAGE
  const halfStatsPaced = $derived(() => {
    const stats = new Map<string, PeriodStats>();
    for (const [h, entries] of halfData().entries()) {
      const y = parseInt(h.split('-')[0], 10);
      const isH2 = h.endsWith('H2');
      stats.set(h, calculateStats(entries, curHalfPct, getDaysInHalf(y, isH2), getDayOfHalf));
    }
    return stats;
  });

  // Calculate TOTAL historical stats to find the true "Best" overall.
  function sumAllStats(entries: HeatmapEntry[] | undefined): number {
    if (!entries) return 0;
    return entries.reduce((sum, e) => sum + e.focus_secs, 0);
  }

  const bestYearInfo = $derived(() => {
    let bestY = currentYear - 1;
    let maxSecs = -1;
    for (const [y, entries] of yearlyData().entries()) {
      if (y === currentYear) continue;
      const total = sumAllStats(entries);
      if (total > maxSecs) { maxSecs = total; bestY = y; }
    }
    return { 
      year: bestY, 
      stats: yearStatsPaced().get(bestY) || { totalSecs: 0, totalRounds: 0, activeDays: 0, daysInPeriod: getDaysInYear(bestY) }
    };
  });

  const [curHYear, curHNum] = currentHalf.split('-H');
  const prevHalfStr = curHNum === '2' ? `${curHYear}-H1` : `${parseInt(curHYear)-1}-H2`;
  
  const bestHalfInfo = $derived(() => {
    let bestH = prevHalfStr;
    let maxSecs = -1;
    for (const [h, entries] of halfData().entries()) {
      if (h === currentHalf) continue;
      const total = sumAllStats(entries);
      if (total > maxSecs) { maxSecs = total; bestH = h; }
    }
    const [by, bh] = bestH.split('-H');
    return { 
      half: bestH, 
      stats: halfStatsPaced().get(bestH) || { totalSecs: 0, totalRounds: 0, activeDays: 0, daysInPeriod: getDaysInHalf(parseInt(by, 10), bh === '2') }
    };
  });

  // ── Current Pacing Stats ───────────────────────────────────────────────────

  const curYearStats = $derived(yearStatsPaced().get(currentYear) || { totalSecs: 0, totalRounds: 0, activeDays: 0, daysInPeriod: curDaysInYear });
  const prevYearStats = $derived(yearStatsPaced().get(currentYear - 1) || { totalSecs: 0, totalRounds: 0, activeDays: 0, daysInPeriod: getDaysInYear(currentYear - 1) });
  
  const curHalfStats = $derived(halfStatsPaced().get(currentHalf) || { totalSecs: 0, totalRounds: 0, activeDays: 0, daysInPeriod: curDaysInHalf });
  const prevHalfStats = $derived(halfStatsPaced().get(prevHalfStr) || { totalSecs: 0, totalRounds: 0, activeDays: 0, daysInPeriod: getDaysInHalf(parseInt(prevHalfStr.split('-')[0], 10), prevHalfStr.endsWith('H2')) });

  // ── Best Day / Month ───────────────────────────────────────────────────────
  
  const bestDay = $derived(() => {
    if (!heatmap?.entries || heatmap.entries.length === 0) return null;
    return heatmap.entries.reduce((best, cur) => cur.focus_secs > best.focus_secs ? cur : best);
  });

  const bestMonth = $derived(() => {
    if (!heatmap?.entries) return null;
    const monthMap = new Map<string, number>();
    for (const e of heatmap.entries) {
      const m = e.date.substring(0, 7); // YYYY-MM
      monthMap.set(m, (monthMap.get(m) || 0) + e.focus_secs);
    }
    let maxMonth = '', maxSecs = -1;
    for (const [m, secs] of monthMap.entries()) {
      if (secs > maxSecs) { maxSecs = secs; maxMonth = m; }
    }
    return { month: maxMonth, secs: maxSecs };
  });

  // Forecast using actual days in year
  const projectedYearSecs = $derived(() => {
    const elapsedDays = currentDayOfYear + 1; // 0-indexed to count
    if (elapsedDays <= 0) return 0;
    const avgDaily = curYearStats.totalSecs / elapsedDays;
    return avgDaily * curDaysInYear;
  });

  // Chart data
  function getCumulativeYear(year: number): number[] {
    const totalDays = getDaysInYear(year);
    const arr = new Array(totalDays).fill(0);
    const entries = yearlyData().get(year);
    if (entries) {
      for (const e of entries) {
        const d = getDayOfYear(e.date);
        if (d >= 0 && d < totalDays) {
          arr[d] += e.focus_secs;
        }
      }
    }
    for (let i = 1; i < totalDays; i++) {
      arr[i] += arr[i-1];
    }
    return arr;
  }

  const chartData = $derived(() => {
    return {
      current: getCumulativeYear(currentYear).slice(0, currentDayOfYear + 1),
      previous: getCumulativeYear(currentYear - 1),
      best: getCumulativeYear(bestYearInfo().year)
    };
  });

  // ── Render Helpers ─────────────────────────────────────────────────────────

  function pctChange(cur: number, past: number): string {
    if (past === 0) return cur > 0 ? '—' : '—'; // Prevent infinite +100% inflation
    if (cur === 0 && past === 0) return '—';
    const diff = ((cur - past) / past) * 100;
    if (diff === 0) return '0%';
    return (diff > 0 ? '+' : '') + diff.toFixed(1) + '%';
  }

  function getTrend(cur: number, past: number): string {
    if (past === 0 || cur === past) return '';
    return cur > past ? '▲' : '▼';
  }

  function isPos(cur: number, past: number): boolean {
    if (past === 0 || cur === past) return false;
    return cur > past;
  }

  function formatRounds(r: number): string {
    const rounded = Math.round(r * 10) / 10;
    return rounded.toString();
  }

  // Chart rendering constants
  const CHART_W = 800;
  const CHART_H = 150;
  
  const chartMaxVal = $derived(() => {
    const data = chartData();
    let m = 1;
    if (data.current.length > 0) m = Math.max(m, data.current[data.current.length - 1]);
    if (data.previous.length > 0) m = Math.max(m, data.previous[data.previous.length - 1]);
    if (data.best.length > 0) m = Math.max(m, data.best[data.best.length - 1]);
    return m;
  });

  function makePath(points: number[]): string {
    if (points.length === 0) return '';
    const maxMins = Math.max(1, chartMaxVal() / 60);
    const totalDays = points.length < 366 ? 365 : 366;
    let d = '';
    for (let i = 0; i < points.length; i++) {
      const x = (i / (totalDays - 1)) * CHART_W;
      const h = ((points[i]/60) / maxMins) * CHART_H;
      const y = CHART_H - h;
      d += i === 0 ? `M ${x} ${y} ` : `L ${x} ${y} `;
    }
    return d;
  }
</script>

<div class="comparisons-dashboard">
  {#if heatmap}
    <!-- Top Summary -->
    <div class="top-summary">
      <div class="summary-details">
        <h3>So far in {currentYear} <span style="font-weight: 400; font-size: 0.9em; opacity: 0.7">(Day {currentDayOfYear + 1})</span></h3>
        <div class="summary-stats">
          <span><strong>{formatDuration(curYearStats.totalSecs)}</strong> focused</span>
          <span class="sep">•</span>
          <span><strong>{formatRounds(curYearStats.totalRounds)}</strong> rounds</span>
          <span class="sep">•</span>
          <span><strong>{Math.round((curYearStats.activeDays / Math.max(1, currentDayOfYear + 1)) * 100)}%</strong> consistency</span>
        </div>
      </div>
      <div class="forecast-box">
        <div class="forecast-icon">🚀</div>
        <div class="forecast-text">
          <strong>Forecast:</strong> If you maintain your current daily average, you will finish {currentYear} with <strong>{formatDuration(projectedYearSecs())}</strong> focused.
        </div>
      </div>
    </div>

    <!-- Comparison Cards -->
    <div class="cards-grid">
      <!-- Card 1: Prev Year -->
      <div class="card">
        <div class="card-title">vs. Previous Year ({currentYear - 1})</div>
        <div class="card-subtitle">At this day ({currentDayOfYear + 1} / {curDaysInYear})</div>
        <div class="card-metrics">
          <div class="metric-row">
            <span class="m-label">Hours</span>
            <span class="m-val">{formatDuration(curYearStats.totalSecs)}</span>
            <span class="m-diff" class:pos={isPos(curYearStats.totalSecs, prevYearStats.totalSecs)} class:neg={curYearStats.totalSecs < prevYearStats.totalSecs}>
              {pctChange(curYearStats.totalSecs, prevYearStats.totalSecs)} {getTrend(curYearStats.totalSecs, prevYearStats.totalSecs)}
            </span>
          </div>
          <div class="metric-row">
            <span class="m-label">Rounds</span>
            <span class="m-val">{formatRounds(curYearStats.totalRounds)}</span>
            <span class="m-diff" class:pos={isPos(curYearStats.totalRounds, prevYearStats.totalRounds)} class:neg={curYearStats.totalRounds < prevYearStats.totalRounds}>
              {pctChange(curYearStats.totalRounds, prevYearStats.totalRounds)} {getTrend(curYearStats.totalRounds, prevYearStats.totalRounds)}
            </span>
          </div>
          <div class="metric-row">
            <span class="m-label">Consistency</span>
            <span class="m-val">{Math.round((curYearStats.activeDays / (currentDayOfYear + 1)) * 100)}%</span>
            <span class="m-diff" class:pos={isPos(curYearStats.activeDays, prevYearStats.activeDays)} class:neg={curYearStats.activeDays < prevYearStats.activeDays}>
              {pctChange(curYearStats.activeDays, prevYearStats.activeDays)} {getTrend(curYearStats.activeDays, prevYearStats.activeDays)}
            </span>
          </div>
        </div>
      </div>

      <!-- Card 2: Best Year -->
      <div class="card">
        <div class="card-title">vs. Best Year ({bestYearInfo().year})</div>
        <div class="card-subtitle">At this day ({Math.min(currentDayOfYear + 1, bestYearInfo().stats.daysInPeriod)} / {bestYearInfo().stats.daysInPeriod})</div>
        <div class="card-metrics">
          <div class="metric-row">
            <span class="m-label">Hours</span>
            <span class="m-val">{formatDuration(curYearStats.totalSecs)}</span>
            <span class="m-diff" class:pos={isPos(curYearStats.totalSecs, bestYearInfo().stats.totalSecs)} class:neg={curYearStats.totalSecs < bestYearInfo().stats.totalSecs}>
              {pctChange(curYearStats.totalSecs, bestYearInfo().stats.totalSecs)} {getTrend(curYearStats.totalSecs, bestYearInfo().stats.totalSecs)}
            </span>
          </div>
          <div class="metric-row">
            <span class="m-label">Rounds</span>
            <span class="m-val">{formatRounds(curYearStats.totalRounds)}</span>
            <span class="m-diff" class:pos={isPos(curYearStats.totalRounds, bestYearInfo().stats.totalRounds)} class:neg={curYearStats.totalRounds < bestYearInfo().stats.totalRounds}>
              {pctChange(curYearStats.totalRounds, bestYearInfo().stats.totalRounds)} {getTrend(curYearStats.totalRounds, bestYearInfo().stats.totalRounds)}
            </span>
          </div>
          <div class="metric-row">
            <span class="m-label">Consistency</span>
            <span class="m-val">{Math.round((curYearStats.activeDays / (currentDayOfYear + 1)) * 100)}%</span>
            <span class="m-diff" class:pos={isPos(curYearStats.activeDays, bestYearInfo().stats.activeDays)} class:neg={curYearStats.activeDays < bestYearInfo().stats.activeDays}>
              {pctChange(curYearStats.activeDays, bestYearInfo().stats.activeDays)} {getTrend(curYearStats.activeDays, bestYearInfo().stats.activeDays)}
            </span>
          </div>
        </div>
      </div>

      <!-- Card 3: Prev Half -->
      <div class="card">
        <div class="card-title">vs. Previous Half ({prevHalfStr})</div>
        <div class="card-subtitle">At Day {currentDayOfHalf + 1} ({Math.round(curHalfPct * 100)}% of Half)</div>
        <div class="card-metrics">
          <div class="metric-row">
            <span class="m-label">Hours</span>
            <span class="m-val">{formatDuration(curHalfStats.totalSecs)}</span>
            <span class="m-diff" class:pos={isPos(curHalfStats.totalSecs, prevHalfStats.totalSecs)} class:neg={curHalfStats.totalSecs < prevHalfStats.totalSecs}>
              {pctChange(curHalfStats.totalSecs, prevHalfStats.totalSecs)} {getTrend(curHalfStats.totalSecs, prevHalfStats.totalSecs)}
            </span>
          </div>
          <div class="metric-row">
            <span class="m-label">Rounds</span>
            <span class="m-val">{formatRounds(curHalfStats.totalRounds)}</span>
            <span class="m-diff" class:pos={isPos(curHalfStats.totalRounds, prevHalfStats.totalRounds)} class:neg={curHalfStats.totalRounds < prevHalfStats.totalRounds}>
              {pctChange(curHalfStats.totalRounds, prevHalfStats.totalRounds)} {getTrend(curHalfStats.totalRounds, prevHalfStats.totalRounds)}
            </span>
          </div>
        </div>
      </div>

      <!-- Card 4: Best Half -->
      <div class="card">
        <div class="card-title">vs. Best Half ({bestHalfInfo().half})</div>
        <div class="card-subtitle">At Day {currentDayOfHalf + 1} ({Math.round(curHalfPct * 100)}% of Half)</div>
        <div class="card-metrics">
          <div class="metric-row">
            <span class="m-label">Hours</span>
            <span class="m-val">{formatDuration(curHalfStats.totalSecs)}</span>
            <span class="m-diff" class:pos={isPos(curHalfStats.totalSecs, bestHalfInfo().stats.totalSecs)} class:neg={curHalfStats.totalSecs < bestHalfInfo().stats.totalSecs}>
              {pctChange(curHalfStats.totalSecs, bestHalfInfo().stats.totalSecs)} {getTrend(curHalfStats.totalSecs, bestHalfInfo().stats.totalSecs)}
            </span>
          </div>
          <div class="metric-row">
            <span class="m-label">Rounds</span>
            <span class="m-val">{formatRounds(curHalfStats.totalRounds)}</span>
            <span class="m-diff" class:pos={isPos(curHalfStats.totalRounds, bestHalfInfo().stats.totalRounds)} class:neg={curHalfStats.totalRounds < bestHalfInfo().stats.totalRounds}>
              {pctChange(curHalfStats.totalRounds, bestHalfInfo().stats.totalRounds)} {getTrend(curHalfStats.totalRounds, bestHalfInfo().stats.totalRounds)}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- The "Race" Chart -->
    <div class="chart-section">
      <h3>Cumulative Trajectory (Yearly Race)</h3>
      <div class="chart-container">
        <svg width="100%" height="100%" viewBox="0 0 {CHART_W} {CHART_H + 20}" preserveAspectRatio="none">
          <!-- Grid lines -->
          <line x1="0" y1="0" x2={CHART_W} y2="0" stroke="var(--color-background-light)" stroke-width="1" />
          <line x1="0" y1={CHART_H/2} x2={CHART_W} y2={CHART_H/2} stroke="var(--color-background-light)" stroke-width="1" />
          <line x1="0" y1={CHART_H} x2={CHART_W} y2={CHART_H} stroke="var(--color-background-light)" stroke-width="1" />

          <!-- Paths -->
          {#if chartData().best.length > 0}
            <path d={makePath(chartData().best)} fill="none" stroke="var(--color-warning, #FFA000)" stroke-width="2" stroke-dasharray="4 4" />
          {/if}
          {#if chartData().previous.length > 0}
            <path d={makePath(chartData().previous)} fill="none" stroke="var(--color-foreground-darker)" stroke-width="2" stroke-dasharray="2 2" />
          {/if}
          {#if chartData().current.length > 0}
            <path d={makePath(chartData().current)} fill="none" stroke="var(--color-focus-round)" stroke-width="3" />
          {/if}
        </svg>
      </div>
      <div class="chart-legend">
        <span class="legend-item"><span class="legend-color cur"></span> Current Year</span>
        <span class="legend-item"><span class="legend-color prev"></span> Previous Year ({currentYear - 1})</span>
        <span class="legend-item"><span class="legend-color best"></span> Best Year ({bestYearInfo().year})</span>
      </div>
    </div>

    <!-- Shoutouts -->
    <div class="shoutouts">
      {#if bestDay() && bestMonth()}
        <div class="shoutout-item">
          <strong>All-time Best Day:</strong> <span class="highlight">{new Date(`${bestDay()!.date}T12:00:00Z`).toLocaleDateString()}</span>
          <span class="metric">({formatDuration(bestDay()!.focus_secs)})</span>
        </div>
        <div class="shoutout-item">
          <strong>All-time Best Month:</strong> <span class="highlight">{bestMonth()!.month}</span>
          <span class="metric">({formatDuration(bestMonth()!.secs)})</span>
        </div>
      {/if}
    </div>

  {:else}
    <div class="loading">Loading comparisons...</div>
  {/if}
</div>

<style>
  .comparisons-dashboard {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 24px;
    overflow-y: auto;
  }

  .top-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--color-background-light);
    border-radius: 8px;
    padding: 20px;
  }

  .summary-details h3 {
    margin: 0 0 10px 0;
    font-size: 1.1rem;
    color: var(--color-foreground);
  }

  .summary-stats {
    font-size: 1rem;
    color: var(--color-foreground);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  
  .summary-stats strong {
    color: var(--color-focus-round);
  }

  .sep {
    color: var(--color-foreground-darker);
  }

  .forecast-box {
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--color-background);
    border: 1px solid var(--color-focus-round);
    padding: 12px 16px;
    border-radius: 6px;
    max-width: 320px;
  }

  .forecast-icon {
    font-size: 1.5rem;
  }

  .forecast-text {
    font-size: 0.85rem;
    color: var(--color-foreground);
    line-height: 1.4;
  }

  .forecast-text strong {
    color: var(--color-focus-round);
  }

  .cards-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
  }

  .card {
    background: var(--color-background-light);
    border-radius: 8px;
    padding: 16px;
  }

  .card-title {
    font-weight: 600;
    font-size: 1rem;
    color: var(--color-foreground);
  }

  .card-subtitle {
    font-size: 0.8rem;
    color: var(--color-foreground-darker);
    margin-top: 4px;
    margin-bottom: 16px;
  }

  .metric-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 0;
    border-bottom: 1px solid var(--color-background);
  }

  .metric-row:last-child {
    border-bottom: none;
  }

  .m-label {
    font-size: 0.9rem;
    color: var(--color-foreground-darker);
    flex: 1;
  }

  .m-val {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--color-foreground);
    margin-right: 12px;
  }

  .m-diff {
    font-size: 0.85rem;
    font-weight: 600;
    width: 60px;
    text-align: right;
  }

  .m-diff.pos {
    color: var(--color-focus-round);
  }

  .m-diff.neg {
    color: var(--color-focus-short);
  }

  .chart-section {
    background: var(--color-background-light);
    border-radius: 8px;
    padding: 20px;
  }

  .chart-section h3 {
    margin: 0 0 20px 0;
    font-size: 1rem;
    color: var(--color-foreground);
  }

  .chart-container {
    height: 150px;
    width: 100%;
    position: relative;
    padding-top: 10px;
  }

  .chart-legend {
    display: flex;
    justify-content: center;
    gap: 20px;
    margin-top: 16px;
    font-size: 0.85rem;
    color: var(--color-foreground-darker);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .legend-color {
    width: 16px;
    height: 3px;
    border-radius: 2px;
  }

  .legend-color.cur { background: var(--color-focus-round); height: 4px; }
  .legend-color.prev { border-bottom: 2px dashed var(--color-foreground-darker); }
  .legend-color.best { border-bottom: 2px dashed var(--color-warning, #FFA000); }

  .shoutouts {
    display: flex;
    gap: 24px;
    background: var(--color-background-light);
    border-radius: 8px;
    padding: 16px;
  }

  .shoutout-item {
    font-size: 0.95rem;
    color: var(--color-foreground);
  }

  .shoutout-item .highlight {
    color: var(--color-focus-round);
    font-weight: 600;
  }
  
  .shoutout-item .metric {
    color: var(--color-foreground-darker);
    font-size: 0.85rem;
  }

  .loading {
    text-align: center;
    padding: 40px;
    color: var(--color-foreground-darker);
  }
</style>
