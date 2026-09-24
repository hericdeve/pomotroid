<script lang="ts">
  import { onMount } from 'svelte';
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
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
    subjectEventsGetAll,
  } from '$lib/ipc';
  import type { SubjectStats, ScheduledBlock, GoogleAuthStatus, GoogleCalendarItem, GoogleOverlayEvent, SubjectEvent } from '$lib/types';
  import { error as logError } from '@tauri-apps/plugin-log';
  import { settings } from '$lib/stores/settings';
  import WeeklyCalendar from './WeeklyCalendar.svelte';
  import { openSettingsWindow } from '$lib/utils/windows';

  let subjects = $state<SubjectStats[]>([]);
  let blocks = $state<ScheduledBlock[]>([]);
  let subjectEvents = $state<SubjectEvent[]>([]);
  let loading = $state(true);
  let showSidebar = $state(false);

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
  let showSyncedCalendar = $derived(syncedCalendar ? syncedCalendar.is_visible : false);

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

  function getDateForDay(mondayYmd: string, dayOfWeek: number): string {
    const [y, m, d] = mondayYmd.split('-').map(Number);
    const date = new Date(y, m - 1, d + dayOfWeek);
    const resY = date.getFullYear();
    const resM = String(date.getMonth() + 1).padStart(2, '0');
    const resD = String(date.getDate()).padStart(2, '0');
    return `${resY}-${resM}-${resD}`;
  }

  let currentMondayYmd = $derived(getMondayYmd(weekOffset));
  let currentSundayYmd = $derived(getDateForDay(currentMondayYmd, 6));

  onMount(() => {
    let mounted = true;

    const handleFocus = async () => {
      try {
        const [localVis, gStatus, evs] = await Promise.all([
          calendarGetLocalVisible().catch(() => showLocalCalendar),
          googleCalendarGetStatus().catch(() => authStatus),
          subjectEventsGetAll().catch(() => subjectEvents),
        ]);
        if (!mounted) return;
        showLocalCalendar = localVis;
        subjectEvents = evs;
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
        const [subjectsData, blocksData, localVis, gStatus, evs] = await Promise.all([
          subjectsGetAll(),
          scheduleGetAll(),
          calendarGetLocalVisible().catch(() => showLocalCalendar),
          googleCalendarGetStatus().catch(() => ({
            is_signed_in: false,
            email: null,
            client_id: null,
            has_client_secret: false,
          })),
          subjectEventsGetAll().catch(() => []),
        ]);
        if (!mounted) return;

        subjectEvents = evs;

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
          if (syncedCalendar) {
            try {
              blocks = await googleCalendarSyncNow(currentMondayYmd);
            } catch (e) {
              logError(`Initial sync failed: ${e}`);
            }
          }
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

      if (targetGCal && targetGCal.is_visible) {
        calType = 'google';
      } else if (showLocalCalendar) {
        calType = 'local';
      } else if (targetGCal) {
        calType = 'google';
      } else {
        alert('Please enable the Local calendar or select a synced Google calendar in Settings to add blocks.');
        return;
      }

      const newBlock = await scheduleAddBlock(
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

      blocks = [...blocks, newBlock];
      if (authStatus.is_signed_in && syncedCalendar) {
        blocks = await googleCalendarSyncNow(currentMondayYmd);
      }
    } catch (e) {
      logError(`Failed to add block: ${e}`);
      alert(`Failed to add block: ${e}`);
    }
  }

  async function handleBlockDelete(id: number, scope: 'instance' | 'series' = 'instance') {
    const prevBlocks = blocks;
    const targetBlock = blocks.find(b => b.id === id);
    if (scope === 'series' && targetBlock) {
      const targetSeriesId = targetBlock.recurring_event_id || targetBlock.google_event_id;
      blocks = blocks.filter(b => b.recurring_event_id !== targetSeriesId && b.google_event_id !== targetSeriesId && b.id !== id);
    } else if (targetBlock && !targetBlock.is_exception && targetBlock.calendar_type === 'google') {
      const targetSeriesId = targetBlock.recurring_event_id || targetBlock.google_event_id;
      const sessionDate = getDateForDay(currentMondayYmd, targetBlock.day_of_week);
      blocks = [
        ...blocks,
        {
          ...targetBlock,
          id: -Date.now(),
          subject_topic: '__cancelled__',
          session_date: sessionDate,
          recurring_event_id: targetSeriesId,
          is_exception: true,
        },
      ];
    } else {
      blocks = blocks.filter(b => b.id !== id);
    }
    try {
      await scheduleDeleteBlock(id, currentMondayYmd, scope);
      if (authStatus.is_signed_in && syncedCalendar) {
        blocks = await googleCalendarSyncNow(currentMondayYmd);
      }
    } catch (e) {
      blocks = prevBlocks;
      logError(`Failed to delete block: ${e}`);
      alert(`Failed to delete block: ${e}`);
    }
  }

  async function handleBlockUpdate(
    id: number,
    day: number,
    startMin: number,
    endMin: number,
    scope: 'instance' | 'series' = 'instance'
  ) {
    const prevBlocks = blocks;
    const targetBlock = blocks.find(b => b.id === id);
    if (!targetBlock) return;

    if (scope === 'series') {
      const targetSeriesId = targetBlock.recurring_event_id || targetBlock.google_event_id;
      blocks = blocks.map(b => {
        if ((b.recurring_event_id === targetSeriesId || b.google_event_id === targetSeriesId) && !b.is_exception) {
          return { ...b, day_of_week: day, start_minute: startMin, end_minute: endMin };
        }
        return b;
      });
    } else {
      if (targetBlock.is_exception) {
        const sessionDate = getDateForDay(currentMondayYmd, day);
        blocks = blocks.map(b =>
          b.id === id
            ? {
                ...b,
                day_of_week: day,
                start_minute: startMin,
                end_minute: endMin,
                session_date: sessionDate,
              }
            : b
        );
      } else {
        const targetSeriesId = targetBlock.recurring_event_id || targetBlock.google_event_id;
        const sessionDate = getDateForDay(currentMondayYmd, day);
        const newException: ScheduledBlock = {
          ...targetBlock,
          id: -Date.now(),
          day_of_week: day,
          start_minute: startMin,
          end_minute: endMin,
          session_date: sessionDate,
          recurring_event_id: targetSeriesId,
          is_exception: true,
        };
        blocks = [...blocks, newException];
      }
    }
    try {
      await scheduleUpdateBlock(
        id,
        day,
        startMin,
        endMin,
        targetBlock?.subject_topic ?? null,
        targetBlock?.study_type ?? null,
        targetBlock?.round_tags ?? null,
        currentMondayYmd,
        scope
      );
      if (authStatus.is_signed_in && syncedCalendar) {
        blocks = await googleCalendarSyncNow(currentMondayYmd);
      }
    } catch (e) {
      blocks = prevBlocks;
      logError(`Failed to update block: ${e}`);
      alert(`Failed to update block: ${e}`);
    }
  }

  function calculateAllocatedRounds(subjectName: string): number {
    const exceptionSeriesIds = new Set(
      blocks
        .filter(b => b.is_exception && b.session_date && b.session_date >= currentMondayYmd && b.session_date <= currentSundayYmd)
        .map(b => b.recurring_event_id || b.google_event_id)
        .filter(Boolean)
    );

    const subjectBlocks = blocks.filter(b => {
      if (b.subject !== subjectName) return false;
      if (b.subject_topic === '__cancelled__') return false;
      if (!b.session_date) {
        const seriesId = b.recurring_event_id || b.google_event_id;
        if (seriesId && exceptionSeriesIds.has(seriesId)) return false;
      }
      if (b.calendar_type === 'google') {
        if (!showSyncedCalendar) return false;
        if (syncedCalendar && b.google_calendar_id && b.google_calendar_id !== syncedCalendar.id) {
          return false;
        }
        if (b.session_date) {
          if (b.session_date < currentMondayYmd || b.session_date > currentSundayYmd) {
            return false;
          }
        }
        return true;
      }
      return showLocalCalendar;
    });
    if (subjectBlocks.length === 0) return 0;
    
    const workMins = $settings.time_work_secs / 60;
    const shortBreakMins = $settings.short_breaks_enabled ? ($settings.time_short_break_secs / 60) : 0;
    const longBreakMins = $settings.long_breaks_enabled ? ($settings.time_long_break_secs / 60) : 0;
    const interval = $settings.long_break_interval;
    
    let totalRounds = 0;

    for (const block of subjectBlocks) {
      const effectiveEnd = block.end_minute <= block.start_minute ? block.end_minute + 1440 : block.end_minute;
      let remainingMins = effectiveEnd - block.start_minute;
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

<div class="planning-view" class:sidebar-open={showSidebar}>
  <!-- Left Sidebar: Subjects (toggled by user) -->
  {#if showSidebar}
    <aside
      class="sidebar"
      transition:slide={{ axis: 'x', duration: 250, easing: cubicOut }}
    >
      <div class="sidebar-inner">
        <div class="sidebar-header">
          <span class="section-title">Subjects</span>
          <button
            class="btn-sidebar-collapse"
            onclick={() => showSidebar = false}
            title="Collapse sidebar"
            aria-label="Collapse sidebar"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="15 18 9 12 15 6"></polyline>
            </svg>
          </button>
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
      </div>
    </aside>
  {/if}

  <!-- Right Main Area: Weekly Calendar -->
  <main class="calendar-area">
    <WeeklyCalendar 
      {blocks}
      {overlayEvents}
      {calendars}
      {syncedCalendar}
      {showLocalCalendar}
      {showSyncedCalendar}
      syncedCalendarId={syncedCalendar?.id || null}
      syncedCalendarSummary={syncedCalendar?.summary || null}
      {subjectEvents}
      {weekOffset}
      {isSyncing}
      {showSidebar}
      onToggleSidebar={() => showSidebar = !showSidebar}
      onBlockAdd={handleBlockAdd}
      onBlockDelete={handleBlockDelete}
      onBlockUpdate={handleBlockUpdate}
      onWeekChange={handleWeekChange}
      onSyncClick={handleSyncNow}
      onSettingsClick={() => openSettingsWindow('calendar')}
      onSubjectEventChanged={async () => {
        subjectEvents = await subjectEventsGetAll().catch(() => []);
      }}
    />
  </main>
</div>

<style>
  .planning-view {
    display: flex;
    height: 100%;
    padding: 1rem;
    color: var(--color-foreground);
  }

  /* ── Sidebar ───────────────────────────────────── */
  .sidebar {
    width: 270px;
    margin-right: 1rem;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: transparent;
    border-right: 1px solid var(--color-separator);
    overflow: hidden;
  }

  .sidebar-inner {
    width: 270px;
    min-width: 270px;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    border-bottom: 1px solid var(--color-separator);
  }

  .btn-sidebar-collapse {
    background: none;
    border: none;
    color: var(--color-foreground-darker, #a1a1aa);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, color 0.15s;
  }

  .btn-sidebar-collapse:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--color-foreground);
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
    color: var(--color-foreground-darker);
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
    color: var(--color-foreground-darker);
    font-style: italic;
    opacity: 0.7;
  }

  .empty {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--color-foreground-darker);
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
