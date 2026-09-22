<script lang="ts">
  import { onMount } from 'svelte';
  import type { ScheduledBlock, GoogleOverlayEvent } from '$lib/types';
  import BlockTimelineModal from './BlockTimelineModal.svelte';
  import GoogleEventDetailModal from './GoogleEventDetailModal.svelte';
  import { scheduleUpdateBlock } from '$lib/ipc';

  interface Props {
    blocks: ScheduledBlock[];
    overlayEvents?: GoogleOverlayEvent[];
    showLocalCalendar?: boolean;
    showSyncedCalendar?: boolean;
    syncedCalendarId?: string | null;
    syncedCalendarSummary?: string | null;
    weekOffset?: number;
    isSyncing?: boolean;
    showSidebar?: boolean;
    onToggleSidebar?: () => void;
    onBlockAdd: (day: number, startMin: number, endMin: number, subject: string) => void;
    onBlockDelete: (id: number) => void;
    onBlockUpdate: (id: number, day: number, startMin: number, endMin: number) => void;
    onWeekChange?: (mondayYmd: string, offset: number) => void;
    onSyncClick?: () => void;
    onSettingsClick?: () => void;
  }

  let {
    blocks,
    overlayEvents = [],
    showLocalCalendar = true,
    showSyncedCalendar = true,
    syncedCalendarId = null,
    syncedCalendarSummary = null,
    weekOffset = 0,
    isSyncing = false,
    showSidebar = false,
    onToggleSidebar,
    onBlockAdd,
    onBlockDelete,
    onBlockUpdate,
    onWeekChange,
    onSyncClick,
    onSettingsClick,
  }: Props = $props();

  const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  const HOURS = Array.from({ length: 24 }, (_, i) => i);

  // Snap resolution in minutes
  const SNAP_MINUTES = 15;
  // Visual height representing 1 hour (60px)
  const PIXELS_PER_HOUR = 60;
  const PIXELS_PER_MINUTE = PIXELS_PER_HOUR / 60;

  let now = $state(new Date());
  let calendarBodyRef: HTMLElement | null = $state(null);

  // Week calculation helpers
  function getMonday(offsetWeeks: number = 0): Date {
    const d = new Date();
    const day = d.getDay();
    const diffToMonday = (day + 6) % 7;
    d.setDate(d.getDate() - diffToMonday + (offsetWeeks * 7));
    d.setHours(0, 0, 0, 0);
    return d;
  }

  function getMondayYmd(offsetWeeks: number): string {
    const d = getMonday(offsetWeeks);
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const dayNum = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${dayNum}`;
  }

  function formatWeekRange(monday: Date): string {
    const sunday = new Date(monday.getTime() + 6 * 86400000);
    const mMonth = monday.toLocaleDateString(undefined, { month: 'short' });
    const sMonth = sunday.toLocaleDateString(undefined, { month: 'short' });
    const mYear = monday.getFullYear();
    const sYear = sunday.getFullYear();

    if (mYear !== sYear) {
      return `${mMonth} ${monday.getDate()}, ${mYear} – ${sMonth} ${sunday.getDate()}, ${sYear}`;
    } else if (mMonth !== sMonth) {
      return `${mMonth} ${monday.getDate()} – ${sMonth} ${sunday.getDate()}, ${mYear}`;
    } else {
      return `${mMonth} ${monday.getDate()} – ${sunday.getDate()}, ${mYear}`;
    }
  }

  let mondayDate = $derived(getMonday(weekOffset));
  let formattedWeekRange = $derived(formatWeekRange(mondayDate));

  let weekDays = $derived(
    DAYS.map((name, i) => {
      const d = new Date(mondayDate.getTime() + i * 86400000);
      const isToday =
        d.getFullYear() === now.getFullYear() &&
        d.getMonth() === now.getMonth() &&
        d.getDate() === now.getDate();
      return {
        name,
        dayNumber: d.getDate(),
        date: d,
        isToday,
      };
    })
  );

  let currentMinute = $derived(now.getHours() * 60 + now.getMinutes());

  // All-day and timed overlay events
  let allDayEvents = $derived(overlayEvents.filter(e => e.is_all_day));
  let timedOverlayEvents = $derived(overlayEvents.filter(e => !e.is_all_day));

  let selectedOverlayEvent = $state<GoogleOverlayEvent | null>(null);

  function getAllDayEventsForDay(dayIdx: number) {
    return allDayEvents.filter(e => e.day_of_week === dayIdx);
  }

  function getOverlayEventsForDay(dayIdx: number) {
    return timedOverlayEvents.filter(e => e.day_of_week === dayIdx);
  }

  function handlePrevWeek() {
    const newOffset = weekOffset - 1;
    onWeekChange?.(getMondayYmd(newOffset), newOffset);
  }

  function handleNextWeek() {
    const newOffset = weekOffset + 1;
    onWeekChange?.(getMondayYmd(newOffset), newOffset);
  }

  function handleToday() {
    if (weekOffset === 0) return;
    onWeekChange?.(getMondayYmd(0), 0);
  }

  async function handleModalSave(updatedBlock: ScheduledBlock) {
    try {
      await scheduleUpdateBlock(
        updatedBlock.id!,
        updatedBlock.day_of_week,
        updatedBlock.start_minute,
        updatedBlock.end_minute,
        updatedBlock.subject_topic,
        updatedBlock.study_type,
        updatedBlock.round_tags
      );

      const idx = blocks.findIndex(b => b.id === updatedBlock.id);
      if (idx !== -1) {
        blocks[idx] = { ...blocks[idx], ...updatedBlock };
      }

      selectedBlockForModal = null;
    } catch (e) {
      console.error('Failed to save block tags', e);
    }
  }

  onMount(() => {
    const interval = setInterval(() => {
      now = new Date();
    }, 60000);

    setTimeout(() => {
      if (calendarBodyRef) {
        const y = currentMinute * PIXELS_PER_MINUTE;
        calendarBodyRef.scrollTop = Math.max(0, y - calendarBodyRef.clientHeight / 3);
      }
    }, 0);

    return () => clearInterval(interval);
  });

  let dragOverCell: { day: number; hour: number } | null = $state(null);
  let activeResize = $state<{
    id: number;
    type: 'top' | 'bottom';
    initialY: number;
    startMin: number;
    endMin: number;
    day: number;
  } | null>(null);

  let activeDrag = $state<{
    id: number;
    initialY: number;
    startMin: number;
    endMin: number;
    startDay: number;
    currentDay: number;
    currentStartMin: number;
    currentEndMin: number;
    calendarRect: DOMRect;
    grabbedAsOverflow: boolean;
  } | null>(null);

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
      try {
        const payloadStr = e.dataTransfer.getData('application/json');
        if (payloadStr) {
          const payload = JSON.parse(payloadStr);

          if (payload.type === 'subject') {
            const target = e.currentTarget as HTMLElement;
            const rect = target.getBoundingClientRect();
            const y = e.clientY - rect.top;

            const rawMinutes = hour * 60 + Math.floor(y / PIXELS_PER_MINUTE);
            const startMin = Math.min(
              24 * 60 - SNAP_MINUTES,
              Math.max(0, Math.round(rawMinutes / SNAP_MINUTES) * SNAP_MINUTES)
            );

            const defaultDuration = 120;
            const endMin = startMin + defaultDuration;
            onBlockAdd(day, startMin, endMin, payload.data);
          }
        }
      } catch {
        const subject = e.dataTransfer.getData('text/plain');
        if (subject) {
          const target = e.currentTarget as HTMLElement;
          const rect = target.getBoundingClientRect();
          const y = e.clientY - rect.top;
          const rawMinutes = hour * 60 + Math.floor(y / PIXELS_PER_MINUTE);
          const startMin = Math.min(
            24 * 60 - SNAP_MINUTES,
            Math.max(0, Math.round(rawMinutes / SNAP_MINUTES) * SNAP_MINUTES)
          );
          const defaultDuration = 120;
          const endMin = startMin + defaultDuration;
          onBlockAdd(day, startMin, endMin, subject);
        }
      }
    }
  }

  let isDraggingOrResizing = false;
  let selectedBlockForModal = $state<ScheduledBlock | null>(null);

  function handleBlockMouseDown(e: MouseEvent, block: ScheduledBlock, isPrimary: boolean = true) {
    if (e.button !== 0) return;

    const target = e.target as HTMLElement;
    if (target.closest('.resize-handle') || target.closest('.btn-delete-block')) {
      return;
    }

    e.preventDefault();
    e.stopPropagation();
    isDraggingOrResizing = false;

    const calendarContainer = document.querySelector('.days-columns') as HTMLElement;
    if (!calendarContainer) return;

    activeDrag = {
      id: block.id!,
      initialY: e.clientY,
      startMin: block.start_minute,
      endMin: block.end_minute,
      startDay: block.day_of_week,
      currentDay: block.day_of_week,
      currentStartMin: block.start_minute,
      currentEndMin: block.end_minute,
      calendarRect: calendarContainer.getBoundingClientRect(),
      grabbedAsOverflow: !isPrimary,
    };

    window.addEventListener('mousemove', handleBlockMouseMove);
    window.addEventListener('mouseup', handleBlockMouseUp);
  }

  function handleBlockMouseMove(e: MouseEvent) {
    if (!activeDrag) return;
    isDraggingOrResizing = true;

    const columnWidth = activeDrag.calendarRect.width / 7;
    const offsetX = e.clientX - activeDrag.calendarRect.left;
    let newDay = Math.floor(offsetX / columnWidth);
    newDay = Math.max(0, Math.min(6, newDay));

    if (activeDrag.grabbedAsOverflow) {
      newDay = (newDay - 1 + 7) % 7;
    }

    const deltaY = e.clientY - activeDrag.initialY;
    const deltaMinutes = Math.floor(deltaY / PIXELS_PER_MINUTE);
    const snappedDelta = Math.round(deltaMinutes / SNAP_MINUTES) * SNAP_MINUTES;

    let newStart = activeDrag.startMin + snappedDelta;
    const duration = activeDrag.endMin - activeDrag.startMin;

    while (newStart >= 24 * 60) {
      newStart -= 24 * 60;
      newDay = (newDay + 1) % 7;
    }
    while (newStart < 0) {
      newStart += 24 * 60;
      newDay = (newDay - 1 + 7) % 7;
    }

    activeDrag.currentDay = newDay;
    activeDrag.currentStartMin = newStart;
    activeDrag.currentEndMin = newStart + duration;
  }

  function handleBlockMouseUp(e: MouseEvent) {
    window.removeEventListener('mousemove', handleBlockMouseMove);
    window.removeEventListener('mouseup', handleBlockMouseUp);

    if (activeDrag) {
      if (e.clientX < activeDrag.calendarRect.left - 20) {
        onBlockDelete(activeDrag.id);
      } else {
        if (activeDrag.currentDay !== activeDrag.startDay || activeDrag.currentStartMin !== activeDrag.startMin) {
          onBlockUpdate(activeDrag.id, activeDrag.currentDay, activeDrag.currentStartMin, activeDrag.currentEndMin);
        }
      }
    }

    activeDrag = null;
  }

  function handleBlockClick(e: MouseEvent, block: ScheduledBlock) {
    if (isDraggingOrResizing) return;

    const target = e.target as HTMLElement;
    if (target.closest('.resize-handle') || target.closest('.btn-delete-block')) {
      return;
    }

    selectedBlockForModal = block;
  }

  let currentResizeState = $state<{ startMin: number; endMin: number } | null>(null);

  function handleResizeStart2(e: MouseEvent, block: ScheduledBlock, type: 'top' | 'bottom') {
    e.stopPropagation();
    e.preventDefault();
    activeResize = {
      id: block.id!,
      type,
      initialY: e.clientY,
      startMin: block.start_minute,
      endMin: block.end_minute,
      day: block.day_of_week,
    };
    currentResizeState = { startMin: block.start_minute, endMin: block.end_minute };
    window.addEventListener('mousemove', handleResizeMove2);
    window.addEventListener('mouseup', handleResizeEnd2);
  }

  function handleResizeMove2(e: MouseEvent) {
    if (!activeResize || !currentResizeState) return;
    isDraggingOrResizing = true;

    const deltaY = e.clientY - activeResize.initialY;
    const deltaMinutes = Math.floor(deltaY / PIXELS_PER_MINUTE);
    const snappedDelta = Math.round(deltaMinutes / SNAP_MINUTES) * SNAP_MINUTES;

    if (activeResize.type === 'bottom') {
      let newEnd = activeResize.endMin + snappedDelta;
      if (newEnd <= activeResize.startMin + SNAP_MINUTES) newEnd = activeResize.startMin + SNAP_MINUTES;
      if (newEnd > 48 * 60) newEnd = 48 * 60;
      currentResizeState.endMin = newEnd;
    } else {
      let newStart = activeResize.startMin + snappedDelta;
      if (newStart >= activeResize.endMin - SNAP_MINUTES) newStart = activeResize.endMin - SNAP_MINUTES;
      if (newStart < 0) newStart = 0;
      currentResizeState.startMin = newStart;
    }
  }

  function handleResizeEnd2() {
    window.removeEventListener('mousemove', handleResizeMove2);
    window.removeEventListener('mouseup', handleResizeEnd2);

    if (activeResize && currentResizeState) {
      if (currentResizeState.startMin !== activeResize.startMin || currentResizeState.endMin !== activeResize.endMin) {
        onBlockUpdate(activeResize.id, activeResize.day, currentResizeState.startMin, currentResizeState.endMin);
      }
    }

    activeResize = null;
    currentResizeState = null;
  }

  function formatTime(minutes: number): string {
    const h = Math.floor(minutes / 60) % 24;
    const m = minutes % 60;
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}`;
  }

  function getSegmentsForDay(dayIdx: number) {
    const segments = [];
    const visibleBlocks = blocks.filter(b => {
      if (b.calendar_type === 'google') {
        if (!showSyncedCalendar) return false;
        if (syncedCalendarId && b.google_calendar_id && b.google_calendar_id !== syncedCalendarId) {
          return false;
        }
        return true;
      }
      return showLocalCalendar;
    });

    for (const block of visibleBlocks) {
      const isResizing = activeResize?.id === block.id;
      const isDragging = activeDrag?.id === block.id;

      const blockDay = isDragging && activeDrag ? activeDrag.currentDay : block.day_of_week;
      const startMin =
        isDragging && activeDrag
          ? activeDrag.currentStartMin
          : isResizing && currentResizeState
            ? currentResizeState.startMin
            : block.start_minute;
      const endMin =
        isDragging && activeDrag
          ? activeDrag.currentEndMin
          : isResizing && currentResizeState
            ? currentResizeState.endMin
            : block.end_minute;

      if (blockDay === dayIdx) {
        segments.push({
          block,
          idKey: block.id + '-p',
          startMin,
          endMin: Math.min(endMin, 24 * 60),
          isPrimary: true,
          originalStart: startMin,
          originalEnd: endMin,
          isResizing,
          isDragging,
        });
      }

      if (endMin > 24 * 60) {
        const nextDay = (blockDay + 1) % 7;
        if (nextDay === dayIdx) {
          segments.push({
            block,
            idKey: block.id + '-o',
            startMin: 0,
            endMin: endMin - 24 * 60,
            isPrimary: false,
            originalStart: startMin,
            originalEnd: endMin,
            isResizing,
            isDragging,
          });
        }
      }
    }
    return segments;
  }
