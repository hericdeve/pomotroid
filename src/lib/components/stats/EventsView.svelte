<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    subjectEventsGetAll,
    subjectEventToggleCompleted,
    subjectEventDelete,
    subjectsGetAll,
    subjectEventsSync,
    googleCalendarGetCalendars,
  } from '$lib/ipc';
  import type {
    SubjectEvent,
    SubjectStats,
    GoogleCalendarItem,
  } from '$lib/types';
  import { getSubjectColor, getContrastColor } from '$lib/utils/subjectColors';
  import EventModal from './EventModal.svelte';
  import DropdownSelect from '$lib/components/DropdownSelect.svelte';
  import { error as logError } from '@tauri-apps/plugin-log';

  let events = $state<SubjectEvent[]>([]);
  let subjects = $state<SubjectStats[]>([]);
  let calendars = $state<GoogleCalendarItem[]>([]);
  let loading = $state(true);

  // Live ticker updating every second
  let now = $state(new Date());
  let tickerInterval: ReturnType<typeof setInterval> | null = null;

  // Filters & State
  let statusFilter = $state<'upcoming' | 'overdue' | 'completed' | 'all'>('upcoming');
  let filterSubject = $state<string>('');
  let filterType = $state<string>('');
  let searchQuery = $state<string>('');

  let showModal = $state(false);
  let selectedEventForEdit = $state<SubjectEvent | null>(null);

  const TYPE_OPTIONS = ['Exam', 'Assignment', 'Project', 'Quiz', 'Other'];

  const TYPE_CONFIG: Record<string, { label: string; icon: string; color: string }> = {
    exam: { label: 'Exam', icon: '📝', color: '#ef4444' },
    assignment: { label: 'Assignment', icon: '📋', color: '#3b82f6' },
    project: { label: 'Project', icon: '🚀', color: '#10b981' },
    quiz: { label: 'Quiz', icon: '💡', color: '#f59e0b' },
    other: { label: 'Other', icon: '📌', color: '#8b5cf6' },
  };

  async function loadData() {
    try {
      loading = true;
      // Trigger Google Calendar event sync first, then load all data
      await subjectEventsSync().catch(() => {});
      const [evs, subjs, cals] = await Promise.all([
        subjectEventsGetAll().catch(() => []),
        subjectsGetAll().catch(() => []),
        googleCalendarGetCalendars().catch(() => []),
      ]);
      events = evs;
      subjects = subjs;
      calendars = cals;
    } catch (e) {
      logError(`Failed to load events data: ${e}`);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadData();
    tickerInterval = setInterval(() => {
      now = new Date();
    }, 1000);
  });

  onDestroy(() => {
    if (tickerInterval) clearInterval(tickerInterval);
  });

  function getEventTargetDate(ev: SubjectEvent): Date {
    if (ev.is_all_day || !ev.event_time) {
      const [y, m, d] = ev.event_date.split('-').map(Number);
      return new Date(y, m - 1, d, 23, 59, 59);
    }
    const [y, m, d] = ev.event_date.split('-').map(Number);
    const [hh, mm] = ev.event_time.split(':').map(Number);
    return new Date(y, m - 1, d, hh, mm, 0);
  }

  interface CountdownInfo {
    isOverdue: boolean;
    days: number;
    hours: number;
    minutes: number;
    seconds: number;
    formatted: string;
    urgency: 'overdue' | 'urgent' | 'soon' | 'upcoming' | 'completed';
  }

  function getCountdownInfo(ev: SubjectEvent): CountdownInfo {
    if (ev.is_completed) {
      return {
        isOverdue: false,
        days: 0,
        hours: 0,
        minutes: 0,
        seconds: 0,
        formatted: 'Completed',
        urgency: 'completed',
      };
    }

    const target = getEventTargetDate(ev);
    const diffMs = target.getTime() - now.getTime();
    const isOverdue = diffMs < 0;
    const absDiff = Math.abs(diffMs);

    const totalSeconds = Math.floor(absDiff / 1000);
    const days = Math.floor(totalSeconds / 86400);
    const hours = Math.floor((totalSeconds % 86400) / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    let formatted = '';
    if (isOverdue) {
      if (days > 0) {
        formatted = `Overdue by ${days}d ${hours}h`;
      } else if (hours > 0) {
        formatted = `Overdue by ${hours}h ${minutes}m`;
      } else {
        formatted = `Overdue by ${minutes}m ${seconds}s`;
      }
    } else {
      if (days > 0) {
        formatted = `${days}d ${hours}h ${minutes}m`;
      } else if (hours > 0) {
        formatted = `${hours}h ${minutes}m ${seconds}s`;
      } else {
        formatted = `${minutes}m ${seconds}s`;
      }
    }

    let urgency: 'overdue' | 'urgent' | 'soon' | 'upcoming' | 'completed' = 'upcoming';
    if (isOverdue) {
      urgency = 'overdue';
    } else if (days === 0) {
      urgency = 'urgent';
    } else if (days <= 2) {
      urgency = 'soon';
    } else {
      urgency = 'upcoming';
    }

    return {
      isOverdue,
      days,
      hours,
      minutes,
      seconds,
      formatted,
      urgency,
    };
  }

  // Next upcoming milestone for KPI card
  let nextMilestone = $derived.by(() => {
    const upcomingEvents = events
      .filter(e => !e.is_completed && getEventTargetDate(e).getTime() >= now.getTime())
      .sort((a, b) => getEventTargetDate(a).getTime() - getEventTargetDate(b).getTime());
    return upcomingEvents[0] || null;
  });

  let nextMilestoneCountdown = $derived(nextMilestone ? getCountdownInfo(nextMilestone) : null);

  // Counts for KPI cards and filter tabs
  let counts = $derived.by(() => {
    let upcoming = 0;
    let overdue = 0;
    let completed = 0;
    for (const ev of events) {
      if (ev.is_completed) {
        completed++;
      } else {
        const isPast = getEventTargetDate(ev).getTime() < now.getTime();
        if (isPast) overdue++;
        else upcoming++;
      }
    }
    return {
      upcoming,
      overdue,
      completed,
      all: events.length,
    };
  });

  // Distinct list of subjects
  let allSubjectsList = $derived.by(() => {
    const set = new Set<string>();
    subjects.forEach(s => set.add(s.name));
    events.forEach(e => set.add(e.subject));
    return Array.from(set).sort();
  });

  // Filtered and sorted events
  let filteredEvents = $derived.by(() => {
    return events
      .filter(ev => {
        const isPast = getEventTargetDate(ev).getTime() < now.getTime();
        if (statusFilter === 'upcoming' && (ev.is_completed || isPast)) return false;
        if (statusFilter === 'overdue' && (ev.is_completed || !isPast)) return false;
        if (statusFilter === 'completed' && !ev.is_completed) return false;

        if (filterSubject && ev.subject !== filterSubject) return false;
        if (filterType && ev.event_type.toLowerCase() !== filterType.toLowerCase()) return false;

        if (searchQuery.trim()) {
          const q = searchQuery.toLowerCase();
          const matchName = ev.name.toLowerCase().includes(q);
          const matchSubject = ev.subject.toLowerCase().includes(q);
          const matchNotes = ev.notes?.toLowerCase().includes(q) || false;
          if (!matchName && !matchSubject && !matchNotes) return false;
        }

        return true;
      })
      .sort((a, b) => {
        if (statusFilter === 'completed') {
          return getEventTargetDate(b).getTime() - getEventTargetDate(a).getTime();
        }
        return getEventTargetDate(a).getTime() - getEventTargetDate(b).getTime();
      });
  });

  async function handleToggleCompleted(ev: SubjectEvent, e: MouseEvent) {
    e.stopPropagation();
    const newStatus = !ev.is_completed;
    try {
      await subjectEventToggleCompleted(ev.id, newStatus);
      ev.is_completed = newStatus;
      events = [...events];
    } catch (err) {
      logError(`Failed to toggle completed for event ${ev.id}: ${err}`);
    }
  }

  async function handleDeleteEvent(ev: SubjectEvent, e: MouseEvent) {
    e.stopPropagation();
    if (!confirm(`Are you sure you want to delete "${ev.name}"?`)) return;
    try {
      await subjectEventDelete(ev.id);
      events = events.filter(e => e.id !== ev.id);
    } catch (err) {
      logError(`Failed to delete event: ${err}`);
    }
  }

  function handleOpenCreate() {
    selectedEventForEdit = null;
    showModal = true;
  }

  function handleEditEvent(ev: SubjectEvent) {
    selectedEventForEdit = ev;
    showModal = true;
  }

  function handleEventSaved(saved: SubjectEvent) {
    const idx = events.findIndex(e => e.id === saved.id);
    if (idx >= 0) {
      events[idx] = saved;
      events = [...events];
    } else {
      events = [saved, ...events];
    }
  }

  function handleEventDeleted(id: number) {
    events = events.filter(e => e.id !== id);
  }

  function formatDateFriendly(dateStr: string): string {
    const [y, m, d] = dateStr.split('-').map(Number);
    const date = new Date(y, m - 1, d);
    return date.toLocaleDateString(undefined, {
      weekday: 'short',
      month: 'short',
      day: 'numeric',
      year: date.getFullYear() !== now.getFullYear() ? 'numeric' : undefined,
    });
  }

  function formatTimeFriendly(timeStr: string | null, isAllDay: boolean, endTimeStr?: string | null): string {
    if (isAllDay || !timeStr) return 'All day';
    const [hh, mm] = timeStr.split(':').map(Number);
    const d = new Date();
    d.setHours(hh, mm, 0, 0);
    const startFormatted = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    if (endTimeStr) {
      const [eh, em] = endTimeStr.split(':').map(Number);
      const e = new Date();
      e.setHours(eh, em, 0, 0);
      const endFormatted = e.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      return `${startFormatted} – ${endFormatted}`;
    }
    return startFormatted;
  }

  function getCalendarName(ev: SubjectEvent): string {
    if (ev.calendar_type === 'google') {
      const match = calendars.find(c => c.id === ev.google_calendar_id);
      return match ? match.summary : 'Google Calendar';
    }
    return 'Local';
  }
</script>

<div class="view events-view">
  <!-- Top Stat / KPI Cards (Consistent with DailyView, WeeklyView, HistoryView) -->
  <div class="cards">
    <div class="card" style="--delay: 0ms">
      <span class="card-label">Next Deadline</span>
      <span class="card-value next-val">
        {nextMilestoneCountdown ? nextMilestoneCountdown.formatted : '—'}
      </span>
      <span class="card-subtext">
        {nextMilestone ? `${nextMilestone.subject}: ${nextMilestone.name}` : 'All caught up'}
      </span>
    </div>
    <div class="card-divider"></div>
    <div class="card" style="--delay: 60ms">
      <span class="card-label">Upcoming</span>
      <span class="card-value">{counts.upcoming}</span>
      <span class="card-subtext">Active deadlines</span>
    </div>
    <div class="card-divider"></div>
    <div class="card" style="--delay: 120ms">
      <span class="card-label">Overdue</span>
      <span class="card-value" class:has-overdue={counts.overdue > 0}>{counts.overdue}</span>
      <span class="card-subtext">Needs attention</span>
    </div>
    <div class="card-divider"></div>
    <div class="card" style="--delay: 180ms">
      <span class="card-label">Completed</span>
      <span class="card-value">{counts.completed}</span>
      <span class="card-subtext">Done & submitted</span>
    </div>
  </div>

  <!-- Controls Bar (Consistent with HistoryView) -->
  <div class="controls">
    <div class="filter-row">
      <DropdownSelect bind:value={filterSubject} options={allSubjectsList} placeholder="All Subjects" />
      <DropdownSelect bind:value={filterType} options={TYPE_OPTIONS} placeholder="All Types" />

      <!-- Search Input -->
      <div class="search-wrap">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="search-icon">
          <circle cx="11" cy="11" r="8"></circle>
          <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
        </svg>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Filter by title..."
          class="search-input"
        />
        {#if searchQuery}
          <button class="btn-clear" onclick={() => searchQuery = ''}>✕</button>
        {/if}
      </div>

      <!-- Add Event Button -->
      <button class="btn btn-add-entry" onclick={handleOpenCreate} title="Add Academic Event">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12h14"/>
          <path d="M12 5v14"/>
        </svg>
        <span>Add Event</span>
      </button>
    </div>

    <!-- Status Tabs (Quick modes matching HistoryView) -->
    <div class="filter-row status-controls">
      <div class="quick-modes">
        <button
          class="btn btn-small"
          class:active={statusFilter === 'upcoming'}
          onclick={() => statusFilter = 'upcoming'}
        >
          Upcoming <span class="tab-badge">{counts.upcoming}</span>
        </button>
        <button
          class="btn btn-small"
          class:active={statusFilter === 'overdue'}
          onclick={() => statusFilter = 'overdue'}
        >
          Overdue <span class="tab-badge" class:overdue-badge={counts.overdue > 0}>{counts.overdue}</span>
        </button>
        <button
          class="btn btn-small"
          class:active={statusFilter === 'completed'}
          onclick={() => statusFilter = 'completed'}
        >
          Completed <span class="tab-badge">{counts.completed}</span>
        </button>
        <button
          class="btn btn-small"
          class:active={statusFilter === 'all'}
          onclick={() => statusFilter = 'all'}
        >
          All <span class="tab-badge">{counts.all}</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Events List Table (Consistent with SubjectsView and HistoryView) -->
  <div class="list-container">
    {#if loading}
      <div class="msg">Loading academic events...</div>
    {:else if filteredEvents.length === 0}
      <div class="msg">
        {#if searchQuery || filterSubject || filterType}
          No events match the selected filters.
        {:else if statusFilter === 'overdue'}
          No overdue events. Everything is on schedule!
        {:else if statusFilter === 'completed'}
          No completed events yet.
        {:else}
          No upcoming events. Use "+ Add Event" to track exams and assignments.
        {/if}
      </div>
    {:else}
      <table class="events-table">
        <thead>
          <tr>
            <th class="col-status"></th>
            <th class="col-subject">Subject</th>
            <th class="col-title">Event Title</th>
            <th class="col-type">Type</th>
            <th class="col-due">Due Date</th>
            <th class="col-countdown">Countdown</th>
            <th class="col-cal">Calendar</th>
            <th class="col-actions">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredEvents as ev (ev.id)}
            {@const timing = getCountdownInfo(ev)}
            {@const typeConf = TYPE_CONFIG[ev.event_type] || TYPE_CONFIG.other}
            {@const subjCol = getSubjectColor(ev.subject)}
            {@const subjFg = getContrastColor(subjCol)}
            <tr
              class="event-row"
              class:is-completed={ev.is_completed}
              class:is-overdue={timing.urgency === 'overdue'}
              onclick={() => handleEditEvent(ev)}
            >
              <!-- Checkbox Toggle -->
              <td class="col-status">
                <button
                  type="button"
                  class="btn-status-toggle"
                  class:checked={ev.is_completed}
                  onclick={(e) => handleToggleCompleted(ev, e)}
                  title={ev.is_completed ? 'Mark incomplete' : 'Mark completed'}
                >
                  {#if ev.is_completed}
                    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                      <polyline points="20 6 9 17 4 12"></polyline>
                    </svg>
                  {/if}
                </button>
              </td>

              <!-- Subject Pill -->
              <td class="col-subject">
                <span class="subject-pill" style="background-color: {subjCol}; color: {subjFg};">
                  {ev.subject}
                </span>
              </td>

              <!-- Event Title & Notes Preview -->
              <td class="col-title">
                <div class="title-cell">
                  <span class="event-name" class:completed-text={ev.is_completed}>
                    {ev.name}
                  </span>
                  {#if ev.notes}
                    <span class="event-notes-sub">{ev.notes}</span>
                  {/if}
                </div>
              </td>

              <!-- Event Type Tag -->
              <td class="col-type">
                <span class="type-tag" style="border-color: {typeConf.color}; color: {typeConf.color};">
                  <span class="type-icon">{typeConf.icon}</span>
                  <span>{typeConf.label}</span>
                </span>
              </td>

              <!-- Due Date & Time -->
              <td class="col-due">
                <div class="due-cell">
                  <span class="due-date">{formatDateFriendly(ev.event_date)}</span>
                  <span class="due-time">{formatTimeFriendly(ev.event_time, ev.is_all_day, ev.end_time)}</span>
                </div>
              </td>

              <!-- Live Countdown Badge -->
              <td class="col-countdown">
                <span
                  class="countdown-badge"
                  class:urgency-overdue={timing.urgency === 'overdue'}
                  class:urgency-urgent={timing.urgency === 'urgent'}
                  class:urgency-soon={timing.urgency === 'soon'}
                  class:urgency-upcoming={timing.urgency === 'upcoming'}
                  class:urgency-completed={timing.urgency === 'completed'}
                >
                  {timing.formatted}
                </span>
              </td>

              <!-- Calendar Destination -->
              <td class="col-cal">
                <span class="cal-badge" class:is-google={ev.calendar_type === 'google'} title={getCalendarName(ev)}>
                  {ev.calendar_type === 'google' ? 'Google' : 'Local'}
                </span>
              </td>

              <!-- Actions Column -->
              <td class="col-actions">
                <div class="row-actions">
                  <button
                    class="btn-icon-action"
                    onclick={(e) => { e.stopPropagation(); handleEditEvent(ev); }}
                    title="Edit event"
                  >
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"></path>
                    </svg>
                  </button>
                  <button
                    class="btn-icon-action btn-delete-action"
                    onclick={(e) => handleDeleteEvent(ev, e)}
                    title="Delete event"
                  >
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <polyline points="3 6 5 6 21 6"></polyline>
                      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                    </svg>
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

{#if showModal}
  <EventModal
    event={selectedEventForEdit}
    onClose={() => showModal = false}
    onSaved={handleEventSaved}
    onDeleted={handleEventDeleted}
  />
{/if}

<style>
  .events-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
    box-sizing: border-box;
    color: var(--color-foreground);
    background: transparent;
  }

  /* ── Stat KPI Cards (100% matched to DailyView / WeeklyView) ── */
  .cards {
    display: flex;
    align-items: stretch;
    border-bottom: 1px solid var(--color-separator);
    flex-shrink: 0;
  }

  .card {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 16px 20px;
    animation: card-rise 0.35s cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--delay, 0ms);
  }

  @keyframes card-rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .card-label {
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-foreground-darker);
  }

  .card-value {
    font-size: 1.85rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
    color: var(--color-foreground);
    line-height: 1.1;
  }

  .card-value.next-val {
    font-size: 1.4rem;
    font-family: monospace;
    color: var(--color-focus-round);
  }

  .card-value.has-overdue {
    color: #ef4444;
  }

  .card-subtext {
    font-size: 0.75rem;
    color: var(--color-foreground-darker);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
  }

  .card-divider {
    width: 1px;
    background: var(--color-separator);
    align-self: stretch;
    margin: 12px 0;
  }

  /* ── Controls & Filter Bar (100% matched to HistoryView) ── */
  .controls {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px 0 16px;
    flex-shrink: 0;
  }

  .filter-row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 8px;
    color: var(--color-foreground-darker);
    pointer-events: none;
  }

  .search-input {
    background: transparent;
    border: 1px solid var(--color-separator);
    color: var(--color-foreground);
    padding: 6px 26px 6px 28px;
    border-radius: 4px;
    font-size: 0.85rem;
    width: 140px;
    transition: width 0.2s, border-color 0.2s;
    outline: none;
  }

  .search-input:focus {
    width: 180px;
    border-color: var(--color-focus-round);
  }

  .btn-clear {
    position: absolute;
    right: 6px;
    background: none;
    border: none;
    color: var(--color-foreground-darker);
    cursor: pointer;
    font-size: 0.75rem;
  }

  .btn-add-entry {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--color-focus-round, var(--color-accent));
    color: var(--color-background);
    border: none;
    border-radius: 4px;
    padding: 6px 14px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: filter 0.15s ease;
    margin-left: auto;
  }

  .btn-add-entry:hover {
    filter: brightness(1.1);
  }

  .quick-modes {
    display: flex;
    gap: 4px;
    background: var(--color-background);
    padding: 3px;
    border-radius: 6px;
    border: 1px solid var(--color-separator);
  }

  .btn-small {
    padding: 4px 10px;
    font-size: 0.8rem;
    border: none;
    background: transparent;
    color: var(--color-foreground-darker);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-small:hover {
    color: var(--color-foreground);
    background: var(--color-hover);
  }

  .btn-small.active {
    background: var(--color-focus-round);
    color: var(--color-background);
    font-weight: 600;
  }

  .tab-badge {
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
    padding: 1px 5px;
    border-radius: 8px;
    font-size: 0.7rem;
    font-weight: 600;
  }

  .btn-small.active .tab-badge {
    background: rgba(0, 0, 0, 0.25);
    color: var(--color-background);
  }

  .tab-badge.overdue-badge {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  /* ── Events Table (100% matched to SubjectsView & HistoryView) ── */
  .list-container {
    flex: 1;
    overflow-y: auto;
    background: transparent;
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    margin: 14px 16px 16px 16px;
  }

  .msg {
    padding: 36px 16px;
    text-align: center;
    color: var(--color-foreground-darker);
    font-size: 0.9rem;
  }

  .events-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  th, td {
    padding: 0.65rem 0.85rem;
    text-align: left;
    border-bottom: 1px solid var(--color-separator);
    vertical-align: middle;
  }

  th {
    font-weight: 600;
    color: var(--color-foreground-darker);
    background: var(--color-background);
    position: sticky;
    top: 0;
    z-index: 2;
    font-size: 0.78rem;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .event-row {
    transition: background 0.15s;
    cursor: pointer;
  }

  .event-row:hover {
    background: var(--color-hover);
  }

  .event-row.is-completed {
    opacity: 0.6;
  }

  .event-row.is-overdue {
    border-left: 3px solid #ef4444;
  }

  /* Columns */
  .col-status {
    width: 32px;
    text-align: center;
  }

  .btn-status-toggle {
    width: 18px;
    height: 18px;
    border: 1px solid var(--color-foreground-darker);
    border-radius: 4px;
    background: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-background);
    padding: 0;
    transition: all 0.15s;
  }

  .btn-status-toggle.checked {
    background: var(--color-focus-round);
    border-color: var(--color-focus-round);
  }

  .col-subject {
    width: 14%;
  }

  .subject-pill {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 0.74rem;
    font-weight: 600;
    max-width: 110px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-title {
    width: 30%;
  }

  .title-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .event-name {
    font-weight: 600;
    color: var(--color-foreground);
    line-height: 1.25;
  }

  .event-name.completed-text {
    text-decoration: line-through;
    color: var(--color-foreground-darker);
  }

  .event-notes-sub {
    font-size: 0.74rem;
    color: var(--color-foreground-darker);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-type {
    width: 12%;
  }

  .type-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 1px solid currentColor;
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 0.72rem;
    font-weight: 500;
  }

  .type-icon {
    font-size: 0.75rem;
  }

  .col-due {
    width: 16%;
  }

  .due-cell {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .due-date {
    color: var(--color-foreground);
    font-weight: 500;
  }

  .due-time {
    font-size: 0.74rem;
    color: var(--color-foreground-darker);
  }

  .col-countdown {
    width: 16%;
  }

  .countdown-badge {
    display: inline-block;
    font-family: monospace;
    font-size: 0.76rem;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 12px;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .urgency-overdue {
    background: rgba(239, 68, 68, 0.16);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.35);
  }

  .urgency-urgent {
    background: rgba(249, 115, 22, 0.16);
    color: #f97316;
    border: 1px solid rgba(249, 115, 22, 0.35);
  }

  .urgency-soon {
    background: rgba(245, 158, 11, 0.16);
    color: #f59e0b;
    border: 1px solid rgba(245, 158, 11, 0.35);
  }

  .urgency-upcoming {
    background: rgba(16, 185, 129, 0.16);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.35);
  }

  .urgency-completed {
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
    color: var(--color-foreground-darker);
  }

  .col-cal {
    width: 70px;
  }

  .cal-badge {
    font-size: 0.72rem;
    color: var(--color-foreground-darker);
    background: color-mix(in oklch, var(--color-foreground) 6%, transparent);
    padding: 2px 6px;
    border-radius: 4px;
    display: inline-block;
  }

  .cal-badge.is-google {
    color: #60a5fa;
  }

  .col-actions {
    width: 60px;
    text-align: right;
  }

  .row-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
  }

  .btn-icon-action {
    background: transparent;
    border: none;
    color: var(--color-foreground-darker);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }

  .btn-icon-action:hover {
    color: var(--color-foreground);
    background: var(--color-hover);
  }

  .btn-delete-action:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.15);
  }
</style>
