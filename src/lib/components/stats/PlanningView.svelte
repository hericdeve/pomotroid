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
      const data = await subjectsGetAll();
      subjects = data.sort((a, b) => {
        const aHasGoal = a.weekly_goal != null;
        const bHasGoal = b.weekly_goal != null;
        if (aHasGoal && !bHasGoal) return -1;
        if (!aHasGoal && bHasGoal) return 1;
        return a.name.localeCompare(b.name);
      });
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
    const prevBlocks = blocks;
    blocks = blocks.filter(b => b.id !== id);
    try {
      await scheduleDeleteBlock(id);
    } catch (e) {
      blocks = prevBlocks;
      logError(`Failed to delete block: ${e}`);
      alert(`Failed to delete block: ${e}`);
    }
  }

  async function handleBlockUpdate(id: number, day: number, startMin: number, endMin: number) {
    const prevBlocks = blocks;
    blocks = blocks.map(b => b.id === id ? { ...b, day_of_week: day, start_minute: startMin, end_minute: endMin } : b);
    try {
      await scheduleUpdateBlock(id, day, startMin, endMin);
    } catch (e) {
      blocks = prevBlocks;
      logError(`Failed to update block: ${e}`);
      alert(`Failed to update block: ${e}`);
    }
  }

  function calculateAllocatedRounds(subjectName: string): number {
    const subjectBlocks = blocks.filter(b => b.subject === subjectName);
    if (subjectBlocks.length === 0) return 0;
    
    const workMins = $settings.time_work_secs / 60;
    const shortBreakMins = $settings.short_breaks_enabled ? ($settings.time_short_break_secs / 60) : 0;
    const longBreakMins = $settings.long_breaks_enabled ? ($settings.time_long_break_secs / 60) : 0;
    const interval = $settings.long_break_interval;
    
    let totalRounds = 0;

    for (const block of subjectBlocks) {
      let remainingMins = block.end_minute - block.start_minute;
      let roundsInBlock = 0;
      let cyclePosition = 1; // 1-based, up to interval

      while (remainingMins >= workMins) {
        // Complete a work round
        remainingMins -= workMins;
        roundsInBlock++;
        
        // If we are out of time, no need to process the break
        if (remainingMins <= 0) break;

        // Take the appropriate break
        if (interval > 0 && cyclePosition % interval === 0) {
          remainingMins -= longBreakMins;
          cyclePosition = 1; // Reset cycle after long break
        } else {
          remainingMins -= shortBreakMins;
          cyclePosition++;
        }
      }
      
      totalRounds += roundsInBlock;
    }
    
    return totalRounds;
  }

  function handleDragStart(e: DragEvent, subject: SubjectStats) {
    if (e.dataTransfer) {
      e.dataTransfer.setData('application/json', JSON.stringify({ type: 'subject', data: subject.name }));
      e.dataTransfer.effectAllowed = 'copy';

      // Create a more subtle, custom drag ghost
      const dragGhost = document.createElement('div');
      dragGhost.textContent = subject.name;
      // Basic styling matching a generic subtle badge
      dragGhost.style.backgroundColor = 'var(--color-focus-round)';
      dragGhost.style.color = 'var(--color-background)';
      dragGhost.style.padding = '4px 12px';
      dragGhost.style.borderRadius = '4px';
      dragGhost.style.fontSize = '12px';
      dragGhost.style.fontWeight = '600';
      dragGhost.style.boxShadow = '0 2px 8px rgba(0,0,0,0.2)';
      dragGhost.style.position = 'absolute';
      dragGhost.style.top = '-1000px';
      dragGhost.style.left = '-1000px';
      dragGhost.style.opacity = '0.9';
      
      document.body.appendChild(dragGhost);
      e.dataTransfer.setDragImage(dragGhost, 10, 10);

      // Clean up after the browser captures the snapshot (setTimeout(0) runs after the current event loop)
      setTimeout(() => {
        if (document.body.contains(dragGhost)) {
          document.body.removeChild(dragGhost);
        }
      }, 0);
    }
  }
</script>

<div class="planning-view">
  <!-- Left Sidebar: Subjects -->
  <aside class="sidebar">
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
    background: transparent;
    border-right: 1px solid var(--color-separator);
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
    border-radius: 6px;
    background: transparent;
    cursor: grab;
    transition: all 0.1s;
    position: relative;
  }

  .subject-item:hover {
    background: var(--color-hover);
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
    position: absolute;
    bottom: 0;
    left: 12px;
    right: 12px;
    height: 2px;
    background: transparent;
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
