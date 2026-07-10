<script lang="ts">
  import { settings } from '$lib/stores/settings';
  import type { ScheduledBlock } from '$lib/types';

  interface Props {
    block: ScheduledBlock;
    onClose: () => void;
    onSave: (updatedBlock: ScheduledBlock) => void;
  }

  let { block, onClose, onSave }: Props = $props();

  function formatTime(minutes: number): string {
    const h = Math.floor(minutes / 60) % 24;
    const m = minutes % 60;
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}`;
  }

  interface TimelineEvent {
    startStr: string;
    endStr: string;
    label: string;
    isBreak: boolean;
    roundNum?: number;
  }

  let subjectTopic = $state(block.subject_topic || '');
  let studyType = $state(block.study_type || '');
  let roundTagsStr = $state(block.round_tags || '{}');
  
  let roundTags = $derived.by(() => {
    try {
      return JSON.parse(roundTagsStr);
    } catch (e) {
      return {};
    }
  });

  function updateRoundTag(roundNum: number, field: 'topic' | 'studyType', value: string) {
    let current = { ...roundTags };
    if (!current[roundNum]) current[roundNum] = {};
    current[roundNum][field] = value;
    roundTagsStr = JSON.stringify(current);
  }

  function handleSave() {
    onSave({
      ...block,
      subject_topic: subjectTopic || null,
      study_type: studyType || null,
      round_tags: roundTagsStr === '{}' ? null : roundTagsStr
    });
  }

  let events = $derived.by(() => {
    let result: TimelineEvent[] = [];
    let currentMin = block.start_minute;
    let remainingMins = block.end_minute - block.start_minute;
    let cyclePosition = 1;
    let roundNum = 1;
    
    const workMins = Math.round($settings.time_work_secs / 60);
    const shortBreakMins = $settings.short_breaks_enabled ? Math.round($settings.time_short_break_secs / 60) : 0;
    const longBreakMins = $settings.long_breaks_enabled ? Math.round($settings.time_long_break_secs / 60) : 0;
    const interval = $settings.long_break_interval;

    while (remainingMins > 0) {
      // Work round
      const actualWorkMins = Math.min(workMins, remainingMins);
      const isPartialWork = actualWorkMins < workMins;
      
      if (isPartialWork) {
        break;
      }
      
      result.push({
        startStr: formatTime(currentMin),
        endStr: formatTime(currentMin + actualWorkMins),
        label: `Work (Round ${roundNum})`,
        isBreak: false,
        roundNum
      });
      
      currentMin += actualWorkMins;
      remainingMins -= actualWorkMins;
      roundNum++;
      
      if (remainingMins <= 0) break;
      
      let isLongBreak = (interval > 0 && cyclePosition % interval === 0);
      let breakMins = isLongBreak ? longBreakMins : shortBreakMins;
      
      if (breakMins > 0) {
        const actualBreakMins = Math.min(breakMins, remainingMins);
        const isPartialBreak = actualBreakMins < breakMins;
        
        if (isPartialBreak) {
          break;
        }
        
        if (remainingMins - actualBreakMins < workMins) {
          break;
        }
        
        const breakType = isLongBreak ? 'Long' : 'Short';
        
        result.push({
          startStr: formatTime(currentMin),
          endStr: formatTime(currentMin + actualBreakMins),
          label: `${breakType} Break`,
          isBreak: true
        });
        
        currentMin += actualBreakMins;
        remainingMins -= actualBreakMins;
      }
      
      if (isLongBreak) {
        cyclePosition = 1;
      } else {
        cyclePosition++;
      }
    }
    
    return result;
  });
</script>

<div class="modal-overlay" role="presentation" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
    <!-- Header -->
    <div class="header">
      <h2>{block.subject} ({formatTime(block.start_minute)} - {formatTime(block.end_minute)})</h2>
      <button class="close-btn" onclick={onClose} aria-label="Close">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="content">
      <div class="session-tags">
        <div class="input-group">
          <label for="session-topic">Session Topic</label>
          <input id="session-topic" type="text" bind:value={subjectTopic} placeholder="Topic for the whole session" />
        </div>
        <div class="input-group">
          <label for="session-study-type">Study Type</label>
          <input id="session-study-type" type="text" bind:value={studyType} placeholder="e.g. Reading, Coding" />
        </div>
      </div>
      <div class="timeline">
        {#each events as ev}
          <div class="timeline-row" class:is-break={ev.isBreak}>
            <div class="timeline-header">
              <span class="timeline-time">{ev.startStr} - {ev.endStr}</span>
              <span class="timeline-label">{ev.label}</span>
            </div>
            {#if !ev.isBreak && ev.roundNum}
              <div class="timeline-inputs">
                <input 
                  type="text" 
                  placeholder="Topic (opt)" 
                  value={roundTags[ev.roundNum!]?.topic || ''} 
                  oninput={(e) => updateRoundTag(ev.roundNum!, 'topic', e.currentTarget.value)}
                />
                <input 
                  type="text" 
                  placeholder="Type (opt)" 
                  value={roundTags[ev.roundNum!]?.studyType || ''} 
                  oninput={(e) => updateRoundTag(ev.roundNum!, 'studyType', e.currentTarget.value)}
                />
              </div>
            {/if}
          </div>
        {/each}
        {#if events.length === 0}
          <div class="empty-state">Not enough time for a complete round.</div>
        {/if}
      </div>
    </div>
    
    <div class="footer">
      <button class="save-btn" onclick={handleSave}>Save</button>
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
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
    flex-shrink: 0;
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
    max-height: 400px;
    overflow-y: auto;
  }

  .timeline {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .timeline-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--color-background-light, rgba(255,255,255,0.03));
    border-radius: 6px;
    border: 1px solid transparent;
  }

  .timeline-row:not(.is-break) {
    border-left: 3px solid var(--color-focus-round);
  }

  .timeline-row.is-break {
    opacity: 0.8;
  }

  .timeline-time {
    font-size: 0.85rem;
    color: var(--color-foreground-darker, var(--color-foreground));
  }

  .timeline-label {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--color-foreground);
  }

  .empty-state {
    text-align: center;
    font-size: 0.85rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    padding: 20px 0;
  }

  .session-tags {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--color-background-light, rgba(255,255,255,0.08));
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .input-group label {
    font-size: 0.8rem;
    color: var(--color-foreground-darker, var(--color-foreground));
  }

  input {
    background: var(--color-background-light, rgba(255,255,255,0.03));
    border: 1px solid var(--color-background-light, rgba(255,255,255,0.1));
    color: var(--color-foreground);
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.9rem;
    outline: none;
    width: 100%;
    box-sizing: border-box;
  }

  input:focus {
    border-color: var(--color-focus-round);
  }

  .timeline-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
  }

  .timeline-inputs {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .timeline-inputs input {
    padding: 4px 8px;
    font-size: 0.8rem;
  }

  .timeline-row {
    display: flex;
    flex-direction: column;
    padding: 8px 12px;
    background: var(--color-background-light, rgba(255,255,255,0.03));
    border-radius: 6px;
    border: 1px solid transparent;
  }

  .timeline-row:not(.is-break) {
    border-left: 3px solid var(--color-focus-round);
  }

  .footer {
    padding: 16px 20px;
    border-top: 1px solid var(--color-background-light, rgba(255,255,255,0.08));
    display: flex;
    justify-content: flex-end;
  }

  .save-btn {
    background: var(--color-focus-round);
    color: var(--color-background);
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .save-btn:hover {
    opacity: 0.9;
  }

  @keyframes fade-in {
    from { opacity: 0; transform: scale(0.97); }
    to   { opacity: 1; transform: scale(1); }
  }
</style>
