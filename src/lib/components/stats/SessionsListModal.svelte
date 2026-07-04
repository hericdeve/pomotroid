<script lang="ts">
  import { onMount } from 'svelte';
  import { sessionsGetHistory } from '$lib/ipc';
  import type { SessionRow, SessionHistoryPage } from '$lib/types';

  let { 
    dateFrom, 
    dateTo, 
    label,
    onClose, 
    onEditSession 
  }: { 
    dateFrom: number; 
    dateTo: number; 
    label: string;
    onClose: () => void;
    onEditSession: (id: number) => void;
  } = $props();

  let history = $state<SessionHistoryPage | null>(null);
  let loading = $state(true);

  // Pagination inside modal
  let limit = 20;
  let offset = $state(0);

  function formatUnix(ts: number) {
    const d = new Date(ts * 1000);
    return `${d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
  }

  function formatDuration(secs: number) {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}m ${s}s`;
  }

  async function loadHistory() {
    loading = true;
    try {
      history = await sessionsGetHistory(limit, offset, {
        date_from: dateFrom,
        date_to: dateTo
      });
    } catch (e) {
      console.error('Failed to load sessions for modal', e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Reload when offset changes
    const _o = offset;
    loadHistory();
  });

  onMount(() => {
    loadHistory();
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="modal-backdrop" onclick={onClose} role="dialog">
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <div class="header">
      <h2>{label}</h2>
      <button class="close-btn" onclick={onClose} aria-label="Close">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    </div>

    <div class="content">
      {#if loading}
        <div class="msg">Loading sessions...</div>
      {:else if !history || history.sessions.length === 0}
        <div class="msg">No sessions found.</div>
      {:else}
        <div class="list">
          {#each history.sessions as row}
            <!-- svelte-ignore a11y_interactive_supports_focus -->
            <div 
              class="session-row" 
              role="button"
              onclick={() => onEditSession(row.id)}
            >
              <div class="time">{formatUnix(row.started_at)}</div>
              <div class="details">
                <div class="primary">
                  <span class="type">{row.round_type === 'work' ? 'Work' : row.round_type === 'short-break' ? 'Short Break' : 'Long Break'}</span>
                  <span class="duration">&bull; {formatDuration(row.duration_secs)}</span>
                </div>
                <div class="secondary">
                  {#if row.subject}
                    <span class="subject">{row.subject}</span>
                    {#if row.subject_topic}
                      <span class="topic"> / {row.subject_topic}</span>
                    {/if}
                  {:else}
                    <span class="no-tags">No tags</span>
                  {/if}
                </div>
              </div>
              <div class="action-icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
                  <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
                </svg>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    {#if history && history.total > limit}
      <div class="footer">
        <button disabled={offset === 0} onclick={() => offset = Math.max(0, offset - limit)}>Prev</button>
        <span class="page-info">
          {Math.min(offset + 1, history.total)} - {Math.min(offset + limit, history.total)} of {history.total}
        </span>
        <button disabled={offset + limit >= history.total} onclick={() => offset += limit}>Next</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--color-background);
    border-radius: 8px;
    width: 450px;
    max-width: 90vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
    border: 1px solid var(--color-separator);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 24px;
    border-bottom: 1px solid var(--color-separator);
  }

  h2 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--color-foreground);
    font-weight: 500;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--color-foreground-darker);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
  }

  .close-btn:hover {
    background: var(--color-background-lighter);
    color: var(--color-foreground);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .msg {
    padding: 32px;
    text-align: center;
    color: var(--color-foreground-darker);
    font-size: 0.9rem;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .session-row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    background: var(--color-background-lighter);
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s;
    border: 1px solid transparent;
  }

  .session-row:hover {
    background: var(--color-hover);
    border-color: var(--color-accent);
  }

  .session-row:hover .action-icon {
    opacity: 1;
  }

  .time {
    font-variant-numeric: tabular-nums;
    color: var(--color-foreground-darker);
    font-size: 0.85rem;
    min-width: 60px;
  }

  .details {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .primary {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.9rem;
  }

  .type {
    font-weight: 500;
    color: var(--color-foreground);
  }

  .duration {
    color: var(--color-foreground-darker);
    font-size: 0.85rem;
  }

  .secondary {
    font-size: 0.8rem;
  }

  .subject {
    color: var(--color-primary);
    font-weight: 500;
  }

  .topic {
    color: var(--color-foreground-darker);
  }

  .no-tags {
    color: var(--color-foreground-darker);
    font-style: italic;
    opacity: 0.6;
  }

  .action-icon {
    opacity: 0;
    transition: opacity 0.15s;
    color: var(--color-primary);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 24px;
    border-top: 1px solid var(--color-separator);
    font-size: 0.85rem;
  }

  .footer button {
    background: var(--color-background-lighter);
    color: var(--color-foreground);
    border: 1px solid var(--color-separator);
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
  }

  .footer button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .page-info {
    color: var(--color-foreground-darker);
  }
</style>
