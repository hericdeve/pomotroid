<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    subjectEventsGetAll,
    subjectEventToggleCompleted,
    subjectsGetAll,
    googleCalendarGetCalendars,
  } from '$lib/ipc';
  import type {
    SubjectEvent,
    SubjectEventType,
    SubjectStats,
    GoogleCalendarItem,
  } from '$lib/types';
  import { getSubjectColor, getContrastColor } from '$lib/utils/subjectColors';
  import EventModal from './EventModal.svelte';
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
  let subjectFilter = $state<string>('all');
  let typeFilter = $state<string>('all');
  let searchQuery = $state<string>('');

  let showModal = $state(false);
  let selectedEventForEdit = $state<SubjectEvent | null>(null);

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

  // Calculate target Date object from event_date + event_time
  function getEventTargetDate(ev: SubjectEvent): Date {
    if (ev.is_all_day || !ev.event_time) {
      // All day: due at 23:59:59 of that date
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
      urgency = 'urgent'; // < 24 hours
    } else if (days <= 2) {
      urgency = 'soon'; // < 3 days
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

  // Next upcoming milestone for Hero Banner
  let nextMilestone = $derived.by(() => {
    const upcomingEvents = events
      .filter(e => !e.is_completed && getEventTargetDate(e).getTime() >= now.getTime())
      .sort((a, b) => getEventTargetDate(a).getTime() - getEventTargetDate(b).getTime());
    return upcomingEvents[0] || null;
  });

  let nextMilestoneCountdown = $derived(nextMilestone ? getCountdownInfo(nextMilestone) : null);

  // Counts for filter tabs
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

  // Filtered and sorted events list
  let filteredEvents = $derived.by(() => {
    return events
      .filter(ev => {
        const isPast = getEventTargetDate(ev).getTime() < now.getTime();
        if (statusFilter === 'upcoming' && (ev.is_completed || isPast)) return false;
        if (statusFilter === 'overdue' && (ev.is_completed || !isPast)) return false;
        if (statusFilter === 'completed' && !ev.is_completed) return false;

        if (subjectFilter !== 'all' && ev.subject !== subjectFilter) return false;
        if (typeFilter !== 'all' && ev.event_type !== typeFilter) return false;

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
        // If viewing completed, sort latest completed first
        if (statusFilter === 'completed') {
          return getEventTargetDate(b).getTime() - getEventTargetDate(a).getTime();
        }
        // Otherwise sort chronologically
        return getEventTargetDate(a).getTime() - getEventTargetDate(b).getTime();
      });
  });

  // Distinct list of subjects that have events or exist in DB
  let allSubjectsList = $derived.by(() => {
    const set = new Set<string>();
    subjects.forEach(s => set.add(s.name));
    events.forEach(e => set.add(e.subject));
    return Array.from(set).sort();
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

  function formatTimeFriendly(timeStr: string | null, isAllDay: boolean): string {
    if (isAllDay || !timeStr) return 'All day';
    const [hh, mm] = timeStr.split(':').map(Number);
    const d = new Date();
    d.setHours(hh, mm, 0, 0);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function getCalendarName(ev: SubjectEvent): string {
    if (ev.calendar_type === 'google') {
      const match = calendars.find(c => c.id === ev.google_calendar_id);
      return match ? match.summary : 'Google Calendar';
    }
    return 'Local';
  }
</script>

<div class="events-view">
  <!-- Top Bar: Header & Add Button -->
  <header class="view-header">
    <div class="header-titles">
      <h1 class="view-title">Academic Events & Deadlines</h1>
      <p class="view-subtitle">Track upcoming exams, assignments, projects, and milestone countdowns.</p>
    </div>
    <button class="btn-create-event" onclick={handleOpenCreate}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <line x1="12" y1="5" x2="12" y2="19"></line>
        <line x1="5" y1="12" x2="19" y2="12"></line>
      </svg>
      <span>New Event</span>
    </button>
  </header>

  <!-- Hero: Next Milestone Countdown Banner -->
  {#if nextMilestone && nextMilestoneCountdown}
    {@const typeConf = TYPE_CONFIG[nextMilestone.event_type] || TYPE_CONFIG.other}
    {@const subjCol = getSubjectColor(nextMilestone.subject)}
    {@const subjFg = getContrastColor(subjCol)}
    <div class="hero-milestone-card" onclick={() => handleEditEvent(nextMilestone!)} role="button" tabindex="0" onkeydown={(e) => { if (e.key === 'Enter') handleEditEvent(nextMilestone!); }}>
      <div class="hero-left">
        <div class="hero-badge-row">
          <span class="hero-type-chip" style="border-color: {typeConf.color}; color: {typeConf.color};">
            <span>{typeConf.icon}</span>
            <span>{typeConf.label}</span>
          </span>
          <span class="hero-subject-chip" style="background-color: {subjCol}; color: {subjFg};">
            {nextMilestone.subject}
          </span>
          <span class="hero-due-badge">
            📅 {formatDateFriendly(nextMilestone.event_date)} • {formatTimeFriendly(nextMilestone.event_time, nextMilestone.is_all_day)}
          </span>
        </div>
        <h2 class="hero-event-title">{nextMilestone.name}</h2>
        {#if nextMilestone.notes}
          <p class="hero-notes">{nextMilestone.notes}</p>
        {/if}
      </div>

      <!-- Real-time Big Countdown Display -->
      <div class="hero-countdown-box">
        <span class="countdown-label">TIME REMAINING</span>
        <div class="countdown-units">
          <div class="unit-box">
            <span class="unit-val">{nextMilestoneCountdown.days}</span>
            <span class="unit-name">DAYS</span>
          </div>
          <span class="unit-sep">:</span>
          <div class="unit-box">
            <span class="unit-val">{String(nextMilestoneCountdown.hours).padStart(2, '0')}</span>
            <span class="unit-name">HOURS</span>
          </div>
          <span class="unit-sep">:</span>
          <div class="unit-box">
            <span class="unit-val">{String(nextMilestoneCountdown.minutes).padStart(2, '0')}</span>
            <span class="unit-name">MINS</span>
          </div>
          <span class="unit-sep">:</span>
          <div class="unit-box">
            <span class="unit-val highlight">{String(nextMilestoneCountdown.seconds).padStart(2, '0')}</span>
            <span class="unit-name">SECS</span>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Filters & Controls Bar -->
  <div class="controls-bar">
    <!-- Status Tabs -->
    <div class="status-tabs">
      <button
        class="status-tab"
        class:active={statusFilter === 'upcoming'}
        onclick={() => statusFilter = 'upcoming'}
      >
        Upcoming <span class="tab-count">{counts.upcoming}</span>
      </button>
      <button
        class="status-tab"
        class:active={statusFilter === 'overdue'}
        onclick={() => statusFilter = 'overdue'}
      >
        Overdue <span class="tab-count" class:has-overdue={counts.overdue > 0}>{counts.overdue}</span>
      </button>
      <button
        class="status-tab"
        class:active={statusFilter === 'completed'}
        onclick={() => statusFilter = 'completed'}
      >
        Completed <span class="tab-count">{counts.completed}</span>
      </button>
      <button
        class="status-tab"
        class:active={statusFilter === 'all'}
        onclick={() => statusFilter = 'all'}
      >
        All <span class="tab-count">{counts.all}</span>
      </button>
    </div>

    <!-- Dropdowns & Search -->
    <div class="filter-actions">
      <!-- Subject Filter -->
      <select bind:value={subjectFilter} class="filter-select">
        <option value="all">All Subjects</option>
        {#each allSubjectsList as subj}
          <option value={subj}>{subj}</option>
        {/each}
      </select>

      <!-- Event Type Filter -->
      <select bind:value={typeFilter} class="filter-select">
        <option value="all">All Types</option>
        <option value="exam">Exams</option>
        <option value="assignment">Assignments</option>
        <option value="project">Projects</option>
        <option value="quiz">Quizzes</option>
        <option value="other">Other</option>
      </select>

      <!-- Search Input -->
      <div class="search-box">
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
          <button class="btn-clear-search" onclick={() => searchQuery = ''}>✕</button>
        {/if}
      </div>
    </div>
  </div>

  <!-- Events List Area -->
  <div class="events-container">
    {#if loading}
      <div class="empty-state">
        <div class="spinner"></div>
        <p>Loading academic events...</p>
      </div>
    {:else if filteredEvents.length === 0}
      <div class="empty-state">
        <div class="empty-icon">📅</div>
        <h3>No events found</h3>
        <p>
          {#if searchQuery || subjectFilter !== 'all' || typeFilter !== 'all'}
            No events match your current filter criteria.
          {:else if statusFilter === 'overdue'}
            Great job! You have no overdue events.
          {:else if statusFilter === 'completed'}
            You haven't completed any events yet.
          {:else}
            No upcoming events yet. Add assignments or exams to start counting down!
          {/if}
        </p>
        {#if statusFilter === 'upcoming' && !searchQuery && subjectFilter === 'all' && typeFilter === 'all'}
          <button class="btn-create-event" onclick={handleOpenCreate}>
            + Add First Event
          </button>
        {/if}
      </div>
    {:else}
      <div class="events-grid">
        {#each filteredEvents as ev (ev.id)}
          {@const timing = getCountdownInfo(ev)}
          {@const typeConf = TYPE_CONFIG[ev.event_type] || TYPE_CONFIG.other}
          {@const subjCol = getSubjectColor(ev.subject)}
          {@const subjFg = getContrastColor(subjCol)}
          <div
            class="event-card"
            class:completed={ev.is_completed}
            class:is-overdue={timing.urgency === 'overdue'}
            class:is-urgent={timing.urgency === 'urgent'}
            onclick={() => handleEditEvent(ev)}
            role="button"
            tabindex="0"
            onkeydown={(e) => { if (e.key === 'Enter') handleEditEvent(ev); }}
          >
            <!-- Card Header: Checkbox + Subject + Type -->
            <div class="card-top-row">
              <!-- Checkbox -->
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="checkbox-wrapper"
                onclick={(e) => handleToggleCompleted(ev, e)}
                title={ev.is_completed ? 'Mark incomplete' : 'Mark completed'}
              >
                <input
                  type="checkbox"
                  checked={ev.is_completed}
                  onclick={(e) => handleToggleCompleted(ev, e)}
                />
              </div>

              <!-- Subject Tag -->
              <span class="subject-badge" style="background-color: {subjCol}; color: {subjFg};">
                {ev.subject}
              </span>

              <!-- Type Badge -->
              <span class="type-badge" style="border-color: {typeConf.color}; color: {typeConf.color};">
                <span>{typeConf.icon}</span>
                <span>{typeConf.label}</span>
              </span>

              <!-- Calendar Destination Badge -->
              <span class="cal-badge" class:google={ev.calendar_type === 'google'} title="Synced destination">
                {#if ev.calendar_type === 'google'}
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10"/>
                  </svg>
                  <span>{getCalendarName(ev)}</span>
                {:else}
                  <span>Local</span>
                {/if}
              </span>
            </div>

            <!-- Card Title -->
            <h3 class="event-card-title" class:completed-title={ev.is_completed}>
              {ev.name}
            </h3>

            <!-- Date & Time Info -->
            <div class="date-time-row">
              <span class="date-info">
                📅 {formatDateFriendly(ev.event_date)}
              </span>
              <span class="time-info">
                🕒 {formatTimeFriendly(ev.event_time, ev.is_all_day)}
              </span>
            </div>

            <!-- Countdown Pill / Urgency Indicator -->
            <div class="countdown-pill-row">
              <div class="countdown-pill" class:urgency-overdue={timing.urgency === 'overdue'} class:urgency-urgent={timing.urgency === 'urgent'} class:urgency-soon={timing.urgency === 'soon'} class:urgency-upcoming={timing.urgency === 'upcoming'} class:urgency-completed={timing.urgency === 'completed'}>
                {#if timing.urgency === 'overdue'}
                  <span class="pill-icon">⚠️</span>
                {:else if timing.urgency === 'urgent'}
                  <span class="pill-icon">🔥</span>
                {:else if timing.urgency === 'soon'}
                  <span class="pill-icon">⏳</span>
                {:else if timing.urgency === 'completed'}
                  <span class="pill-icon">✓</span>
                {:else}
                  <span class="pill-icon">⏱️</span>
                {/if}
                <span class="pill-text">{timing.formatted}</span>
              </div>
            </div>

            <!-- Notes preview if available -->
            {#if ev.notes}
              <p class="event-notes-preview">{ev.notes}</p>
            {/if}

            <!-- Card Actions -->
            <div class="card-footer">
              <span class="edit-hint">Click to edit</span>
              <button
                class="btn-edit-action"
                onclick={(e) => {
                  e.stopPropagation();
                  handleEditEvent(ev);
                }}
                title="Edit event"
              >
                ✏️ Edit
              </button>
            </div>
          </div>
        {/each}
      </div>
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
    padding: 1.25rem 1.5rem;
    overflow-y: auto;
    box-sizing: border-box;
    color: var(--color-foreground);
    background: transparent;
    gap: 1.25rem;
  }

  /* Header */
  .view-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .view-title {
    font-size: 1.35rem;
    font-weight: 700;
    margin: 0 0 4px 0;
    color: var(--color-foreground);
  }

  .view-subtitle {
    margin: 0;
    font-size: 0.85rem;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .btn-create-event {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--color-focus-round);
    color: var(--color-background);
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.86rem;
    font-weight: 600;
    cursor: pointer;
    transition: filter 0.15s, transform 0.1s;
    flex-shrink: 0;
  }

  .btn-create-event:hover {
    filter: brightness(1.1);
  }

  .btn-create-event:active {
    transform: scale(0.98);
  }

  /* Hero Milestone Card */
  .hero-milestone-card {
    background: color-mix(in oklch, var(--color-foreground) 4%, transparent);
    border: 1px solid var(--color-separator);
    border-radius: 8px;
    padding: 1.2rem 1.4rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1.5rem;
    cursor: pointer;
    transition: border-color 0.2s, background-color 0.2s, transform 0.1s;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.08);
  }

  .hero-milestone-card:hover {
    border-color: var(--color-accent);
    background: color-mix(in oklch, var(--color-foreground) 6%, transparent);
  }

  .hero-left {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .hero-badge-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .hero-type-chip {
    display: flex;
    align-items: center;
    gap: 4px;
    border: 1px solid currentColor;
    border-radius: 4px;
    padding: 2px 7px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .hero-subject-chip {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .hero-due-badge {
    font-size: 0.78rem;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .hero-event-title {
    margin: 0;
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--color-foreground);
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hero-notes {
    margin: 0;
    font-size: 0.82rem;
    color: var(--color-foreground-darker, #a1a1aa);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Big Real-Time Countdown Box */
  .hero-countdown-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    background: color-mix(in oklch, var(--color-foreground) 6%, transparent);
    border: 1px solid var(--color-separator);
    border-radius: 8px;
    padding: 10px 16px;
    flex-shrink: 0;
  }

  .countdown-label {
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-foreground-darker, #a1a1aa);
    margin-bottom: 6px;
  }

  .countdown-units {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .unit-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 38px;
  }

  .unit-val {
    font-size: 1.35rem;
    font-weight: 800;
    font-family: monospace;
    line-height: 1;
    color: var(--color-foreground);
  }

  .unit-val.highlight {
    color: var(--color-focus-round);
  }

  .unit-name {
    font-size: 0.6rem;
    font-weight: 600;
    color: var(--color-foreground-darker, #a1a1aa);
    margin-top: 2px;
  }

  .unit-sep {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--color-separator);
    margin-bottom: 8px;
  }

  /* Controls & Filter Bar */
  .controls-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--color-separator);
  }

  .status-tabs {
    display: flex;
    gap: 4px;
    background: color-mix(in oklch, var(--color-foreground) 4%, transparent);
    padding: 3px;
    border-radius: 6px;
    border: 1px solid var(--color-separator);
  }

  .status-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    color: var(--color-foreground-darker, #a1a1aa);
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 0.82rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }

  .status-tab:hover {
    color: var(--color-foreground);
  }

  .status-tab.active {
    background: var(--color-focus-round);
    color: var(--color-background);
    font-weight: 600;
  }

  .tab-count {
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
    padding: 1px 6px;
    border-radius: 10px;
    font-size: 0.72rem;
    font-weight: 600;
  }

  .status-tab.active .tab-count {
    background: rgba(0, 0, 0, 0.2);
    color: var(--color-background);
  }

  .tab-count.has-overdue {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .filter-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .filter-select {
    padding: 6px 10px;
    background: color-mix(in oklch, var(--color-foreground) 6%, transparent);
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    color: var(--color-foreground);
    font-size: 0.8rem;
    outline: none;
    cursor: pointer;
  }

  .filter-select option {
    background: var(--color-background);
    color: var(--color-foreground);
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 8px;
    color: var(--color-foreground-darker, #a1a1aa);
    pointer-events: none;
  }

  .search-input {
    padding: 6px 26px 6px 28px;
    background: color-mix(in oklch, var(--color-foreground) 6%, transparent);
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    color: var(--color-foreground);
    font-size: 0.8rem;
    outline: none;
    width: 140px;
    transition: width 0.2s, border-color 0.2s;
  }

  .search-input:focus {
    width: 190px;
    border-color: var(--color-accent);
  }

  .btn-clear-search {
    position: absolute;
    right: 6px;
    background: none;
    border: none;
    color: var(--color-foreground-darker, #a1a1aa);
    cursor: pointer;
    font-size: 0.75rem;
    padding: 2px 4px;
  }

  /* Grid Area */
  .events-container {
    flex: 1;
  }

  .events-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1rem;
    padding-bottom: 2rem;
  }

  /* Event Card */
  .event-card {
    background: color-mix(in oklch, var(--color-foreground) 3%, transparent);
    border: 1px solid var(--color-separator);
    border-radius: 8px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    cursor: pointer;
    transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s;
    position: relative;
    box-sizing: border-box;
  }

  .event-card:hover {
    border-color: var(--color-accent);
    transform: translateY(-2px);
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.12);
  }

  .event-card.completed {
    opacity: 0.65;
    background: color-mix(in oklch, var(--color-foreground) 2%, transparent);
  }

  .event-card.is-overdue {
    border-left: 3px solid #ef4444;
  }

  .event-card.is-urgent {
    border-left: 3px solid #f97316;
  }

  .card-top-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .checkbox-wrapper {
    display: flex;
    align-items: center;
    cursor: pointer;
  }

  .checkbox-wrapper input {
    cursor: pointer;
    width: 15px;
    height: 15px;
    accent-color: var(--color-focus-round);
  }

  .subject-badge {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 600;
    max-width: 130px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .type-badge {
    display: flex;
    align-items: center;
    gap: 3px;
    border: 1px solid currentColor;
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 0.7rem;
    font-weight: 500;
  }

  .cal-badge {
    margin-left: auto;
    font-size: 0.68rem;
    color: var(--color-foreground-darker, #a1a1aa);
    display: flex;
    align-items: center;
    gap: 3px;
    background: color-mix(in oklch, var(--color-foreground) 5%, transparent);
    padding: 2px 5px;
    border-radius: 4px;
  }

  .cal-badge.google {
    color: #60a5fa;
  }

  .event-card-title {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
    line-height: 1.25;
    color: var(--color-foreground);
  }

  .completed-title {
    text-decoration: line-through;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .date-time-row {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 0.78rem;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  /* Countdown Pill */
  .countdown-pill-row {
    display: flex;
    align-items: center;
  }

  .countdown-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 9px;
    border-radius: 12px;
    font-size: 0.76rem;
    font-weight: 600;
    font-family: monospace;
    letter-spacing: 0.02em;
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
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .event-notes-preview {
    margin: 0;
    font-size: 0.78rem;
    color: var(--color-foreground-darker, #a1a1aa);
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: auto;
    padding-top: 6px;
    border-top: 1px solid var(--color-separator);
  }

  .edit-hint {
    font-size: 0.7rem;
    color: var(--color-foreground-darker, #a1a1aa);
    opacity: 0.6;
  }

  .btn-edit-action {
    background: transparent;
    border: none;
    color: var(--color-foreground-darker, #a1a1aa);
    font-size: 0.72rem;
    cursor: pointer;
    padding: 3px 6px;
    border-radius: 4px;
    transition: all 0.15s;
  }

  .btn-edit-action:hover {
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
    color: var(--color-foreground);
  }

  /* Empty State */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 1rem;
    color: var(--color-foreground-darker, #a1a1aa);
    text-align: center;
    gap: 10px;
  }

  .empty-icon {
    font-size: 2.5rem;
  }

  .empty-state h3 {
    margin: 0;
    font-size: 1.15rem;
    color: var(--color-foreground);
  }

  .empty-state p {
    margin: 0;
    font-size: 0.85rem;
    max-width: 380px;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--color-separator);
    border-top-color: var(--color-focus-round);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
