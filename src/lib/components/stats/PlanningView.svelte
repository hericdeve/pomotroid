<script lang="ts">
  import { onMount } from 'svelte';
  import { subjectsGetAll, scheduleGetAll, scheduleAddBlock, scheduleDeleteBlock, scheduleUpdateBlock } from '$lib/ipc';
  import type { SubjectStats, ScheduledBlock } from '$lib/types';
  import { error as logError } from '@tauri-apps/plugin-log';
  import { settings } from '$lib/stores/settings';
  import WeeklyCalendar from './WeeklyCalendar.svelte';

  let subjects = $state<SubjectStats[]>([]);
  let blocks = $state<ScheduledBlock[]>([]);
  let loading = $state(true);

  onMount(async () => {
    try {
      subjects = await subjectsGetAll();
      blocks = await scheduleGetAll();
    } catch (e) {
      logError(`Failed to load planning data: ${e}`);
    } finally {
      loading = false;
    }
  });

  async function handleBlockAdd(day: number, startMin: number, endMin: number, subject: string) {
    try {
      const id = await scheduleAddBlock(subject, day, startMin, endMin);
      blocks = [...blocks, { id, subject, day_of_week: day, start_minute: startMin, end_minute: endMin }];
    } catch (e) {
      logError(`Failed to add block: ${e}`);
      alert(`Failed to add block: ${e}`);
    }
  }

  async function handleBlockDelete(id: number) {
    try {
      await scheduleDeleteBlock(id);
      blocks = blocks.filter(b => b.id !== id);
    } catch (e) {
      logError(`Failed to delete block: ${e}`);
      alert(`Failed to delete block: ${e}`);
    }
  }

  async function handleBlockUpdate(id: number, day: number, startMin: number, endMin: number) {
    try {
      await scheduleUpdateBlock(id, day, startMin, endMin);
      blocks = blocks.map(b => b.id === id ? { ...b, day_of_week: day, start_minute: startMin, end_minute: endMin } : b);
    } catch (e) {
      logError(`Failed to update block: ${e}`);
      alert(`Failed to update block: ${e}`);
    }
  }

  function calculateAllocatedRounds(subjectName: string): number {
    const subjectBlocks = blocks.filter(b => b.subject === subjectName);
    const totalMinutes = subjectBlocks.reduce((sum, b) => sum + (b.end_minute - b.start_minute), 0);
    
    // Calculate how many minutes one round typically takes
    const workMins = $settings.time_work_secs / 60;
    const shortBreakMins = $settings.short_breaks_enabled ? ($settings.time_short_break_secs / 60) : 0;
    const longBreakMins = $settings.long_breaks_enabled ? ($settings.time_long_break_secs / 60) : 0;
    const interval = $settings.long_break_interval;
    
    // Average minutes per round over a full interval cycle
    let cycleMins = 0;
    if (interval > 0) {
       cycleMins = (workMins * interval) + (shortBreakMins * (interval - 1)) + longBreakMins;
    } else {
       cycleMins = workMins + shortBreakMins;
    }
    const avgRoundMins = interval > 0 ? cycleMins / interval : cycleMins;
    
    if (avgRoundMins === 0) return 0;
    return Math.round(totalMinutes / avgRoundMins);
  }

  function handleDragStart(e: DragEvent, subject: SubjectStats) {
    if (e.dataTransfer) {
      e.dataTransfer.setData('application/json', JSON.stringify({ type: 'subject', data: subject.name }));
      e.dataTransfer.effectAllowed = 'copy';
    }
  }
</script>

