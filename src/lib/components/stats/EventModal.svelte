<script lang="ts">
  import { onMount } from 'svelte';
  import {
    subjectsGetAll,
    subjectEventCreate,
    subjectEventUpdate,
    subjectEventDelete,
  } from '$lib/ipc';
  import type {
    SubjectEvent,
    SubjectEventType,
    SubjectStats,
    CreateSubjectEventPayload,
    UpdateSubjectEventPayload,
  } from '$lib/types';
  import { getSubjectColor } from '$lib/utils/subjectColors';
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
  let name = $state('');
  let subject = $state('');
  let eventType = $state<SubjectEventType>('exam');
  let eventDate = $state('');
  let isAllDay = $state(false);
  let eventTime = $state('09:00');
  let endDate = $state('');
  let endTime = $state('');
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

  function addOneHour(timeStr: string): string {
    const [h, m] = timeStr.split(':').map(Number);
    const newH = ((h || 0) + 1) % 24;
    return `${String(newH).padStart(2, '0')}:${String(m || 0).padStart(2, '0')}`;
  }

  onMount(async () => {
    try {
      subjects = await subjectsGetAll().catch(() => []);
    } catch (e) {
      logError(`Failed to load subjects for EventModal: ${e}`);
    }

    if (event) {
      name = event.name;
      subject = event.subject;
      eventType = (event.event_type as SubjectEventType) || 'exam';
      eventDate = event.event_date;
      isAllDay = event.is_all_day;
      eventTime = event.event_time || '09:00';
      endDate = event.end_date || event.event_date;
      endTime = event.end_time || (event.event_time ? addOneHour(event.event_time) : '10:00');
      notes = event.notes || '';
      isCompleted = event.is_completed;
    } else {
      name = '';
      subject = initialSubject || (subjects.length > 0 ? subjects[0].name : '');
      eventType = 'assignment';
      const today = initialDate || getTodayYmd();
      eventDate = today;
      isAllDay = false;
      eventTime = '09:00';
      endDate = today;
      endTime = '10:00';
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
          end_date: isAllDay ? null : (endDate || null),
          end_time: isAllDay ? null : (endTime || null),
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
          end_date: isAllDay ? null : (endDate || null),
          end_time: isAllDay ? null : (endTime || null),
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
      <div class="header-title">
        {isEditMode ? 'Edit Academic Event' : 'New Academic Event'}
      </div>
      <button class="close-btn" aria-label="Close" onclick={onClose}>×</button>
    </div>

    <!-- Error Banner -->
    {#if errorMessage}
      <div class="error-banner">
        {errorMessage}
      </div>
    {/if}

    <!-- Modal Form Body -->
    <div class="scrollable-body">
      <!-- Title Input -->
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
          <!-- svelte-ignore a11y_label_has_associated_control -->
          <label>Event Type</label>
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
                <span>{t.label}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>

      <!-- Date & Time Row -->
      <div class="form-grid-2">
        <div class="form-row">
          <label for="event-date-input">Start Date *</label>
          <input
            id="event-date-input"
            type="date"
            bind:value={eventDate}
            class="custom-input"
          />
        </div>

        <div class="form-row">
          <div class="time-header-row">
            <label for="event-time-input">Start Time</label>
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

      <!-- End Date & Time Row (only for non-all-day events) -->
      {#if !isAllDay}
        <div class="form-grid-2">
          <div class="form-row">
            <label for="event-end-date-input">End Date</label>
            <input
              id="event-end-date-input"
              type="date"
              bind:value={endDate}
              class="custom-input"
              min={eventDate}
            />
          </div>
          <div class="form-row">
            <label for="event-end-time-input">End Time</label>
            <input
              id="event-end-time-input"
              type="time"
              bind:value={endTime}
              class="custom-input"
            />
          </div>
        </div>
      {/if}

      <!-- Notes Field -->
      <div class="form-row">
        <label for="event-notes-input">Notes & Instructions (Optional)</label>
        <textarea
          id="event-notes-input"
          bind:value={notes}
          placeholder="Chapters to study, assignment requirements, submission link..."
          rows="2"
          class="custom-textarea"
        ></textarea>
      </div>

      <!-- Completed Checkbox (for edit mode) -->
      {#if isEditMode}
        <div class="form-row">
          <label class="completion-toggle-label">
            <input
              type="checkbox"
              bind:checked={isCompleted}
            />
            <span>Mark as completed / submitted</span>
          </label>
        </div>
      {/if}
    </div>

    <!-- Modal Actions Footer -->
    <div class="actions">
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

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
    animation: fade-in 0.15s ease-out;
  }

  .modal-content {
    background: var(--color-background);
    border: 1px solid var(--color-separator);
    border-radius: 8px;
    width: 480px;
    max-width: 90vw;
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    animation: slide-up 0.15s ease-out;
    color: var(--color-foreground);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-separator);
  }

  .header-title {
    font-size: 1.05rem;
    font-weight: 600;
    color: var(--color-foreground);
    letter-spacing: 0.02em;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.5rem;
    color: var(--color-foreground-darker);
    cursor: pointer;
    padding: 0;
    line-height: 1;
    transition: color 0.15s;
  }

  .close-btn:hover {
    color: var(--color-accent);
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

  .scrollable-body {
    padding: 20px;
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
    gap: 12px;
  }

  label {
    font-size: 0.85rem;
    color: var(--color-foreground-darker);
    font-weight: 500;
  }

  .custom-input,
  .custom-textarea {
    width: 100%;
    padding: 8px 12px;
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--color-foreground);
    font-size: 0.88rem;
    font-family: 'Mona Sans', system-ui, sans-serif;
    transition: var(--transition-default);
    outline: none;
    box-sizing: border-box;
  }

  .custom-input:focus,
  .custom-textarea:focus {
    border-color: var(--color-focus-round);
    background: color-mix(in oklch, var(--color-foreground) 15%, transparent);
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
    gap: 4px;
  }

  .type-pill {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 7px;
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--color-foreground-darker);
    font-size: 0.76rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .type-pill:hover {
    background: color-mix(in oklch, var(--color-foreground) 14%, transparent);
    color: var(--color-foreground);
  }

  .type-pill.active {
    background: var(--color-focus-round);
    color: var(--color-background);
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
    color: var(--color-foreground-darker);
  }

  .all-day-hint {
    padding: 8px 12px;
    font-size: 0.82rem;
    color: var(--color-foreground-darker);
    background: color-mix(in oklch, var(--color-foreground) 4%, transparent);
    border-radius: 4px;
    border: 1px dashed var(--color-separator);
  }

  /* Completion Toggle */
  .completion-toggle-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    padding: 6px 8px;
    background: color-mix(in oklch, var(--color-foreground) 5%, transparent);
    border-radius: 4px;
    font-size: 0.82rem;
    color: var(--color-foreground);
  }

  /* Actions Footer */
  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--color-separator);
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid var(--color-foreground-darker);
    color: var(--color-foreground);
    padding: 7px 16px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.88rem;
    transition: background 0.15s;
  }

  .btn-cancel:hover:not(:disabled) {
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
  }

  .btn-save {
    background: var(--color-focus-round);
    border: none;
    color: var(--color-background);
    padding: 7px 18px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.88rem;
    font-weight: 500;
    transition: filter 0.15s;
  }

  .btn-save:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-delete {
    background: transparent;
    border: 1px solid #ef4444;
    color: #ef4444;
    padding: 7px 16px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.88rem;
    margin-right: auto;
    transition: background 0.15s;
  }

  .btn-delete:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.15);
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
    from { transform: translateY(10px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
</style>
