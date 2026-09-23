<script lang="ts">
  import { onMount } from 'svelte';
  import { settings } from '$lib/stores/settings';
  import type { ScheduledBlock, GoogleOverlayEvent, GoogleCalendarItem, SubjectEvent } from '$lib/types';
  import BlockTimelineModal from './BlockTimelineModal.svelte';
  import GoogleEventDetailModal from './GoogleEventDetailModal.svelte';
  import EventModal from './EventModal.svelte';
  import { getSubjectColor, getContrastColor } from '$lib/utils/subjectColors';
  import { scheduleUpdateBlock } from '$lib/ipc';

  interface Props {
    blocks: ScheduledBlock[];
    overlayEvents?: GoogleOverlayEvent[];
    calendars?: GoogleCalendarItem[];
    syncedCalendar?: GoogleCalendarItem | null;
    showLocalCalendar?: boolean;
    showSyncedCalendar?: boolean;
    syncedCalendarId?: string | null;
    syncedCalendarSummary?: string | null;
    subjectEvents?: SubjectEvent[];
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
    onSubjectEventClick?: (event: SubjectEvent) => void;
    onSubjectEventChanged?: () => void;
  }

  let {
    blocks,
    overlayEvents = [],
    calendars = [],
    syncedCalendar = null,
    showLocalCalendar = true,
    showSyncedCalendar = true,
    syncedCalendarId = null,
    syncedCalendarSummary = null,
    subjectEvents = [],
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
    onSubjectEventClick,
    onSubjectEventChanged,
  }: Props = $props();

  function getContrastTextColor(hexBg: string | undefined | null, fallbackFg?: string | null): string {
    if (fallbackFg && fallbackFg.trim().length > 0) {
      return fallbackFg;
    }
    if (!hexBg || !hexBg.startsWith('#')) {
      return '#ffffff';
    }
    let hex = hexBg.replace('#', '');
    if (hex.length === 3) {
      hex = hex.split('').map(c => c + c).join('');
    }
    if (hex.length !== 6) {
      return '#ffffff';
    }
    const r = parseInt(hex.substring(0, 2), 16);
    const g = parseInt(hex.substring(2, 4), 16);
    const b = parseInt(hex.substring(4, 6), 16);
    const yiq = (r * 299 + g * 587 + b * 114) / 1000;
    return yiq >= 128 ? '#000000' : '#ffffff';
  }

  interface BlockTimingInfo {
    rounds: number;
    studyMins: number;
    estSessionMins: number;
    formattedStudyTime: string;
    formattedSessionTime: string;
  }

  function formatMinsDuration(totalMinutes: number): string {
    if (totalMinutes <= 0) return '0m';
    const h = Math.floor(totalMinutes / 60);
    const m = totalMinutes % 60;
    if (h > 0 && m > 0) return `${h}h${m.toString().padStart(2, '0')}`;
    if (h > 0) return `${h}h`;
    return `${m}m`;
  }

  function getBlockTimingInfo(startMin: number, endMin: number): BlockTimingInfo {
    const workMins = $settings.time_work_secs / 60;
    if (workMins <= 0) {
      return {
        rounds: 0,
        studyMins: 0,
        estSessionMins: 0,
        formattedStudyTime: '0m',
        formattedSessionTime: '0m',
      };
    }
    const shortBreakMins = $settings.short_breaks_enabled ? ($settings.time_short_break_secs / 60) : 0;
    const longBreakMins = $settings.long_breaks_enabled ? ($settings.time_long_break_secs / 60) : 0;
    const interval = $settings.long_break_interval;

    let remainingMins = endMin - startMin;
    let rounds = 0;
    let cyclePosition = 1;
    let breakMinsUsed = 0;

    while (remainingMins >= workMins) {
      remainingMins -= workMins;
      rounds++;
      if (remainingMins <= 0) break;

      const isLongBreak = interval > 0 && cyclePosition % interval === 0;
      const bMins = isLongBreak ? longBreakMins : shortBreakMins;

      if (remainingMins >= workMins + bMins) {
        breakMinsUsed += bMins;
        remainingMins -= bMins;
        cyclePosition = isLongBreak ? 1 : cyclePosition + 1;
      } else {
        break;
      }
    }

    const studyMins = Math.round(rounds * workMins);
    const estSessionMins = rounds > 0 ? (studyMins + Math.round(breakMinsUsed)) : 0;

    return {
      rounds,
      studyMins,
      estSessionMins,
      formattedStudyTime: formatMinsDuration(studyMins),
      formattedSessionTime: formatMinsDuration(estSessionMins),
    };
  }

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

  function formatYmd(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  function parseTimeToMinute(timeStr: string): number {
    const [hh, mm] = timeStr.split(':').map(Number);
    return (hh || 0) * 60 + (mm || 0);
  }

  function getTypeIcon(type: string): string {
    switch (type) {
      case 'exam': return '📝';
      case 'assignment': return '📋';
      case 'project': return '🚀';
      case 'quiz': return '💡';
      default: return '📌';
    }
  }

  function isSubjectEventVisible(ev: SubjectEvent): boolean {
    if (ev.calendar_type === 'google') {
      if (!showSyncedCalendar) return false;
      if (ev.google_calendar_id && calendars.length > 0) {
        const cal = calendars.find(c => c.id === ev.google_calendar_id);
        if (cal && !cal.is_visible) return false;
      }
      return true;
    }
    return showLocalCalendar;
  }

  function getSubjectEventsForDay(dayIdx: number): SubjectEvent[] {
    if (!subjectEvents || subjectEvents.length === 0) return [];
    const day = weekDays[dayIdx];
    if (!day) return [];
    const ymd = formatYmd(day.date);
    return subjectEvents.filter(ev => ev.event_date === ymd && isSubjectEventVisible(ev));
  }

  function getTimedSubjectEventsForDay(dayIdx: number): SubjectEvent[] {
    return getSubjectEventsForDay(dayIdx).filter(ev => !ev.is_all_day && !!ev.event_time);
  }

  let selectedSubjectEventForEdit = $state<SubjectEvent | null>(null);

  function handleSubjectEventClick(ev: SubjectEvent) {
    if (onSubjectEventClick) {
      onSubjectEventClick(ev);
    } else {
      selectedSubjectEventForEdit = ev;
    }
  }

  let currentWeekSubjectEvents = $derived(
    weekDays.flatMap((_, idx) => getSubjectEventsForDay(idx))
  );

  let hasEventsOrAllDay = $derived(allDayEvents.length > 0 || currentWeekSubjectEvents.length > 0);

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
        updatedBlock.round_tags,
        getMondayYmd(weekOffset)
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

  function getDayLayout(dayIdx: number) {
    const rawSegments = getSegmentsForDay(dayIdx);
    const overlayList = timedOverlayEvents.filter(e => e.day_of_week === dayIdx);

    type LayoutItem = {
      kind: 'overlay' | 'block';
      startMin: number;
      endMin: number;
      col: number;
      totalCols: number;
      overlayEvent?: GoogleOverlayEvent;
      segment?: any;
    };

    const items: LayoutItem[] = [
      ...overlayList.map(ev => ({
        kind: 'overlay' as const,
        startMin: ev.start_minute,
        endMin: Math.max(ev.start_minute + 15, ev.end_minute),
        col: 0,
        totalCols: 1,
        overlayEvent: ev,
      })),
      ...rawSegments.map(seg => ({
        kind: 'block' as const,
        startMin: seg.startMin,
        endMin: Math.max(seg.startMin + 15, seg.endMin),
        col: 0,
        totalCols: 1,
        segment: seg,
      })),
    ];

    if (items.length === 0) {
      return { overlays: [], segments: [] };
    }

    // Sort: startMin asc, then duration desc
    items.sort((a, b) => {
      if (a.startMin !== b.startMin) return a.startMin - b.startMin;
      return (b.endMin - b.startMin) - (a.endMin - a.startMin);
    });

    // Group into overlapping clusters
    const clusters: LayoutItem[][] = [];
    let currentCluster: LayoutItem[] = [];
    let clusterEnd = -1;

    for (const item of items) {
      if (currentCluster.length === 0) {
        currentCluster.push(item);
        clusterEnd = item.endMin;
      } else {
        if (item.startMin < clusterEnd) {
          currentCluster.push(item);
          clusterEnd = Math.max(clusterEnd, item.endMin);
        } else {
          clusters.push(currentCluster);
          currentCluster = [item];
          clusterEnd = item.endMin;
        }
      }
    }
    if (currentCluster.length > 0) {
      clusters.push(currentCluster);
    }

    // Assign columns within each cluster using greedy coloring
    for (const cluster of clusters) {
      const colEnds: number[] = [];
      for (const item of cluster) {
        let placed = false;
        for (let c = 0; c < colEnds.length; c++) {
          if (colEnds[c] <= item.startMin) {
            colEnds[c] = item.endMin;
            item.col = c;
            placed = true;
            break;
          }
        }
        if (!placed) {
          item.col = colEnds.length;
          colEnds.push(item.endMin);
        }
      }

      const totalCols = colEnds.length;
      for (const item of cluster) {
        item.totalCols = totalCols;
      }
    }

    const overlays = items
      .filter(i => i.kind === 'overlay')
      .map(i => ({
        ev: i.overlayEvent!,
        col: i.col,
        totalCols: i.totalCols,
      }));

    const segments = items
      .filter(i => i.kind === 'block')
      .map(i => ({
        ...i.segment!,
        col: i.col,
        totalCols: i.totalCols,
      }));

    return { overlays, segments };
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
          class="btn-nav btn-prev"
          onclick={handlePrevWeek}
          title="Previous week"
          aria-label="Previous week"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 18 9 12 15 6"></polyline>
          </svg>
        </button>
        <button
          class="btn-nav btn-today"
          class:is-active={weekOffset === 0}
          onclick={handleToday}
        >
          Today
        </button>
        <button
          class="btn-nav btn-next"
          onclick={handleNextWeek}
          title="Next week"
          aria-label="Next week"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="9 18 15 12 9 6"></polyline>
          </svg>
        </button>
      </div>
      <span class="week-date-range">{formattedWeekRange}</span>
    </div>

    <div class="toolbar-actions">
      {#if syncedCalendarSummary}
        {@const syncedCalColor = syncedCalendar?.background_color || '#4285f4'}
        <div 
          class="synced-badge" 
          style="--synced-color: {syncedCalColor};"
          title="Two-way synced with Google Calendar: {syncedCalendarSummary}"
        >
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
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
          </svg>
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

    <!-- All-Day Events & Academic Events Row -->
    {#if hasEventsOrAllDay}
      <div class="all-day-row">
        <div class="time-column-header all-day-label">events</div>
        <div class="days-columns all-day-days">
          {#each weekDays as _, dayIdx}
            <div class="day-column all-day-cell">
              <!-- Academic / Subject Events -->
              {#each getSubjectEventsForDay(dayIdx) as ev (ev.id)}
                {@const evBg = getSubjectColor(ev.subject)}
                {@const evFg = getContrastColor(evBg)}
                {@const typeIcon = getTypeIcon(ev.event_type)}
                <button
                  class="subject-event-badge"
                  class:is-completed={ev.is_completed}
                  style="background-color: {evBg}; color: {evFg};"
                  onclick={() => handleSubjectEventClick(ev)}
                  title="{ev.subject}: {ev.name}{ev.event_time ? ' (' + ev.event_time + ')' : ''}{ev.is_completed ? ' • Completed' : ''}"
                >
                  <span class="event-icon">{typeIcon}</span>
                  {#if ev.event_time}
                    <span class="event-time-tag">{ev.event_time}</span>
                  {/if}
                  <span class="overlay-badge-text">{ev.name}</span>
                </button>
              {/each}

              <!-- Google Calendar Overlay All-Day Events -->
              {#each getAllDayEventsForDay(dayIdx) as ev (ev.id)}
                {@const allDayBg = ev.calendar_color}
                {@const allDayFg = getContrastTextColor(allDayBg, ev.calendar_foreground_color)}
                <button
                  class="overlay-all-day-badge"
                  style="background-color: {allDayBg}; color: {allDayFg};"
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
          {@const dayLayout = getDayLayout(dayIdx)}
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

            <!-- Academic / Subject Event Timed Markers -->
            {#each getTimedSubjectEventsForDay(dayIdx) as ev (ev.id)}
              {@const startMin = parseTimeToMinute(ev.event_time!)}
              {@const top = startMin * PIXELS_PER_MINUTE}
              {@const evBg = getSubjectColor(ev.subject)}
              {@const evFg = getContrastColor(evBg)}
              {@const typeIcon = getTypeIcon(ev.event_type)}
              <button
                class="subject-milestone-marker"
                class:is-completed={ev.is_completed}
                style="top: {top}px; background-color: {evBg}; color: {evFg}; border-color: color-mix(in srgb, {evBg} 70%, #000000);"
                onclick={(e) => {
                  e.stopPropagation();
                  handleSubjectEventClick(ev);
                }}
                title="{typeIcon} {ev.subject}: {ev.name} ({ev.event_time}){ev.is_completed ? ' • Completed' : ''}"
              >
                <div class="milestone-content">
                  <span class="milestone-icon">{typeIcon}</span>
                  <span class="milestone-title">{ev.name}</span>
                  <span class="milestone-time">{ev.event_time}</span>
                </div>
              </button>
            {/each}

            <!-- Overlay Events (Read-only from Google Calendar) -->
            {#each dayLayout.overlays as item (item.ev.id + '-' + item.ev.day_of_week)}
              {@const ev = item.ev}
              {@const top = ev.start_minute * PIXELS_PER_MINUTE}
              {@const height = Math.max(20, (ev.end_minute - ev.start_minute) * PIXELS_PER_MINUTE)}
              {@const leftPct = (item.col / item.totalCols) * 100}
              {@const widthPct = (1 / item.totalCols) * 100}
              {@const overlayBg = ev.calendar_color}
              {@const overlayFg = getContrastTextColor(overlayBg, ev.calendar_foreground_color)}
              <button
                class="overlay-block"
                style="top: {top}px; height: {height}px; left: calc({leftPct}% + 2px); width: calc({widthPct}% - 4px); background-color: {overlayBg}; color: {overlayFg}; border-color: color-mix(in srgb, {overlayBg} 70%, #000000);"
                onclick={(e) => {
                  e.stopPropagation();
                  selectedOverlayEvent = ev;
                }}
                title="{ev.summary} ({formatTime(ev.start_minute)} – {formatTime(ev.end_minute)}) • {ev.calendar_summary}"
              >
                <div class="overlay-content">
                  <span class="overlay-title">{ev.summary}</span>
                  {#if height >= 30}
                    <span class="overlay-time">{formatTime(ev.start_minute)} – {formatTime(ev.end_minute)}</span>
                  {/if}
                </div>
              </button>
            {/each}

            <!-- Pomotroid Scheduled Blocks -->
            {#each dayLayout.segments as seg (seg.idKey)}
              {@const top = seg.startMin * PIXELS_PER_MINUTE}
              {@const height = (seg.endMin - seg.startMin) * PIXELS_PER_MINUTE}
              {@const isGoogleSynced = seg.block.calendar_type === 'google' || !!seg.block.google_event_id}
              {@const leftPct = (seg.col / seg.totalCols) * 100}
              {@const widthPct = (1 / seg.totalCols) * 100}
              {@const targetCal = isGoogleSynced ? (calendars.find(c => c.id === seg.block.google_calendar_id) || syncedCalendar) : null}
              {@const blockBg = targetCal ? targetCal.background_color : 'var(--color-focus-round)'}
              {@const blockFg = targetCal ? getContrastTextColor(targetCal.background_color, targetCal.foreground_color) : 'var(--color-background)'}
              {@const timing = getBlockTimingInfo(seg.originalStart, seg.originalEnd)}
              {@const isNarrow = seg.totalCols >= 2}
              {@const hasStudyTime = timing.studyMins > 0}
              {@const hasSessionTime = timing.estSessionMins > 0 && timing.estSessionMins !== timing.studyMins}

              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="scheduled-block"
                class:resizing={seg.isResizing}
                class:dragging={seg.isDragging}
                class:is-overflow={!seg.isPrimary}
                class:is-google-synced={isGoogleSynced}
                class:is-short={height < 32}
                onmousedown={(e) => handleBlockMouseDown(e, seg.block, seg.isPrimary)}
                onclick={(e) => handleBlockClick(e, seg.block)}
                style="top: {top}px; height: {height}px; left: calc({leftPct}% + 2px); width: calc({widthPct}% - 4px); background-color: {blockBg}; color: {blockFg}; border-color: color-mix(in srgb, {blockBg} 70%, #000000);"
                title="{seg.block.subject} ({formatTime(seg.originalStart)} - {formatTime(seg.originalEnd)}) • {timing.rounds} {timing.rounds === 1 ? 'round' : 'rounds'} allocated • Study: {timing.formattedStudyTime}{timing.estSessionMins > 0 ? ' • Est. Session: ' + timing.formattedSessionTime : ''}{seg.block.subject_topic ? ' • ' + seg.block.subject_topic : ''}{seg.block.study_type ? ' • ' + seg.block.study_type : ''}{isGoogleSynced ? ' • Synced with Google' : ''}"
              >
                <!-- Resize top handle -->
                {#if seg.isPrimary}
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div class="resize-handle top" onmousedown={(e) => handleResizeStart2(e, seg.block, 'top')}></div>
                {/if}

                <div class="block-content">
                  {#if height < 32}
                    <div class="block-compact-line">
                      <span class="block-subject">{seg.block.subject}</span>
                      <span class="block-metric-badge compact">
                        <span class="rounds-dot"></span>
                        <span class="metric-highlight">{timing.rounds}</span>{#if !isNarrow && hasStudyTime}<span class="metric-sep">·</span><span>{timing.formattedStudyTime}</span>{/if}
                      </span>
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
                    <!-- Line 1: Subject -->
                    <div class="block-header-line">
                      <span class="block-subject">{seg.block.subject}</span>
                    </div>

                    <!-- Line 2: Rounds & Study time (Dynamic - appears if height >= 52) -->
                    {#if height >= 52}
                      <div class="block-metrics-row">
                        <span class="block-metric-badge">
                          <span class="rounds-dot"></span>
                          <span class="metric-highlight">{timing.rounds}</span>
                          {#if hasStudyTime}
                            <span class="metric-sep">·</span>
                            <span>{timing.formattedStudyTime}</span>
                          {/if}
                        </span>
                      </div>
                    {/if}

                    <!-- Topic line if set and height permits -->
                    {#if height >= 115 && (seg.block.subject_topic || seg.block.study_type)}
                      <div class="block-topic-line">
                        <span class="block-topic">
                          {seg.block.subject_topic || ''}{seg.block.subject_topic && seg.block.study_type ? ' • ' : ''}{seg.block.study_type || ''}
                        </span>
                      </div>
                    {/if}

                    <!-- Line 3: Time range & Estimated session duration -->
                    <div class="block-footer-line">
                      <span class="block-time">
                        {#if !seg.isPrimary}
                          (Cont.)
                        {/if}
                        {formatTime(seg.originalStart)} - {formatTime(seg.originalEnd)}
                      </span>
                      {#if height < 52}
                        <span class="block-metric-badge compact">
                          <span class="rounds-dot"></span>
                          <span class="metric-highlight">{timing.rounds}</span>
                          {#if !isNarrow && hasStudyTime}
                            <span class="metric-sep">·</span>
                            <span>{timing.formattedStudyTime}</span>
                          {/if}
                        </span>
                      {:else if hasSessionTime && !isNarrow}
                        <span class="block-session-tag" title="Estimated session time with breaks">
                          ~{timing.formattedSessionTime}
                        </span>
                      {/if}
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

{#if selectedSubjectEventForEdit}
  <EventModal
    event={selectedSubjectEventForEdit}
    onClose={() => selectedSubjectEventForEdit = null}
    onSaved={() => {
      selectedSubjectEventForEdit = null;
      onSubjectEventChanged?.();
    }}
    onDeleted={() => {
      selectedSubjectEventForEdit = null;
      onSubjectEventChanged?.();
    }}
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
    background: color-mix(in srgb, var(--synced-color, #4285f4) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--synced-color, #4285f4) 35%, transparent);
    color: var(--synced-color, #4285f4);
    padding: 0.25rem 0.6rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .sync-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--synced-color, #4285f4);
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
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 0.35rem;
    font-size: 0.85rem;
    cursor: pointer;
    color: var(--color-foreground-darker, #a1a1aa);
    transition: background 0.15s, color 0.15s;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .btn-calendar-settings:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--color-text);
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
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
    opacity: 0.95;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    transition: opacity 0.15s, transform 0.1s;
  }

  .overlay-all-day-badge:hover {
    opacity: 1;
    transform: translateY(-1px);
  }

  .subject-event-badge {
    border: none;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 0.72rem;
    font-weight: 600;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    display: flex;
    align-items: center;
    gap: 4px;
    transition: filter 0.15s, transform 0.1s, opacity 0.15s;
  }

  .subject-event-badge:hover {
    filter: brightness(1.1);
    transform: translateY(-1px);
  }

  .subject-event-badge.is-completed {
    opacity: 0.6;
    text-decoration: line-through;
  }

  .subject-event-badge .event-icon {
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .subject-event-badge .event-time-tag {
    font-size: 0.65rem;
    opacity: 0.85;
    background: rgba(0, 0, 0, 0.2);
    padding: 0 4px;
    border-radius: 2px;
  }

  .subject-milestone-marker {
    position: absolute;
    left: 2px;
    right: 2px;
    height: 22px;
    border: 1px solid;
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 0.72rem;
    font-weight: 600;
    z-index: 8;
    cursor: pointer;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    display: flex;
    align-items: center;
    overflow: hidden;
    transition: filter 0.15s, transform 0.1s;
  }

  .subject-milestone-marker:hover {
    filter: brightness(1.1);
    z-index: 12;
  }

  .subject-milestone-marker.is-completed {
    opacity: 0.6;
    text-decoration: line-through;
  }

  .milestone-content {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    overflow: hidden;
    white-space: nowrap;
  }

  .milestone-icon {
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .milestone-title {
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .milestone-time {
    font-size: 0.65rem;
    opacity: 0.85;
    font-family: monospace;
    flex-shrink: 0;
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
    align-items: center;
    justify-content: flex-end;
    padding-right: 8px;
    color: var(--color-foreground-darker);
    font-size: 0.72rem;
    transform: translateY(-50%);
    user-select: none;
    line-height: 1;
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
    position: relative;
    transition: background-color 0.1s;
  }

  .hour-cell::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    border-bottom: 1px dashed var(--color-separator);
    opacity: 0.25;
    pointer-events: none;
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
    box-sizing: border-box;
    border-radius: 6px;
    padding: 3px 6px;
    overflow: hidden;
    display: flex;
    align-items: flex-start;
    cursor: pointer;
    z-index: 5;
    border: 1px solid rgba(0, 0, 0, 0.12);
    text-align: left;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
    transition: opacity 0.15s, box-shadow 0.15s, z-index 0.15s;
  }

  .overlay-block:hover {
    z-index: 8;
    opacity: 0.95;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
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
    line-height: 1.25;
    color: inherit;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow-wrap: break-word;
    word-break: normal;
  }

  .overlay-time {
    font-size: 0.65rem;
    color: inherit;
    opacity: 0.85;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Pomotroid Scheduled Blocks ───────────────────────────── */
  .scheduled-block {
    position: absolute;
    box-sizing: border-box;
    background: var(--color-focus-round);
    color: var(--color-background);
    border-radius: 6px;
    padding: 4px 6px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    z-index: 10;
    transition: opacity 0.2s, box-shadow 0.2s;
    cursor: grab;
    border: 1px solid rgba(0, 0, 0, 0.12);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  }

  .scheduled-block.is-short {
    padding: 1px 4px;
    border-radius: 3px;
  }

  .scheduled-block:active {
    cursor: grabbing;
  }

  .scheduled-block.resizing,
  .scheduled-block.dragging {
    transition: none;
    z-index: 30;
    opacity: 0.9;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.45), 0 0 0 1px var(--color-background);
  }

  .scheduled-block.dragging {
    opacity: 0.75;
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
    padding-right: 4px;
    min-width: 0;
    pointer-events: none;
  }

  .scheduled-block.is-short .block-content {
    padding-right: 4px;
    justify-content: center;
  }

  .block-header-line {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    min-width: 0;
  }

  .block-metrics-row {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: 2px;
    overflow: hidden;
    min-width: 0;
  }

  .block-metric-badge {
    display: inline-flex;
    align-items: center;
    gap: 3.5px;
    font-size: 0.67rem;
    font-weight: 500;
    color: inherit;
    opacity: 0.92;
    background: color-mix(in srgb, currentColor 14%, transparent);
    padding: 1.5px 6px;
    border-radius: 999px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.25;
    max-width: 100%;
    flex-shrink: 0;
  }

  .block-metric-badge.compact {
    font-size: 0.6rem;
    padding: 0.5px 5px;
    gap: 2.5px;
    opacity: 0.88;
  }

  .metric-highlight {
    font-weight: 600;
  }

  .metric-sep {
    opacity: 0.55;
    font-weight: 400;
    margin: 0 1px;
  }

  .block-session-tag {
    font-size: 0.63rem;
    font-weight: 500;
    color: inherit;
    opacity: 0.78;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .rounds-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: currentColor;
    opacity: 0.85;
    flex-shrink: 0;
  }

  .block-topic-line {
    overflow: hidden;
    min-width: 0;
    margin-top: 1px;
  }

  .block-topic {
    font-size: 0.65rem;
    color: inherit;
    opacity: 0.82;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
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
    color: inherit;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .block-compact-line .block-time {
    font-size: 0.62rem;
    color: inherit;
    opacity: 0.85;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .block-compact-line .gcal-sync-badge {
    color: currentColor;
    background: color-mix(in srgb, currentColor 16%, transparent);
    border-radius: 3px;
    width: 11px;
    height: 11px;
    padding: 1px;
    flex-shrink: 0;
    opacity: 0.85;
  }

  .block-compact-line .gcal-sync-badge svg {
    width: 7px;
    height: 7px;
  }

  .block-subject {
    font-weight: 600;
    font-size: 0.78rem;
    line-height: 1.25;
    color: inherit;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow-wrap: break-word;
    word-break: normal;
    min-width: 0;
  }

  .gcal-sync-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: currentColor;
    background: color-mix(in srgb, currentColor 16%, transparent);
    border-radius: 4px;
    padding: 2px;
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    opacity: 0.85;
  }

  .block-time {
    font-size: 0.7rem;
    color: inherit;
    opacity: 0.9;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .btn-delete-block {
    position: absolute;
    top: 4px;
    right: 4px;
    background: color-mix(in srgb, currentColor 18%, transparent);
    color: currentColor;
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
    background: color-mix(in srgb, currentColor 35%, transparent);
  }
</style>
