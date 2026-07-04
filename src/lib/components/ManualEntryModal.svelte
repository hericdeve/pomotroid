<script lang="ts">
  import { onMount } from 'svelte';
  import { createManualSession } from '$lib/ipc';
  import type { CreateManualSessionPayload, UpdateSessionPayload } from '$lib/types';
  import { settings } from '$lib/stores/settings';
  import EntryDetail from './EntryDetail.svelte';

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  let timeMode = $state<'now' | 'custom'>('now');
  let customDate = $state('');
  
  // Use a sensible default if the store isn't populated, but default to settings otherwise
  let durationMins = $state(25); 

  let detailPayload = $state<UpdateSessionPayload>({
    subject: '',
    subject_topic: '',
    study_type: 'None / Uncategorized',
    notes: ''
  });

  onMount(() => {
    // Initialize defaults on mount
    durationMins = Math.max(1, Math.round(($settings.time_work_secs || 1500) / 60));
    
    const now = new Date();
    // Adjust to local timezone for datetime-local input
    now.setMinutes(now.getMinutes() - now.getTimezoneOffset());
    customDate = now.toISOString().slice(0, 16);
  });

  async function handleSave() {
    let started_at: number;
    if (timeMode === 'now') {
      started_at = Math.floor(Date.now() / 1000);
    } else {
      started_at = Math.floor(new Date(customDate).getTime() / 1000);
      if (isNaN(started_at)) {
        started_at = Math.floor(Date.now() / 1000); // fallback
      }
    }

    const payload: CreateManualSessionPayload = {
      started_at,
      duration_secs: durationMins * 60,
      subject: detailPayload.subject || null,
      subject_topic: detailPayload.subject_topic || null,
      study_type: detailPayload.study_type === 'None / Uncategorized' ? null : detailPayload.study_type,
      notes: detailPayload.notes || null
    };

    try {
      await createManualSession(payload);
      onclose();
    } catch (err) {
      console.error('Failed to create manual session:', err);
    }
  }
</script>

<div class="modal-backdrop" onclick={onclose} role="presentation">
  <div class="modal-content" onclick={(e) => e.stopPropagation()} role="presentation">
    <div class="modal-header">
      <h2>Manual Entry</h2>
      <button class="close-btn" aria-label="Close" onclick={onclose}>×</button>
    </div>
    
    <div class="scrollable-body">
      <div class="form-row">
        <!-- svelte-ignore a11y_label_has_associated_control -->
        <label>Time Occurred</label>
        <div class="toggle-group">
          <button class:active={timeMode === 'now'} onclick={() => timeMode = 'now'}>Right Now</button>
          <button class:active={timeMode === 'custom'} onclick={() => timeMode = 'custom'}>Exact Time</button>
        </div>
      </div>
      
      {#if timeMode === 'custom'}
        <div class="form-row">
          <input type="datetime-local" bind:value={customDate} class="custom-input" />
        </div>
      {/if}

      <div class="form-row">
        <label for="duration-input">Duration (minutes)</label>
        <input id="duration-input" type="number" bind:value={durationMins} min="1" class="custom-input" />
      </div>

      <div class="divider"></div>

      <!-- The reusable detail form, bind:payload to extract values without firing saves -->
      <EntryDetail bind:payload={detailPayload} />
    </div>

    <div class="actions">
      <button class="btn-cancel" onclick={onclose}>Cancel</button>
      <button class="btn-save" onclick={handleSave}>Save Entry</button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
    animation: fade-in 0.2s ease-out;
  }

  .modal-content {
    background: var(--color-background);
    width: 90%;
    max-width: 400px;
    border-radius: 8px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.3);
    display: flex;
    flex-direction: column;
    max-height: 90vh;
    animation: slide-up 0.2s ease-out;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-separator);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.1rem;
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

  .scrollable-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    font-size: 0.85rem;
    color: var(--color-foreground-darker);
    font-weight: 500;
  }

  .toggle-group {
    display: flex;
    gap: 8px;
  }

  .toggle-group button {
    flex: 1;
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
    border: 1px solid transparent;
    color: var(--color-foreground);
    padding: 8px;
    border-radius: 4px;
    font-size: 0.85rem;
    cursor: pointer;
    transition: var(--transition-default);
  }

  .toggle-group button.active {
    background: var(--color-focus-round);
    color: var(--color-background);
  }

  .toggle-group button:hover:not(.active) {
    background: color-mix(in oklch, var(--color-foreground) 15%, transparent);
  }

  .custom-input {
    width: 100%;
    padding: 10px 14px;
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--color-foreground);
    font-size: 0.9rem;
    font-family: 'Mona Sans', system-ui, sans-serif;
    transition: var(--transition-default);
    outline: none;
  }

  .custom-input:focus {
    border-color: var(--color-accent);
    background: color-mix(in oklch, var(--color-foreground) 15%, transparent);
  }

  .divider {
    height: 1px;
    background: var(--color-separator);
    margin: 8px 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--color-separator);
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid var(--color-foreground-darker);
    color: var(--color-foreground);
    padding: 8px 16px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.15s;
  }

  .btn-cancel:hover {
    background: color-mix(in oklch, var(--color-foreground) 10%, transparent);
  }

  .btn-save {
    background: var(--color-focus-round);
    border: none;
    color: var(--color-background);
    padding: 8px 16px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 500;
    transition: filter 0.15s;
  }

  .btn-save:hover {
    filter: brightness(1.1);
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
