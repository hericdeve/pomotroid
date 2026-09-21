<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { googleCalendarSaveCredentials } from '$lib/ipc';

  interface Props {
    initialClientId?: string | null;
    initialClientSecret?: string | null;
    onClose: () => void;
    onSaved: () => void;
  }

  let { initialClientId = '', initialClientSecret = '', onClose, onSaved }: Props = $props();

  let clientId = $state(initialClientId || '');
  let clientSecret = $state(initialClientSecret || '');
  let saving = $state(false);
  let errorMsg = $state<string | null>(null);

  async function handleSave() {
    if (!clientId.trim()) {
      errorMsg = 'Client ID is required';
      return;
    }
    saving = true;
    errorMsg = null;
    try {
      await googleCalendarSaveCredentials(clientId.trim(), clientSecret.trim() || null);
      onSaved();
    } catch (e) {
      errorMsg = String(e);
    } finally {
      saving = false;
    }
  }

  function openConsole() {
    openUrl('https://console.cloud.google.com/apis/credentials');
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-content" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <div class="title-with-icon">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M19.5 3h-15C3.12 3 2 4.12 2 5.5v13C2 19.88 3.12 21 4.5 21h15c1.38 0 2.5-1.12 2.5-2.5v-13C22 4.12 20.88 3 19.5 3zm-9 14H6v-2h4.5v2zm0-4H6v-2h4.5v2zm0-4H6V7h4.5v2zm7.5 8h-6v-2h6v2zm0-4h-6v-2h6v2zm0-4h-6V7h6v2z" opacity="0.9"/>
        </svg>
        <h3>Google Calendar Credentials</h3>
      </div>
      <button class="btn-close" onclick={onClose} aria-label="Close">✕</button>
    </div>

    <div class="modal-body">
      <div class="info-banner">
        <p>
          To connect your Google Calendar, provide an OAuth 2.0 Client ID from your Google Cloud project:
        </p>
        <ol>
          <li>Open <strong>Google Cloud Console → APIs & Services → Credentials</strong>.</li>
          <li>Click <strong>Create Credentials → OAuth client ID</strong>.</li>
          <li>Select application type <strong>Desktop app</strong> (or Installed Application).</li>
          <li>Copy and paste the generated <strong>Client ID</strong> below.</li>
        </ol>
        <button class="btn-link" onclick={openConsole} type="button">
          Open Google Cloud Console ↗
        </button>
      </div>

      {#if errorMsg}
        <div class="error-banner">{errorMsg}</div>
      {/if}

      <div class="form-group">
        <label for="client-id">Client ID <span class="required">*</span></label>
        <input
          id="client-id"
          type="text"
          bind:value={clientId}
          placeholder="e.g. 123456789-abcdef.apps.googleusercontent.com"
          disabled={saving}
        />
      </div>

      <div class="form-group">
        <label for="client-secret">Client Secret <span class="optional">(optional for desktop apps)</span></label>
        <input
          id="client-secret"
          type="password"
          bind:value={clientSecret}
          placeholder="Leave blank if not needed"
          disabled={saving}
        />
      </div>
    </div>

    <div class="modal-footer">
      <button class="btn-cancel" onclick={onClose} disabled={saving} type="button">Cancel</button>
      <button class="btn-save" onclick={handleSave} disabled={saving} type="button">
        {#if saving}Saving...{:else}Save Credentials{/if}
      </button>
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
    width: 480px;
    max-width: 90vw;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--color-foreground, #f4f4f5);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-separator, rgba(255, 255, 255, 0.1));
  }

  .title-with-icon {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--color-focus-round, #4285f4);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--color-foreground, #f4f4f5);
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
  }

  .info-banner {
    background: var(--color-background-light, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.08));
    border-radius: 8px;
    padding: 12px 16px;
    font-size: 0.85rem;
    line-height: 1.45;
  }

  .info-banner p {
    margin: 0 0 8px 0;
  }

  .info-banner ol {
    margin: 0 0 10px 0;
    padding-left: 20px;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .info-banner li {
    margin-bottom: 4px;
  }

  .btn-link {
    background: none;
    border: none;
    color: var(--color-focus-round, #3b82f6);
    font-size: 0.85rem;
    font-weight: 500;
    padding: 0;
    cursor: pointer;
    text-decoration: underline;
  }

  .btn-link:hover {
    opacity: 0.85;
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #ef4444;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--color-foreground, #f4f4f5);
  }

  .required {
    color: #ef4444;
  }

  .optional {
    font-size: 0.75rem;
    font-weight: normal;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  input {
    background: var(--color-background-light, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.15));
    border-radius: 6px;
    padding: 10px 12px;
    color: var(--color-foreground, #f4f4f5);
    font-size: 0.9rem;
    outline: none;
    transition: border-color 0.15s;
  }

  input:focus {
    border-color: var(--color-focus-round, #4285f4);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 20px;
    border-top: 1px solid var(--color-separator, rgba(255, 255, 255, 0.1));
    background: var(--color-background-light, rgba(255, 255, 255, 0.02));
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.15));
    color: var(--color-foreground, #f4f4f5);
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-save {
    background: var(--color-focus-round, #4285f4);
    border: none;
    color: #fff;
    font-weight: 600;
    padding: 8px 18px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: opacity 0.15s;
  }

  .btn-save:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
