<script lang="ts">
  import { onMount } from 'svelte';
  import { getSession, updateSession } from '$lib/ipc';
  import EntryDetail from './EntryDetail.svelte';
  import { pendingTags } from '$lib/stores/pendingTags';
  import { get } from 'svelte/store';

  interface Props {
    sessionId: number | null;
    onClose: () => void;
  }
  let { sessionId, onClose }: Props = $props();

  let payload = $state<{
    subject: string;
    subject_topic: string;
    study_type: string;
    notes: string;
  } | null>(null);
  let initialLoaded = $state(false);

  onMount(async () => {
    try {
      if (sessionId !== null) {
        const row = await getSession(sessionId);
        if (row) {
          payload = {
            subject: row.subject || '',
            subject_topic: row.subject_topic || '',
            study_type: row.study_type || '',
            notes: row.notes || '',
          };
          initialLoaded = true;
        } else {
          onClose();
        }
      } else {
        const p = get(pendingTags);
        payload = { ...p };
        initialLoaded = true;
      }
    } catch (e) {
      console.error(e);
      onClose();
    }
  });

  let timeout: ReturnType<typeof setTimeout>;
  $effect(() => {
    if (!initialLoaded || !payload) return;
    
    // Read properties to subscribe to changes
    const p = { ...payload };
    
    clearTimeout(timeout);
    timeout = setTimeout(async () => {
      try {
        if (sessionId !== null) {
          await updateSession(sessionId, {
            subject: p.subject || null,
            subject_topic: p.subject_topic || null,
            study_type: p.study_type || null,
            notes: p.notes || null,
          });
        }
        pendingTags.set({ ...p });
      } catch (e) {
        console.error(e);
      }
    }, 500);
  });
</script>

<div class="modal-overlay" role="presentation" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal" role="dialog" onclick={(e) => e.stopPropagation()}>
    <div class="header">
      <h2>Tag Session</h2>
      <button class="close-btn" onclick={onClose} aria-label="Close">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
    <div class="content">
      {#if initialLoaded && payload}
        <EntryDetail bind:payload />
      {:else}
        <div class="loading">Loading...</div>
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
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(4px);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    animation: fade-in 0.2s ease-out;
  }
  .modal {
    background: var(--color-background);
    border-radius: 12px;
    width: 100%;
    max-width: 400px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-background-light);
  }
  .header h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
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
  }
  .close-btn:hover {
    background: var(--color-hover);
    color: var(--color-foreground);
  }
  .content {
    padding: 20px;
    max-height: 70vh;
    overflow-y: auto;
  }
  .loading {
    color: var(--color-foreground-darker, var(--color-foreground));
    text-align: center;
    padding: 20px;
  }
  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>
