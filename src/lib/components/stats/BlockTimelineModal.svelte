<script lang="ts">
  import { settings } from '$lib/stores/settings';
  import type { ScheduledBlock } from '$lib/types';

  interface Props {
    block: ScheduledBlock;
    onClose: () => void;
    onSave: (updatedBlock: ScheduledBlock, scope?: 'instance' | 'series') => void;
  }

  let { block, onClose, onSave }: Props = $props();

  function formatTime(minutes: number): string {
    const h = Math.floor(minutes / 60) % 24;
    const m = minutes % 60;
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}`;
  }

  function formatMinsDuration(totalMinutes: number): string {
    if (totalMinutes <= 0) return '0m';
    const h = Math.floor(totalMinutes / 60);
    const m = totalMinutes % 60;
    if (h > 0 && m > 0) return `${h}h${m.toString().padStart(2, '0')}`;
    if (h > 0) return `${h}h`;
    return `${m}m`;
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
  
  let isRecurringMaster = $derived(
    block.calendar_type === 'google' &&
    !block.is_exception &&
    Boolean(block.recurring_event_id || block.google_event_id)
  );

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

  function handleSave(scope: 'instance' | 'series' = 'instance') {
    onSave({
      ...block,
      subject_topic: subjectTopic || null,
      study_type: studyType || null,
      round_tags: roundTagsStr === '{}' ? null : roundTagsStr
    }, scope);
  }

  let events = $derived.by(() => {
    let result: TimelineEvent[] = [];
    let currentMin = block.start_minute;
    const effectiveEnd = block.end_minute <= block.start_minute ? block.end_minute + 1440 : block.end_minute;
    let remainingMins = effectiveEnd - block.start_minute;
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

  let workRounds = $derived(events.filter(e => !e.isBreak).length);
  let studyMins = $derived(Math.round(workRounds * ($settings.time_work_secs / 60)));
  let totalBlockMins = $derived(
    (block.end_minute <= block.start_minute ? block.end_minute + 1440 : block.end_minute) - block.start_minute
  );
</script>

<div class="modal-overlay" role="presentation" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
    <!-- Header -->
    <div class="header">
      <div class="header-titles">
        <h2>{block.subject}</h2>
        <div class="header-meta">
          <span class="meta-time">{formatTime(block.start_minute)} – {formatTime(block.end_minute)}</span>
          <span class="meta-badge rounds">
            <span class="rounds-dot"></span>
            {workRounds}
          </span>
          <span class="meta-badge study">{formatMinsDuration(studyMins)}</span>
          {#if totalBlockMins > studyMins}
            <span class="meta-badge session">~{formatMinsDuration(totalBlockMins)}</span>
          {/if}
          {#if block.is_exception}
            <span class="meta-badge exception" title="Modified for this week only">This week only</span>
          {:else if block.calendar_type === 'google'}
            <span class="meta-badge recurring" title="Weekly recurring session">Weekly recurring</span>
          {/if}
        </div>
      </div>
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
      {#if isRecurringMaster}
        <button class="save-btn secondary" onclick={() => handleSave('instance')}>This session only</button>
        <button class="save-btn" onclick={() => handleSave('series')}>All weekly sessions</button>
      {:else}
        <button class="save-btn" onclick={() => handleSave('instance')}>Save</button>
      {/if}
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

  .header-titles {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
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

  .header-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }

  .meta-time {
    font-size: 0.74rem;
    font-weight: 600;
    color: var(--color-foreground);
    opacity: 0.85;
  }

  .meta-badge {
    display: inline-flex;
    align-items: center;
    gap: 3.5px;
    font-size: 0.67rem;
    font-weight: 500;
    color: var(--color-foreground);
    background: var(--color-background-light, rgba(255, 255, 255, 0.08));
    padding: 1.5px 6.5px;
    border-radius: 999px;
    opacity: 0.9;
  }

  .meta-badge.study {
    background: color-mix(in srgb, var(--color-focus-round, #4a90e2) 18%, transparent);
    color: var(--color-foreground);
  }

  .meta-badge.exception {
    background: color-mix(in srgb, #f59e0b 20%, transparent);
    color: #fbbf24;
    border: 1px solid color-mix(in srgb, #f59e0b 35%, transparent);
  }

  .meta-badge.recurring {
    background: color-mix(in srgb, #9333ea 20%, transparent);
    color: #c084fc;
    border: 1px solid color-mix(in srgb, #9333ea 35%, transparent);
  }

  .rounds-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--color-focus-round, currentColor);
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

  .save-btn.secondary {
    background: var(--color-background-light, rgba(255, 255, 255, 0.08));
    color: var(--color-foreground);
    border: 1px solid var(--color-background-light, rgba(255, 255, 255, 0.15));
    margin-right: 8px;
  }

  .save-btn.secondary:hover {
    background: var(--color-hover, rgba(255, 255, 255, 0.15));
  }

  @keyframes fade-in {
    from { opacity: 0; transform: scale(0.97); }
    to   { opacity: 1; transform: scale(1); }
  }
</style>
