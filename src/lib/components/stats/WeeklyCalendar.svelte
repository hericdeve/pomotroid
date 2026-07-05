<script lang="ts">
  import { onMount } from 'svelte';
  import type { ScheduledBlock } from '$lib/types';

  interface Props {
    blocks: ScheduledBlock[];
    onBlockAdd: (day: number, startMin: number, endMin: number, subject: string) => void;
    onBlockDelete: (id: number) => void;
  }

  let { blocks, onBlockAdd, onBlockDelete }: Props = $props();

  const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  const HOURS = Array.from({ length: 24 }, (_, i) => i);

  // Snap resolution in minutes
  const SNAP_MINUTES = 30;
  // Visual height representing 1 hour (e.g. 60px)
  const PIXELS_PER_HOUR = 60;
  const PIXELS_PER_MINUTE = PIXELS_PER_HOUR / 60;

  let dragOverCell: { day: number, hour: number } | null = $state(null);

  function handleDragOver(e: DragEvent, day: number, hour: number) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'copy';
    }
    dragOverCell = { day, hour };
  }

  function handleDragLeave() {
    dragOverCell = null;
  }

  function handleDrop(e: DragEvent, day: number, hour: number) {
    e.preventDefault();
    dragOverCell = null;
    
    if (e.dataTransfer) {
      const subject = e.dataTransfer.getData('text/plain');
      if (subject) {
        // Find Y offset within the target hour cell to snap to resolution
        let offsetMinutes = 0;
        const target = e.currentTarget as HTMLElement;
        const rect = target.getBoundingClientRect();
        const y = e.clientY - rect.top;
        
        // Calculate minutes based on Y offset
        const minutes = Math.floor(y / PIXELS_PER_MINUTE);
        // Snap to grid
        offsetMinutes = Math.floor(minutes / SNAP_MINUTES) * SNAP_MINUTES;

        const startMin = hour * 60 + offsetMinutes;
        const defaultDuration = 120; // 2 hours by default
        const endMin = Math.min(24 * 60, startMin + defaultDuration);
        
        onBlockAdd(day, startMin, endMin, subject);
      }
    }
  }

  function formatTime(minutes: number): string {
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}`;
  }
</script>

<div class="calendar-container">
  <div class="calendar-header">
    <div class="time-column-header"></div>
    {#each DAYS as day}
      <div class="day-header">{day}</div>
    {/each}
  </div>
  
  <div class="calendar-body">
    <!-- Background Grid -->
    <div class="grid-layer">
      <!-- Time Labels Column -->
      <div class="time-labels">
        {#each HOURS as hour}
          <div class="time-label" style="height: {PIXELS_PER_HOUR}px">
            {hour.toString().padStart(2, '0')}:00
          </div>
        {/each}
      </div>

      <!-- Days Columns -->
      <div class="days-columns">
        {#each DAYS as dayName, dayIdx}
          <div class="day-column">
            {#each HOURS as hour}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div 
                class="hour-cell"
                class:drag-over={dragOverCell?.day === dayIdx && dragOverCell?.hour === hour}
                style="height: {PIXELS_PER_HOUR}px"
                ondragover={(e) => handleDragOver(e, dayIdx, hour)}
                ondragleave={handleDragLeave}
                ondrop={(e) => handleDrop(e, dayIdx, hour)}
              ></div>
            {/each}

            <!-- Blocks Overlay for this day -->
            {#each blocks.filter(b => b.day_of_week === dayIdx) as block (block.id)}
              {@const top = block.start_minute * PIXELS_PER_MINUTE}
              {@const height = (block.end_minute - block.start_minute) * PIXELS_PER_MINUTE}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div 
                class="scheduled-block"
                style="top: {top}px; height: {height}px;"
                title="{block.subject} ({formatTime(block.start_minute)} - {formatTime(block.end_minute)})"
              >
                <div class="block-content">
                  <span class="block-subject">{block.subject}</span>
                  <span class="block-time">{formatTime(block.start_minute)}</span>
                </div>
                <button 
                  class="btn-delete-block" 
                  onclick={() => onBlockDelete(block.id)}
                  title="Remove block"
                >
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                  </svg>
                </button>
              </div>
            {/each}
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .calendar-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background: var(--color-background);
    color: var(--color-text);
    overflow: hidden;
    font-size: 0.85rem;
  }

  .calendar-header {
    display: flex;
    border-bottom: 1px solid var(--color-subtext);
    background: var(--color-background-light, rgba(255,255,255,0.02));
    padding-right: 12px; /* scrollbar offset compensation */
  }

  .time-column-header {
    width: 60px;
    flex-shrink: 0;
    border-right: 1px solid var(--color-subtext);
  }

  .day-header {
    flex: 1;
    text-align: center;
    padding: 0.75rem 0;
    font-weight: 600;
    border-right: 1px solid var(--color-subtext);
  }
  .day-header:last-child {
    border-right: none;
  }

  .calendar-body {
    flex: 1;
    overflow-y: auto;
    position: relative;
  }

  .grid-layer {
    display: flex;
    position: relative;
  }

  .time-labels {
    width: 60px;
    flex-shrink: 0;
    border-right: 1px solid var(--color-subtext);
    background: var(--color-background-light, rgba(255,255,255,0.01));
  }

  .time-label {
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 4px;
    color: var(--color-subtext);
    font-size: 0.75rem;
    border-bottom: 1px solid transparent; /* matches right grid */
  }

  .days-columns {
    display: flex;
    flex: 1;
  }

  .day-column {
    flex: 1;
    border-right: 1px solid var(--color-subtext);
    position: relative;
  }
  .day-column:last-child {
    border-right: none;
  }

  .hour-cell {
    border-bottom: 1px solid var(--color-subtext);
    transition: background-color 0.1s;
  }

  .hour-cell.drag-over {
    background: rgba(255, 255, 255, 0.1);
  }

  .scheduled-block {
    position: absolute;
    left: 2px;
    right: 2px;
    background: var(--color-focus-round);
    color: var(--color-background);
    border-radius: 4px;
    padding: 4px 6px;
    overflow: hidden;
    box-shadow: 0 2px 4px rgba(0,0,0,0.2);
    display: flex;
    flex-direction: column;
    z-index: 10;
    transition: opacity 0.2s;
  }

  .scheduled-block:hover {
    z-index: 11;
  }

  .block-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    pointer-events: none; /* Let clicks pass to the block itself if needed */
  }

  .block-subject {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .block-time {
    font-size: 0.7rem;
    opacity: 0.9;
  }

  .btn-delete-block {
    position: absolute;
    top: 4px;
    right: 4px;
    background: rgba(0,0,0,0.2);
    color: var(--color-background);
    border: none;
    border-radius: 4px;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.2s, background 0.2s;
  }

  .scheduled-block:hover .btn-delete-block {
    opacity: 1;
  }

  .btn-delete-block:hover {
    background: rgba(0,0,0,0.5);
  }
</style>
