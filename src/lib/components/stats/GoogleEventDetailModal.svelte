<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import type { GoogleOverlayEvent } from '$lib/types';

  interface Props {
    event: GoogleOverlayEvent;
    onClose: () => void;
  }

  let { event, onClose }: Props = $props();

  function formatTime(minutes: number): string {
    const h = Math.floor(minutes / 60) % 24;
    const m = minutes % 60;
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}`;
  }

  function handleOpenLink() {
    if (event.html_link) {
      openUrl(event.html_link);
    }
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-content" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <div class="header-badge" style="background-color: {event.calendar_color};"></div>
      <div class="header-titles">
        <span class="calendar-name">{event.calendar_summary}</span>
        <h3 class="event-title">{event.summary}</h3>
      </div>
      <button class="btn-close" onclick={onClose} aria-label="Close">✕</button>
    </div>

    <div class="modal-body">
      <div class="detail-row">
        <span class="detail-icon">🕒</span>
        <div class="detail-text">
          <span class="detail-label">Time</span>
          <span class="detail-value">
            {#if event.is_all_day}
              All day
            {:else}
              {formatTime(event.start_minute)} – {formatTime(event.end_minute)}
            {/if}
          </span>
        </div>
      </div>

      {#if event.location}
        <div class="detail-row">
          <span class="detail-icon">📍</span>
          <div class="detail-text">
            <span class="detail-label">Location</span>
            <span class="detail-value">{event.location}</span>
          </div>
        </div>
      {/if}

      {#if event.description}
        <div class="detail-row description-row">
          <span class="detail-icon">📝</span>
          <div class="detail-text">
            <span class="detail-label">Description</span>
            <div class="description-content">{event.description}</div>
          </div>
        </div>
      {/if}
    </div>

    <div class="modal-footer">
      <button class="btn-secondary" onclick={onClose} type="button">Close</button>
      {#if event.html_link}
        <button class="btn-primary" onclick={handleOpenLink} type="button">
          Open in Google Calendar ↗
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-content {
    background: var(--color-background, #18181b);
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    width: 440px;
    max-width: 90vw;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--color-foreground, #f4f4f5);
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 18px 20px;
    border-bottom: 1px solid var(--color-separator, rgba(255, 255, 255, 0.1));
  }

  .header-badge {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    margin-top: 6px;
    flex-shrink: 0;
  }

  .header-titles {
    flex: 1;
  }

  .calendar-name {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, #a1a1aa);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 600;
    display: block;
    margin-bottom: 2px;
  }

  .event-title {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 600;
    color: var(--color-foreground, #f4f4f5);
    line-height: 1.3;
  }

  .btn-close {
    background: transparent;
    border: none;
    color: var(--color-foreground-darker, #a1a1aa);
    cursor: pointer;
    font-size: 1rem;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .btn-close:hover {
    color: var(--color-foreground, #fff);
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-height: 350px;
    overflow-y: auto;
  }

  .detail-row {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .detail-icon {
    font-size: 1.1rem;
    line-height: 1;
    margin-top: 2px;
    opacity: 0.8;
  }

  .detail-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .detail-label {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, #a1a1aa);
    text-transform: uppercase;
    font-weight: 500;
  }

  .detail-value {
    font-size: 0.95rem;
    color: var(--color-foreground, #f4f4f5);
  }

  .description-content {
    font-size: 0.85rem;
    line-height: 1.5;
    color: var(--color-foreground, #e4e4e7);
    white-space: pre-wrap;
    background: var(--color-background-light, rgba(255, 255, 255, 0.03));
    padding: 10px 12px;
    border-radius: 6px;
    margin-top: 4px;
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.08));
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid var(--color-separator, rgba(255, 255, 255, 0.1));
    background: var(--color-background-light, rgba(255, 255, 255, 0.02));
  }

  .btn-secondary {
    background: transparent;
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.15));
    color: var(--color-foreground, #f4f4f5);
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-primary {
    background: var(--color-focus-round, #4285f4);
    border: none;
    color: #fff;
    font-weight: 600;
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: opacity 0.15s;
  }

  .btn-primary:hover {
    opacity: 0.9;
  }
</style>