</script>

<div class="calendar-container">
  <!-- Week Navigation Toolbar -->
  <div class="calendar-toolbar">
    <div class="toolbar-nav-group">
      {#if onToggleSidebar}
        <button
          class="btn-toggle-sidebar"
          class:is-active={showSidebar}
          onclick={onToggleSidebar}
          title={showSidebar ? "Hide Subjects Sidebar" : "Show Subjects Sidebar"}
          aria-label="Toggle Subjects Sidebar"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect width="18" height="18" x="3" y="3" rx="2" ry="2"/>
            <line x1="9" x2="9" y1="3" y2="21"/>
          </svg>
        </button>
      {/if}

      <div class="nav-buttons">
        <button
          class="btn-nav"
          onclick={handlePrevWeek}
          title="Previous week"
          aria-label="Previous week"
        >
          ‹
        </button>
        <button
          class="btn-nav btn-today"
          class:is-active={weekOffset === 0}
          onclick={handleToday}
        >
          Today
        </button>
        <button
          class="btn-nav"
          onclick={handleNextWeek}
          title="Next week"
          aria-label="Next week"
        >
          ›
        </button>
      </div>
      <span class="week-date-range">{formattedWeekRange}</span>
    </div>

    <div class="toolbar-actions">
      {#if syncedCalendarSummary}
        <div class="synced-badge" title="Two-way synced with Google Calendar: {syncedCalendarSummary}">
          <span class="sync-dot"></span>
          <span class="sync-text">⇄ {syncedCalendarSummary}</span>
        </div>
      {/if}

      {#if onSyncClick}
        <button
          class="btn-sync"
          onclick={onSyncClick}
          disabled={isSyncing}
          title="Sync with Google Calendar"
        >
          <svg
            class="sync-icon"
            class:spinning={isSyncing}
            width="13"
            height="13"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67" />
          </svg>
          <span>{isSyncing ? 'Syncing...' : 'Sync'}</span>
        </button>
      {/if}

      {#if onSettingsClick}
        <button
          class="btn-calendar-settings"
          onclick={onSettingsClick}
          title="Configure Calendars in Settings"
          aria-label="Configure Calendars in Settings"
        >
          ⚙️
        </button>
      {/if}
    </div>
  </div>

  <div class="calendar-body" bind:this={calendarBodyRef}>
    <!-- Sticky Day Headers -->
    <div class="calendar-header">
      <div class="time-column-header"></div>
      {#each weekDays as day}
        <div class="day-header" class:is-today={day.isToday}>
          <span class="day-name">{day.name}</span>
          <span class="day-number" class:today-pill={day.isToday}>{day.dayNumber}</span>
        </div>
      {/each}
    </div>

    <!-- All-Day Events Row (if any exist for current week) -->
    {#if allDayEvents.length > 0}
      <div class="all-day-row">
        <div class="time-column-header all-day-label">all-day</div>
        <div class="days-columns all-day-days">
          {#each weekDays as _, dayIdx}
            <div class="day-column all-day-cell">
              {#each getAllDayEventsForDay(dayIdx) as ev (ev.id)}
                <button
                  class="overlay-all-day-badge"
                  style="background-color: {ev.calendar_color};"
                  onclick={() => selectedOverlayEvent = ev}
                  title="{ev.summary} ({ev.calendar_summary})"
                >
                  <span class="overlay-badge-text">{ev.summary}</span>
                </button>
              {/each}
            </div>
          {/each}
        </div>
      </div>
    {/if}

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
        {#each weekDays as day, dayIdx}
          <div class="day-column" class:is-today={day.isToday}>
            <!-- Current Time Marker (only on today) -->
            {#if day.isToday}
              <div
                class="time-marker"
                style="top: {currentMinute * PIXELS_PER_MINUTE}px;"
              ></div>
            {/if}

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

            <!-- Overlay Events (Read-only from Google Calendar) -->
            {#each getOverlayEventsForDay(dayIdx) as ev (ev.id + '-' + ev.day_of_week)}
              {@const top = ev.start_minute * PIXELS_PER_MINUTE}
              {@const height = Math.max(20, (ev.end_minute - ev.start_minute) * PIXELS_PER_MINUTE)}
              <button
                class="overlay-block"
                style="top: {top}px; height: {height}px; border-left-color: {ev.calendar_color};"
                onclick={(e) => {
                  e.stopPropagation();
                  selectedOverlayEvent = ev;
                }}
                title="{ev.summary} ({formatTime(ev.start_minute)} – {formatTime(ev.end_minute)}) • {ev.calendar_summary}"
              >
                <div class="overlay-badge-pill" style="background-color: {ev.calendar_color};"></div>
                <div class="overlay-content">
                  <span class="overlay-title">{ev.summary}</span>
                  {#if height >= 30}
                    <span class="overlay-time">{formatTime(ev.start_minute)} – {formatTime(ev.end_minute)}</span>
                  {/if}
                </div>
              </button>
            {/each}

            <!-- Pomotroid Scheduled Blocks -->
            {#each getSegmentsForDay(dayIdx) as seg (seg.idKey)}
              {@const top = seg.startMin * PIXELS_PER_MINUTE}
              {@const height = (seg.endMin - seg.startMin) * PIXELS_PER_MINUTE}
              {@const isGoogleSynced = seg.block.calendar_type === 'google' || !!seg.block.google_event_id}

              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="scheduled-block"
                class:resizing={seg.isResizing}
                class:dragging={seg.isDragging}
                class:is-overflow={!seg.isPrimary}
                class:is-google-synced={isGoogleSynced}
                class:is-short={height < 30}
                onmousedown={(e) => handleBlockMouseDown(e, seg.block, seg.isPrimary)}
                onclick={(e) => handleBlockClick(e, seg.block)}
                style="top: {top}px; height: {height}px;"
                title="{seg.block.subject} ({formatTime(seg.originalStart)} - {formatTime(seg.originalEnd)}){isGoogleSynced ? ' • Synced with Google' : ''}"
              >
                <!-- Resize top handle -->
                {#if seg.isPrimary}
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div class="resize-handle top" onmousedown={(e) => handleResizeStart2(e, seg.block, 'top')}></div>
                {/if}

                <div class="block-content">
                  {#if height < 30}
                    <div class="block-compact-line">
                      <span class="block-subject">{seg.block.subject}</span>
                      <span class="block-time">
                        {#if !seg.isPrimary}
                          (Cont.)
                        {/if}
                        {formatTime(seg.originalStart)} - {formatTime(seg.originalEnd)}
                      </span>
                      {#if isGoogleSynced && seg.isPrimary}
                        <span class="gcal-sync-badge" title="Synced with Google Calendar">
                          <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                            <path d="m17 2 4 4-4 4"/>
                            <path d="M3 11v-1a4 4 0 0 1 4-4h14"/>
                            <path d="m7 22-4-4 4-4"/>
                            <path d="M21 13v1a4 4 0 0 1-4 4H3"/>
                          </svg>
                        </span>
                      {/if}
                    </div>
                  {:else}
                    <div class="block-header-line">
                      <span class="block-subject">{seg.block.subject}</span>
                    </div>
                    <div class="block-footer-line">
                      <span class="block-time">
                        {#if !seg.isPrimary}
                          (Cont.)
                        {/if}
                        {formatTime(seg.originalStart)} - {formatTime(seg.originalEnd)}
                      </span>
                      {#if isGoogleSynced && seg.isPrimary}
                        <span class="gcal-sync-badge" title="Synced with Google Calendar">
                          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                            <path d="m17 2 4 4-4 4"/>
                            <path d="M3 11v-1a4 4 0 0 1 4-4h14"/>
                            <path d="m7 22-4-4 4-4"/>
                            <path d="M21 13v1a4 4 0 0 1-4 4H3"/>
                          </svg>
                        </span>
                      {/if}
                    </div>
                  {/if}
                </div>

                {#if seg.isPrimary}
                  <button
                    class="btn-delete-block"
                    onclick={() => onBlockDelete(seg.block.id!)}
                    title="Remove block"
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <line x1="18" y1="6" x2="6" y2="18"></line>
                      <line x1="6" y1="6" x2="18" y2="18"></line>
                    </svg>
                  </button>
                {/if}

                <!-- Resize bottom handle -->
                {#if !seg.isPrimary || (seg.isPrimary && seg.originalEnd <= 24 * 60)}
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div class="resize-handle bottom" onmousedown={(e) => handleResizeStart2(e, seg.block, 'bottom')}></div>
                {/if}
              </div>
            {/each}
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

{#if selectedBlockForModal}
  <BlockTimelineModal
    block={selectedBlockForModal}
    onClose={() => selectedBlockForModal = null}
    onSave={handleModalSave}
  />
{/if}

{#if selectedOverlayEvent}
  <GoogleEventDetailModal
    event={selectedOverlayEvent}
    onClose={() => selectedOverlayEvent = null}
  />
{/if}

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

  /* ── Week Navigation Toolbar ──────────────────────────────── */
  .calendar-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    background: var(--color-background);
    border-bottom: 1px solid var(--color-separator);
    flex-shrink: 0;
    gap: 1rem;
  }

  .toolbar-nav-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .btn-toggle-sidebar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 0.35rem;
    color: var(--color-foreground-darker, #a1a1aa);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .btn-toggle-sidebar:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--color-text);
  }

  .btn-toggle-sidebar.is-active {
    color: var(--color-focus-round, #4285f4);
    background: transparent;
  }

  .btn-toggle-sidebar.is-active:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .nav-buttons {
    display: flex;
    align-items: center;
    background: transparent;
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    overflow: hidden;
  }

  .btn-nav {
    background: transparent;
    border: none;
    color: var(--color-text);
    padding: 0.3rem 0.6rem;
    font-size: 0.85rem;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .btn-nav:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .btn-today {
    font-weight: 500;
    border-left: 1px solid var(--color-separator);
    border-right: 1px solid var(--color-separator);
    padding: 0.3rem 0.75rem;
  }

  .btn-today.is-active {
    color: var(--color-focus-round);
    font-weight: 600;
  }

  .week-date-range {
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--color-text);
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .synced-badge {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: rgba(66, 133, 244, 0.12);
    border: 1px solid rgba(66, 133, 244, 0.3);
    color: #4285f4;
    padding: 0.25rem 0.6rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .sync-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #34a853;
  }

  .btn-sync {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--color-separator);
    color: var(--color-text);
    padding: 0.3rem 0.7rem;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }

  .btn-sync:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
    border-color: var(--color-subtext, rgba(255, 255, 255, 0.3));
  }

  .btn-sync:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-calendar-settings {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    padding: 0.3rem 0.6rem;
    font-size: 0.85rem;
    cursor: pointer;
    color: var(--color-foreground-darker, #a1a1aa);
    transition: background 0.15s, border-color 0.15s, color 0.15s;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .btn-calendar-settings:hover {
    background: rgba(255, 255, 255, 0.12);
    border-color: var(--color-subtext, rgba(255, 255, 255, 0.3));
    color: var(--color-foreground, #fff);
  }

  .sync-icon.spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  /* ── Sticky Header ────────────────────────────────────────── */
  .calendar-header {
    display: flex;
    border-bottom: 1px solid var(--color-separator);
    background: var(--color-background);
    position: sticky;
    top: 0;
    z-index: 20;
  }

  .time-column-header {
    width: 60px;
    flex-shrink: 0;
    border-right: 1px solid var(--color-separator);
  }

  .day-header {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 0.5rem 0;
    gap: 2px;
    border-right: 1px solid var(--color-separator);
  }

  .day-header:last-child {
    border-right: none;
  }

  .day-name {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-foreground-darker);
  }

  .day-number {
    font-size: 1rem;
    font-weight: 600;
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    color: var(--color-text);
  }

  .day-number.today-pill {
    background: var(--color-focus-round);
    color: var(--color-background);
  }

  .day-header.is-today .day-name {
    color: var(--color-focus-round);
  }

  /* ── All-Day Row ──────────────────────────────────────────── */
  .all-day-row {
    display: flex;
    border-bottom: 1px solid var(--color-separator);
    background: rgba(255, 255, 255, 0.02);
    min-height: 30px;
    z-index: 18;
  }

  .all-day-label {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--color-foreground-darker);
  }

  .all-day-days {
    display: flex;
    flex: 1;
  }

  .all-day-cell {
    flex: 1;
    padding: 3px 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 28px;
    border-right: 1px solid var(--color-separator);
  }

  .all-day-cell:last-child {
    border-right: none;
  }

  .overlay-all-day-badge {
    border: none;
    border-radius: 3px;
    padding: 2px 6px;
    font-size: 0.7rem;
    font-weight: 500;
    color: #ffffff;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
    opacity: 0.9;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    transition: opacity 0.15s, transform 0.1s;
  }

  .overlay-all-day-badge:hover {
    opacity: 1;
    transform: translateY(-1px);
  }

  /* ── Calendar Body & Grid ─────────────────────────────────── */
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
    border-right: 1px solid var(--color-separator);
    background: transparent;
  }

  .time-label {
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 4px;
    color: var(--color-foreground-darker);
    font-size: 0.75rem;
    border-bottom: 1px solid transparent;
  }

  .days-columns {
    display: flex;
    flex: 1;
  }

  .day-column {
    flex: 1;
    border-right: 1px solid var(--color-separator);
    position: relative;
  }

  .day-column:last-child {
    border-right: none;
  }

  .hour-cell {
    border-bottom: 1px solid var(--color-separator);
    transition: background-color 0.1s;
  }

  .time-marker {
    position: absolute;
    left: 0;
    right: -1px;
    height: 2px;
    background: #ef4444;
    z-index: 15;
    pointer-events: none;
  }

  .time-marker::before {
    content: '';
    position: absolute;
    left: -4px;
    top: -4px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #ef4444;
  }

  .hour-cell.drag-over {
    background: rgba(255, 255, 255, 0.03);
    box-shadow: inset 0 0 0 1px var(--color-subtext, rgba(255, 255, 255, 0.3));
  }

  /* ── Overlay Events (Google Calendar) ─────────────────────── */
  .overlay-block {
    position: absolute;
    left: 3px;
    right: 3px;
    border-radius: 4px;
    padding: 3px 6px;
    overflow: hidden;
    display: flex;
    align-items: flex-start;
    gap: 5px;
    cursor: pointer;
    z-index: 5;
    background: rgba(30, 30, 45, 0.82);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-left: 3px solid;
    text-align: left;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    transition: background 0.15s, border-color 0.15s, z-index 0.15s;
  }

  .overlay-block:hover {
    z-index: 8;
    background: rgba(40, 40, 60, 0.95);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
  }

  .overlay-badge-pill {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    margin-top: 4px;
    flex-shrink: 0;
  }

  .overlay-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .overlay-title {
    font-weight: 600;
    font-size: 0.75rem;
    color: var(--color-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .overlay-time {
    font-size: 0.65rem;
    color: var(--color-foreground-darker);
    opacity: 0.9;
  }

  /* ── Pomotroid Scheduled Blocks ───────────────────────────── */
  .scheduled-block {
    position: absolute;
    left: 2px;
    right: 2px;
    background: var(--color-focus-round);
    color: var(--color-background);
    border-radius: 4px;
    padding: 4px 6px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    z-index: 10;
    transition: opacity 0.2s, box-shadow 0.2s;
    cursor: grab;
    border: 1px solid var(--color-background);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .scheduled-block.is-short {
    padding: 1px 4px;
    border-radius: 3px;
  }

  .scheduled-block.is-google-synced {
    border-left: 3px solid #4285f4;
  }

  .scheduled-block:active {
    cursor: grabbing;
  }

  .scheduled-block.resizing,
  .scheduled-block.dragging {
    transition: none;
    z-index: 12;
    opacity: 0.9;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4), 0 0 0 1px var(--color-background);
  }

  .scheduled-block.dragging {
    opacity: 0.7;
  }

  .scheduled-block:hover {
    z-index: 11;
  }

  .resize-handle {
    height: 8px;
    position: absolute;
    left: 0;
    right: 0;
    cursor: ns-resize;
    z-index: 15;
  }

  .scheduled-block.is-short .resize-handle {
    height: 4px;
  }

  .resize-handle.top {
    top: 0;
  }

  .resize-handle.bottom {
    bottom: 0;
  }

  .resize-handle:hover {
    background: rgba(255, 255, 255, 0.2);
  }

  .block-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-right: 22px;
    min-width: 0;
    pointer-events: none;
  }

  .scheduled-block.is-short .block-content {
    padding-right: 16px;
    justify-content: center;
  }

  .block-header-line {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    min-width: 0;
  }

  .block-footer-line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 4px;
    margin-top: auto;
    overflow: hidden;
    min-width: 0;
  }

  .block-compact-line {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    min-width: 0;
    line-height: 1.1;
  }

  .block-compact-line .block-subject {
    font-size: 0.7rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .block-compact-line .block-time {
    font-size: 0.62rem;
    opacity: 0.85;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .block-compact-line .gcal-sync-badge {
    width: 11px;
    height: 11px;
    padding: 1px;
    flex-shrink: 0;
  }

  .block-compact-line .gcal-sync-badge svg {
    width: 7px;
    height: 7px;
  }

  .block-subject {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .gcal-sync-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: #4285f4;
    background: rgba(255, 255, 255, 0.9);
    border-radius: 50%;
    padding: 2px;
    flex-shrink: 0;
    width: 14px;
    height: 14px;
  }

  .block-time {
    font-size: 0.7rem;
    opacity: 0.9;
  }

  .btn-delete-block {
    position: absolute;
    top: 4px;
    right: 4px;
    background: rgba(0, 0, 0, 0.25);
    color: var(--color-background);
    border: none;
    border-radius: 4px;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0;
    z-index: 20;
    transition: opacity 0.2s, background 0.2s;
  }

  .scheduled-block.is-short .btn-delete-block {
    top: 1px;
    right: 2px;
    width: 13px;
    height: 13px;
    border-radius: 2px;
  }

  .scheduled-block.is-short .btn-delete-block svg {
    width: 8px;
    height: 8px;
  }

  .scheduled-block:hover .btn-delete-block {
    opacity: 1;
  }

  .btn-delete-block:hover {
    background: rgba(0, 0, 0, 0.55);
  }
</style>