<div class="planning-view">
  <!-- Left Sidebar: Subjects -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <h3>Subjects</h3>
    </div>
    
    <div class="subjects-list">
      {#if loading}
        <div class="empty">Loading...</div>
      {:else if subjects.length === 0}
        <div class="empty">No subjects available.</div>
      {:else}
        {#each subjects as subject (subject.id)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div 
            class="subject-item"
            draggable="true"
            ondragstart={(e) => handleDragStart(e, subject)}
          >
            <div class="subject-header">
              <span class="subject-name">{subject.name}</span>
              <span class="drag-handle">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="9" cy="5" r="1"/>
                  <circle cx="9" cy="12" r="1"/>
                  <circle cx="9" cy="19" r="1"/>
                  <circle cx="15" cy="5" r="1"/>
                  <circle cx="15" cy="12" r="1"/>
                  <circle cx="15" cy="19" r="1"/>
                </svg>
              </span>
            </div>
            
            {#if subject.weekly_goal}
              <!-- Real Allocation Progress -->
              {@const mockAllocated = calculateAllocatedRounds(subject.name)} 
              {@const goal = subject.weekly_goal}
              {@const overAllocated = mockAllocated > goal}
              <div class="allocation">
                <div class="allocation-label">
                  <span class="allocated-text" class:over={overAllocated}>
                    {mockAllocated} / {goal} <span class="unit">rounds allocated</span>
                  </span>
                </div>
                <div class="progress-track" class:over={overAllocated}>
                  <div 
                    class="progress-fill" 
                    class:over={overAllocated}
                    style="width: {Math.min(100, (mockAllocated / goal) * 100)}%"
                  ></div>
                </div>
              </div>
            {:else}
              <div class="no-goal">No weekly goal set</div>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </aside>

  <!-- Right Main Area: Weekly Calendar -->
  <main class="calendar-area">
    <WeeklyCalendar 
      {blocks}
      onBlockAdd={handleBlockAdd}
      onBlockDelete={handleBlockDelete}
      onBlockUpdate={handleBlockUpdate}
    />
  </main>
</div>

<style>
  .planning-view {
    display: flex;
    height: 100%;
    gap: 1rem;
    padding: 1rem;
    color: var(--color-text);
  }

  /* ── Sidebar ───────────────────────────────────── */
  .sidebar {
    width: 260px;
    display: flex;
    flex-direction: column;
    background: var(--color-background);
    border: 1px solid var(--color-subtext);
    border-radius: 6px;
    overflow: hidden;
  }

  .sidebar-header {
    padding: 1rem;
    border-bottom: 1px solid var(--color-subtext);
    background: var(--color-background-light, rgba(255,255,255,0.02));
  }

  .sidebar-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .subjects-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .subject-item {
    padding: 0.75rem;
    border-radius: 4px;
    background: rgba(255,255,255,0.03);
    border: 1px solid transparent;
    cursor: grab;
    transition: all 0.2s;
  }

  .subject-item:hover {
    background: rgba(255,255,255,0.06);
    border-color: var(--color-subtext, rgba(255,255,255,0.1));
  }

  .subject-item:active {
    cursor: grabbing;
  }

  .subject-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  .subject-name {
    font-weight: 500;
    font-size: 0.95rem;
  }

  .drag-handle {
    opacity: 0.3;
    display: flex;
    align-items: center;
  }

  .subject-item:hover .drag-handle {
    opacity: 0.8;
  }

  /* ── Progress Allocation ──────────────────────── */
  .allocation {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .allocation-label {
    font-size: 0.75rem;
    color: var(--color-subtext);
  }

  .allocated-text {
    font-weight: 600;
  }

  .allocated-text.over {
    color: #ef4444; /* red for overallocated */
  }

  .unit {
    font-weight: 400;
    opacity: 0.7;
  }

  .progress-track {
    height: 4px;
    background: var(--color-foreground-darker, rgba(255,255,255,0.1));
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-track.over {
    background: rgba(239, 68, 68, 0.2);
  }

  .progress-fill {
    height: 100%;
    background: var(--color-focus-round);
    border-radius: 2px;
    transition: width 0.3s;
  }

  .progress-fill.over {
    background: #ef4444;
  }

  .no-goal {
    font-size: 0.75rem;
    color: var(--color-subtext);
    font-style: italic;
    opacity: 0.7;
  }

  .empty {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--color-subtext);
    font-size: 0.85rem;
  }

  /* ── Main Area ─────────────────────────────────── */
  .calendar-area {
    flex: 1;
    background: var(--color-background);
    border-radius: 6px;
    display: flex;
    overflow: hidden;
  }
</style>
