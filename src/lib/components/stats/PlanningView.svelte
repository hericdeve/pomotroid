<script lang="ts">
  import { onMount } from 'svelte';
  import {
    subjectsGetAll,
    scheduleGetAll,
    scheduleAddBlock,
    scheduleDeleteBlock,
    scheduleUpdateBlock,
    googleCalendarGetStatus,
    googleCalendarGetCalendars,
    googleCalendarSyncNow,
    googleCalendarGetOverlayEvents,
    calendarGetLocalVisible,
  } from '$lib/ipc';
  import type { SubjectStats, ScheduledBlock, GoogleAuthStatus, GoogleCalendarItem, GoogleOverlayEvent } from '$lib/types';
  import { error as logError } from '@tauri-apps/plugin-log';
  import { settings } from '$lib/stores/settings';
  import WeeklyCalendar from './WeeklyCalendar.svelte';
  import { openSettingsWindow } from '$lib/utils/windows';

  let subjects = $state<SubjectStats[]>([]);
  let blocks = $state<ScheduledBlock[]>([]);
  let loading = $state(true);

  // Calendar states (configured in Settings)
  let showLocalCalendar = $state(true);
  let authStatus = $state<GoogleAuthStatus>({
    is_signed_in: false,
    email: null,
    client_id: null,
    has_client_secret: false,
  });
  let calendars = $state<GoogleCalendarItem[]>([]);
  let overlayEvents = $state<GoogleOverlayEvent[]>([]);
  let syncedCalendar = $derived(calendars.find(c => c.is_synced) || null);
  let showSyncedCalendar = $derived(
    syncedCalendar ? syncedCalendar.is_visible : calendars.some(c => c.is_visible)
  );

  let weekOffset = $state(0);
  let isSyncing = $state(false);

  function getMondayYmd(offsetWeeks: number = 0): string {
    const d = new Date();
    const day = d.getDay();
    const diffToMonday = (day + 6) % 7;
    d.setDate(d.getDate() - diffToMonday + (offsetWeeks * 7));
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const dayNum = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${dayNum}`;
  }

  let currentMondayYmd = $derived(getMondayYmd(weekOffset));

  onMount(() => {
    let mounted = true;

    const handleFocus = async () => {
      try {
        const [localVis, gStatus] = await Promise.all([
          calendarGetLocalVisible().catch(() => true),
          googleCalendarGetStatus().catch(() => authStatus),
        ]);
        if (!mounted) return;
        showLocalCalendar = localVis;
        const wasSignedIn = authStatus.is_signed_in;
        authStatus = gStatus;
        if (gStatus.is_signed_in) {
          await loadGoogleData();
          if (syncedCalendar) {
            blocks = await googleCalendarSyncNow(currentMondayYmd);
          }
        } else if (wasSignedIn && !gStatus.is_signed_in) {
          calendars = [];
          overlayEvents = [];
          blocks = await scheduleGetAll();
        }
      } catch (e) {
        logError(`Failed to refresh on window focus: ${e}`);
      }
    };

    window.addEventListener('focus', handleFocus);

    (async () => {
      try {
        const [subjectsData, blocksData, localVis, gStatus] = await Promise.all([
          subjectsGetAll(),
          scheduleGetAll(),
          calendarGetLocalVisible().catch(() => true),
          googleCalendarGetStatus().catch(() => ({
            is_signed_in: false,
            email: null,
            client_id: null,
            has_client_secret: false,
          })),
        ]);
        if (!mounted) return;

        subjects = subjectsData.sort((a, b) => {
          const aHasGoal = a.weekly_goal != null;
          const bHasGoal = b.weekly_goal != null;
          if (aHasGoal && !bHasGoal) return -1;
          if (!aHasGoal && bHasGoal) return 1;
          return a.name.localeCompare(b.name);
        });
        blocks = blocksData;
        showLocalCalendar = localVis;
        authStatus = gStatus;

        if (gStatus.is_signed_in) {
          await loadGoogleData();
        }
      } catch (e) {
        logError(`Failed to load planning data: ${e}`);
      } finally {
        if (mounted) loading = false;
      }
    })();

    return () => {
      mounted = false;
      window.removeEventListener('focus', handleFocus);
    };
  });

  async function loadGoogleData() {
    try {
      calendars = await googleCalendarGetCalendars();
      overlayEvents = await googleCalendarGetOverlayEvents(currentMondayYmd);
    } catch (e: any) {
      logError(`Failed to load Google Calendar data: ${e?.message || e}`);
    }
  }

  async function handleSyncNow() {
    if (!authStatus.is_signed_in) return;
    isSyncing = true;
    try {
      blocks = await googleCalendarSyncNow(currentMondayYmd);
      overlayEvents = await googleCalendarGetOverlayEvents(currentMondayYmd);
    } catch (e) {
      logError(`Sync failed: ${e}`);
    } finally {
      isSyncing = false;
    }
  }

  async function handleWeekChange(newMondayYmd: string, offset: number) {
    weekOffset = offset;
    if (authStatus.is_signed_in) {
      try {
        overlayEvents = await googleCalendarGetOverlayEvents(newMondayYmd);
        if (syncedCalendar) {
          blocks = await googleCalendarSyncNow(newMondayYmd);
        }
      } catch (e) {
        logError(`Failed to update events for week: ${e}`);
      }
    }
  }

  async function handleBlockAdd(day: number, startMin: number, endMin: number, subject: string) {
    try {
      let calType: 'google' | 'local' = 'local';
      let targetGCal = syncedCalendar;

      if (!showLocalCalendar) {
        calType = 'google';
        if (!targetGCal) {
          targetGCal = calendars.find(c => c.is_visible) || calendars[0] || null;
        }
      } else if (targetGCal && targetGCal.is_visible) {
        calType = 'google';
      }

      const id = await scheduleAddBlock(
        subject,
        day,
        startMin,
        endMin,
        null,
        null,
        null,
        calType,
        currentMondayYmd
      );

      blocks = [...blocks, { 
        id, 
        subject, 
        day_of_week: day, 
        start_minute: startMin, 
        end_minute: endMin,
        subject_topic: null,
        study_type: null,
        round_tags: null,
        calendar_type: calType,
        google_calendar_id: calType === 'google' ? (targetGCal?.id || null) : null,
        google_event_id: null,
      }];
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
      await scheduleUpdateBlock(
        id,
        day,
        startMin,
        endMin,
        null,
        null,
        null,
        currentMondayYmd
      );
    } catch (e) {
      blocks = prevBlocks;
      logError(`Failed to update block: ${e}`);
      alert(`Failed to update block: ${e}`);
    }
  }

  function calculateAllocatedRounds(subjectName: string): number {
    const subjectBlocks = blocks.filter(b => {
      if (b.subject !== subjectName) return false;
      if (b.calendar_type === 'google') return true;
      return showLocalCalendar;
    });
    if (subjectBlocks.length === 0) return 0;
    
    const workMins = $settings.time_work_secs / 60;
    const shortBreakMins = $settings.short_breaks_enabled ? ($settings.time_short_break_secs / 60) : 0;
    const longBreakMins = $settings.long_breaks_enabled ? ($settings.time_long_break_secs / 60) : 0;
    const interval = $settings.long_break_interval;
    
    let totalRounds = 0;

    for (const block of subjectBlocks) {
      let remainingMins = block.end_minute - block.start_minute;
      let roundsInBlock = 0;
      let cyclePosition = 1;

      while (remainingMins >= workMins) {
        remainingMins -= workMins;
        roundsInBlock++;
        if (remainingMins <= 0) break;

        if (interval > 0 && cyclePosition % interval === 0) {
          remainingMins -= longBreakMins;
          cyclePosition = 1;
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

      const dragGhost = document.createElement('div');
      dragGhost.textContent = subject.name;
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
    <div class="sidebar-header">
      <span class="section-title">Subjects</span>
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
              <div class="subject-main">
                <span class="subject-name">{subject.name}</span>
                <span class="drag-handle" title="Drag onto calendar">
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
      {overlayEvents}
      {showLocalCalendar}
      {showSyncedCalendar}
      syncedCalendarSummary={syncedCalendar?.summary || null}
      {weekOffset}
      {isSyncing}
      onBlockAdd={handleBlockAdd}
      onBlockDelete={handleBlockDelete}
      onBlockUpdate={handleBlockUpdate}
      onWeekChange={handleWeekChange}
      onSyncClick={handleSyncNow}
      onSettingsClick={() => openSettingsWindow('calendar')}
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
    width: 270px;
    display: flex;
    flex-direction: column;
    background: transparent;
    border-right: 1px solid var(--color-separator);
    overflow: hidden;
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    border-bottom: 1px solid var(--color-separator);
  }

  .section-title {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 700;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  /* ── Subjects List ─────────────────────────────── */
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

  .subject-main {
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
    color: #ef4444;
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
