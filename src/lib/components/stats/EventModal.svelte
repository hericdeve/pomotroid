<script lang="ts">
  import { onMount } from 'svelte';
  import {
    subjectsGetAll,
    googleCalendarGetStatus,
    googleCalendarGetCalendars,
    subjectEventCreate,
    subjectEventUpdate,
    subjectEventDelete,
  } from '$lib/ipc';
  import type {
    SubjectEvent,
    SubjectEventType,
    SubjectStats,
    GoogleCalendarItem,
    GoogleAuthStatus,
    CreateSubjectEventPayload,
    UpdateSubjectEventPayload,
  } from '$lib/types';
  import { getSubjectColor, getContrastColor } from '$lib/utils/subjectColors';
  import { error as logError } from '@tauri-apps/plugin-log';

  interface Props {
    event?: SubjectEvent | null;
    initialDate?: string;
    initialSubject?: string;
    onClose: () => void;
    onSaved: (event: SubjectEvent) => void;
    onDeleted?: (id: number) => void;
  }

  let {
    event = null,
    initialDate,
    initialSubject,
    onClose,
    onSaved,
    onDeleted,
  }: Props = $props();

  const isEditMode = $derived(!!event);

  let subjects = $state<SubjectStats[]>([]);
  let calendars = $state<GoogleCalendarItem[]>([]);
  let isGoogleSignedIn = $state(false);

  let name = $state('');
  let subject = $state('');
  let eventType = $state<SubjectEventType>('exam');
  let eventDate = $state('');
  let isAllDay = $state(false);
  let eventTime = $state('09:00');
  let calendarType = $state<'local' | 'google'>('local');
  let googleCalendarId = $state<string | null>(null);
  let notes = $state('');
  let isCompleted = $state(false);

  let isSaving = $state(false);
  let isDeleting = $state(false);
  let errorMessage = $state<string | null>(null);

  const EVENT_TYPES: { id: SubjectEventType; label: string; icon: string }[] = [
    { id: 'exam', label: 'Exam', icon: '📝' },
    { id: 'assignment', label: 'Assignment', icon: '📋' },
    { id: 'project', label: 'Project', icon: '🚀' },
    { id: 'quiz', label: 'Quiz', icon: '💡' },
    { id: 'other', label: 'Other', icon: '📌' },
  ];

  function getTodayYmd(): string {
    const d = new Date();
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  onMount(async () => {
    try {
      const [subjs, gStatus] = await Promise.all([
        subjectsGetAll().catch(() => []),
        googleCalendarGetStatus().catch(() => ({ is_signed_in: false })),
      ]);
      subjects = subjs;
      isGoogleSignedIn = (gStatus as GoogleAuthStatus).is_signed_in;

      if (isGoogleSignedIn) {
        const cals = await googleCalendarGetCalendars().catch(() => []);
        calendars = cals;
      }
    } catch (e) {
      logError(`Failed to load data for EventModal: ${e}`);
    }

    if (event) {
      name = event.name;
      subject = event.subject;
      eventType = (event.event_type as SubjectEventType) || 'exam';
      eventDate = event.event_date;
      isAllDay = event.is_all_day;
      eventTime = event.event_time || '09:00';
      calendarType = event.calendar_type === 'google' ? 'google' : 'local';
      googleCalendarId = event.google_calendar_id;
      notes = event.notes || '';
      isCompleted = event.is_completed;
    } else {
      name = '';
      subject = initialSubject || (subjects.length > 0 ? subjects[0].name : '');
      eventType = 'assignment';
      eventDate = initialDate || getTodayYmd();
      isAllDay = false;
      eventTime = '09:00';
      calendarType = 'local';
      googleCalendarId = calendars.find(c => c.primary)?.id || calendars[0]?.id || null;
      notes = '';
      isCompleted = false;
    }
  });

  async function handleSave() {
    errorMessage = null;
    const trimmedName = name.trim();
    const trimmedSubject = subject.trim();

    if (!trimmedName) {
      errorMessage = 'Please enter an event name or title.';
      return;
    }
    if (!trimmedSubject) {
      errorMessage = 'Please specify or select a subject.';
      return;
    }
    if (!eventDate) {
      errorMessage = 'Please select a date.';
      return;
    }

    isSaving = true;
    try {
      if (isEditMode && event) {
        const payload: UpdateSubjectEventPayload = {
          name: trimmedName,
          subject: trimmedSubject,
          event_type: eventType,
          event_date: eventDate,
          is_all_day: isAllDay,
          event_time: isAllDay ? null : eventTime,
          calendar_type: calendarType,
          google_calendar_id: calendarType === 'google' ? googleCalendarId : null,
          notes: notes.trim() || null,
          is_completed: isCompleted,
        };
        const updated = await subjectEventUpdate(event.id, payload);
        onSaved(updated);
      } else {
        const payload: CreateSubjectEventPayload = {
          name: trimmedName,
          subject: trimmedSubject,
          event_type: eventType,
          event_date: eventDate,
          is_all_day: isAllDay,
          event_time: isAllDay ? null : eventTime,
          calendar_type: calendarType,
          google_calendar_id: calendarType === 'google' ? googleCalendarId : null,
          notes: notes.trim() || null,
        };
        const created = await subjectEventCreate(payload);
        onSaved(created);
      }
      onClose();
    } catch (e: any) {
      logError(`Failed to save event: ${e}`);
      errorMessage = typeof e === 'string' ? e : e?.message || 'Failed to save event';
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete() {
    if (!event) return;
    if (!confirm(`Are you sure you want to delete "${event.name}"?`)) return;

    isDeleting = true;
    try {
      await subjectEventDelete(event.id);
      onDeleted?.(event.id);
      onClose();
    } catch (e: any) {
      logError(`Failed to delete event: ${e}`);
      errorMessage = typeof e === 'string' ? e : e?.message || 'Failed to delete event';
    } finally {
      isDeleting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="modal-backdrop" onclick={onClose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-content" onclick={(e) => e.stopPropagation()}>
    <!-- Modal Header -->
    <div class="modal-header">
      <div class="header-title-row">
        <span class="header-icon">
          {EVENT_TYPES.find(t => t.id === eventType)?.icon || '📌'}
        </span>
        <h2>{isEditMode ? 'Edit Academic Event' : 'New Academic Event'}</h2>
      </div>
      <button class="close-btn" aria-label="Close" onclick={onClose}>✕</button>
    </div>

    <!-- Error Banner -->
    {#if errorMessage}
      <div class="error-banner">
        {errorMessage}
      </div>
    {/if}

    <!-- Modal Form Body -->
    <div class="modal-body">
      <!-- Title / Name Input -->
      <div class="form-row">
        <label for="event-name-input">Event Title / Description *</label>
        <input
          id="event-name-input"
          type="text"
          bind:value={name}
          placeholder="e.g. Midterm Exam, Essay Submission, Chapter 5 Quiz"
          class="custom-input"
        />
      </div>

      <!-- Subject & Type Row -->
      <div class="form-grid-2">
        <div class="form-row">
          <label for="event-subject-input">Subject *</label>
          <div class="subject-input-wrapper">
            <input
              id="event-subject-input"
              type="text"
              bind:value={subject}
              placeholder="e.g. Biology, Calculus"
              list="subject-suggestions"
              class="custom-input"
            />
            <datalist id="subject-suggestions">
              {#each subjects as s}
                <option value={s.name}></option>
              {/each}
            </datalist>
            {#if subject.trim()}
              {@const subColor = getSubjectColor(subject)}
              <span
                class="subject-color-dot"
                style="background-color: {subColor};"
                title="Subject indicator"
              ></span>
            {/if}
          </div>
        </div>

        <div class="form-row">
          <label for="event-type-select">Event Type</label>
          <div class="type-pill-selector">
            {#each EVENT_TYPES as t}
              <button
                type="button"
                class="type-pill"
                class:active={eventType === t.id}
                onclick={() => eventType = t.id}
                title={t.label}
              >
                <span>{t.icon}</span>
                <span class="type-label">{t.label}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>

      <!-- Date & Time Row -->
      <div class="form-grid-2">
        <div class="form-row">
          <label for="event-date-input">Due Date *</label>
          <input
            id="event-date-input"
            type="date"
            bind:value={eventDate}
            class="custom-input"
          />
        </div>

        <div class="form-row">
          <div class="time-header-row">
            <label for="event-time-input">Time</label>
            <label class="all-day-checkbox-label">
              <input
                type="checkbox"
                bind:checked={isAllDay}
              />
              <span>All Day</span>
            </label>
          </div>
          {#if !isAllDay}
            <input
              id="event-time-input"
              type="time"
              bind:value={eventTime}
              class="custom-input"
            />
          {:else}
            <div class="all-day-hint">No specific time (All Day)</div>
          {/if}
        </div>
      </div>

      <!-- Calendar Sync Destination -->
      <div class="form-row">
        <label for="calendar-sync-select">Calendar Destination</label>
        <div class="calendar-choice-box">
          <div class="toggle-group">
            <button
              type="button"
              class:active={calendarType === 'local'}
              onclick={() => {
                calendarType = 'local';
                googleCalendarId = null;
              }}
            >
              <span class="cal-btn-icon">📅</span>
              <span>Local Calendar</span>
            </button>
            <button
              type="button"
              class:active={calendarType === 'google'}
              disabled={!isGoogleSignedIn}
              onclick={() => {
                calendarType = 'google';
                if (!googleCalendarId && calendars.length > 0) {
                  googleCalendarId = calendars.find(c => c.primary)?.id || calendars[0].id;
                }
              }}
              title={!isGoogleSignedIn ? 'Sign in to Google in Settings to sync' : 'Sync to Google Calendar'}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10"/>
              </svg>
              <span>Google Calendar</span>
            </button>
          </div>

          {#if calendarType === 'google'}
            <div class="google-calendar-select-wrapper">
              {#if calendars.length > 0}
                <label for="google-cal-picker" class="sublabel">Target Google Calendar:</label>
                <select
                  id="google-cal-picker"
                  bind:value={googleCalendarId}
                  class="custom-select"
                >
                  {#each calendars as cal}
                    <option value={cal.id}>
                      {cal.summary} {cal.primary ? '(Primary)' : ''}
                    </option>
                  {/each}
                </select>
              {:else}
                <span class="hint-muted">No writable Google Calendars found.</span>
              {/if}
            </div>
          {:else if !isGoogleSignedIn}
            <div class="google-hint">
              <span>Tip: Connect Google Calendar in Settings to automatically sync assignments and exams across your devices.</span>
            </div>
          {/if}
        </div>
      </div>

      <!-- Notes Field -->
      <div class="form-row">
        <label for="event-notes-input">Notes & Instructions (Optional)</label>
        <textarea
          id="event-notes-input"
          bind:value={notes}
          placeholder="Chapters to study, essay requirements, submission link..."
          rows="3"
          class="custom-textarea"
        ></textarea>
      </div>

      <!-- Completed Checkbox (for edit mode or manual marking) -->
      {#if isEditMode}
        <div class="form-row">
          <label class="completion-toggle-label">
            <input
              type="checkbox"
              bind:checked={isCompleted}
            />
            <span class="completion-text">Mark as completed / submitted</span>
          </label>
        </div>
      {/if}
    </div>

    <!-- Modal Actions Footer -->
    <div class="modal-actions">
      {#if isEditMode}
        <button
          type="button"
          class="btn-delete"
          onclick={handleDelete}
          disabled={isDeleting || isSaving}
        >
          {isDeleting ? 'Deleting...' : 'Delete'}
        </button>
      {/if}
      <div class="actions-right">
        <button
          type="button"
          class="btn-cancel"
          onclick={onClose}
          disabled={isSaving || isDeleting}
        >
          Cancel
        </button>
        <button
          type="button"
          class="btn-save"
          onclick={handleSave}
          disabled={isSaving || isDeleting}
        >
          {isSaving ? 'Saving...' : (isEditMode ? 'Save Changes' : 'Create Event')}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    animation: fade-in 0.15s ease-out;
  }

  .modal-content {
    background: var(--color-background);
    color: var(--color-foreground);
    width: 90%;
    max-width: 540px;
    max-height: 90vh;
    border-radius: 8px;
    border: 1px solid var(--color-separator);
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: slide-up 0.15s ease-out;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 20px;
    border-bottom: 1px solid var(--color-separator);
  }

  .header-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .header-icon {
    font-size: 1.25rem;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 600;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--color-foreground-darker, #a1a1aa);
    font-size: 1.1rem;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
    transition: var(--transition-default);
  }

  .close-btn:hover {
    color: var(--color-foreground);
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
    border-left: 3px solid #ef4444;
    padding: 8px 16px;
    font-size: 0.85rem;
    margin: 12px 20px 0;
    border-radius: 2px;
  }

  .modal-body {
    padding: 18px 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }

  label {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .sublabel {
    font-size: 0.76rem;
    margin-top: 6px;
    display: block;
  }

  .custom-input,
  .custom-select,
  .custom-textarea {
    width: 100%;
    padding: 8px 12px;
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    color: var(--color-foreground);
    font-size: 0.88rem;
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s, background-color 0.15s;
    box-sizing: border-box;
  }

  .custom-input:focus,
  .custom-select:focus,
  .custom-textarea:focus {
    border-color: var(--color-accent);
    background: color-mix(in oklch, var(--color-foreground) 12%, transparent);
  }

  .custom-select option {
    background: var(--color-background);
    color: var(--color-foreground);
  }

  .subject-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .subject-color-dot {
    position: absolute;
    right: 10px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: 0 0 2px rgba(0, 0, 0, 0.5);
  }

  /* Type Pill Selector */
  .type-pill-selector {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .type-pill {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 5px 8px;
    background: color-mix(in oklch, var(--color-foreground) 6%, transparent);
    border: 1px solid var(--color-separator);
    border-radius: 5px;
    color: var(--color-foreground-darker, #a1a1aa);
    font-size: 0.78rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .type-pill:hover {
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
    color: var(--color-foreground);
  }

  .type-pill.active {
    background: var(--color-focus-round);
    color: var(--color-background);
    border-color: var(--color-focus-round);
    font-weight: 600;
  }

  /* Time Header & All Day */
  .time-header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .all-day-checkbox-label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 0.78rem;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .all-day-hint {
    padding: 8px 12px;
    font-size: 0.82rem;
    color: var(--color-foreground-darker, #a1a1aa);
    background: color-mix(in oklch, var(--color-foreground) 4%, transparent);
    border-radius: 6px;
    border: 1px dashed var(--color-separator);
  }

  /* Calendar Toggle Box */
  .calendar-choice-box {
    background: color-mix(in oklch, var(--color-foreground) 4%, transparent);
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .toggle-group {
    display: flex;
    gap: 6px;
  }

  .toggle-group button {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 7px 12px;
    background: transparent;
    border: 1px solid var(--color-separator);
    border-radius: 4px;
    color: var(--color-foreground-darker, #a1a1aa);
    font-size: 0.82rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .toggle-group button:hover:not(:disabled) {
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
    color: var(--color-foreground);
  }

  .toggle-group button.active {
    background: var(--color-focus-round);
    color: var(--color-background);
    border-color: var(--color-focus-round);
    font-weight: 500;
  }

  .toggle-group button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .google-calendar-select-wrapper {
    margin-top: 4px;
  }

  .google-hint {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, #a1a1aa);
    padding: 4px 6px;
  }

  .hint-muted {
    font-size: 0.78rem;
    color: var(--color-foreground-darker, #a1a1aa);
    font-style: italic;
  }

  /* Completion Toggle */
  .completion-toggle-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    padding: 8px 10px;
    background: color-mix(in oklch, var(--color-foreground) 5%, transparent);
    border-radius: 6px;
    font-size: 0.85rem;
    color: var(--color-foreground);
  }

  .completion-toggle-label input {
    cursor: pointer;
  }

  /* Actions Footer */
  .modal-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    border-top: 1px solid var(--color-separator);
  }

  .actions-right {
    display: flex;
    gap: 10px;
    margin-left: auto;
  }

  .btn-cancel,
  .btn-delete,
  .btn-save {
    padding: 7px 16px;
    border-radius: 5px;
    font-size: 0.86rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid var(--color-separator);
    color: var(--color-foreground);
  }

  .btn-cancel:hover:not(:disabled) {
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
  }

  .btn-delete {
    background: transparent;
    border: 1px solid #ef4444;
    color: #ef4444;
  }

  .btn-delete:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.15);
  }

  .btn-save {
    background: var(--color-focus-round);
    border: 1px solid var(--color-focus-round);
    color: var(--color-background);
    font-weight: 500;
  }

  .btn-save:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-cancel:disabled,
  .btn-delete:disabled,
  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes slide-up {
    from { transform: translateY(12px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
</style>
