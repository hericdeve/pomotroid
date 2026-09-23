<script lang="ts">
  import { onMount } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    googleCalendarGetStatus,
    googleCalendarSaveCredentials,
    googleCalendarAuthStart,
    googleCalendarSignOut,
    googleCalendarGetCalendars,
    googleCalendarToggleVisibility,
    googleCalendarSetSyncedCalendar,
    googleCalendarSyncNow,
    calendarGetLocalVisible,
    calendarSetLocalVisible,
    googleCalendarSetEventsCalendar,
    googleCalendarGetEventsCalendar,
  } from '$lib/ipc';
  import SettingsToggle from '$lib/components/settings/SettingsToggle.svelte';
  import type { GoogleAuthStatus, GoogleCalendarItem } from '$lib/types';
  import { info, error as logError } from '@tauri-apps/plugin-log';

  let authStatus = $state<GoogleAuthStatus>({
    is_signed_in: false,
    email: null,
    client_id: null,
    has_client_secret: false,
  });

  let calendars = $state<GoogleCalendarItem[]>([]);
  let localVisible = $state(true);

  // Credentials form state
  let clientId = $state('');
  let clientSecret = $state('');
  let isSavingCreds = $state(false);
  let credsSavedMsg = $state<string | null>(null);
  let credsErrorMsg = $state<string | null>(null);
  let showInstructions = $state(false);

  // Actions state
  let isSigningIn = $state(false);
  let isSyncing = $state(false);
  let syncMsg = $state<string | null>(null);
  let authErrorMsg = $state<string | null>(null);

  let syncedCalendar = $derived(calendars.find((c) => c.is_synced) || null);
  let eventsCalendar = $derived(calendars.find((c) => c.is_events_synced) || null);

  function getMondayYmd(): string {
    const d = new Date();
    const day = d.getDay();
    const diffToMonday = (day + 6) % 7;
    d.setDate(d.getDate() - diffToMonday);
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const dayNum = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${dayNum}`;
  }

  async function loadData() {
    try {
      const [status, localVis] = await Promise.all([
        googleCalendarGetStatus(),
        calendarGetLocalVisible().catch(() => localVisible),
      ]);
      authStatus = status;
      localVisible = localVis;
      if (status.client_id) {
        clientId = status.client_id;
      }

      if (status.is_signed_in) {
        calendars = await googleCalendarGetCalendars();
      } else {
        calendars = [];
      }
    } catch (e) {
      await logError(`[calendar_settings] Failed to load initial data: ${e}`);
    }
  }

  onMount(() => {
    loadData();
  });

  async function handleSaveCredentials() {
    if (!clientId.trim()) {
      credsErrorMsg = 'Client ID is required';
      credsSavedMsg = null;
      return;
    }
    isSavingCreds = true;
    credsErrorMsg = null;
    credsSavedMsg = null;
    try {
      await googleCalendarSaveCredentials(clientId.trim(), clientSecret.trim() || null);
      authStatus = await googleCalendarGetStatus();
      credsSavedMsg = 'Credentials saved successfully!';
      setTimeout(() => {
        credsSavedMsg = null;
      }, 3500);
    } catch (e) {
      credsErrorMsg = String(e);
    } finally {
      isSavingCreds = false;
    }
  }

  async function handleSignIn() {
    if (!authStatus.client_id && !clientId.trim()) {
      credsErrorMsg = 'Please enter and save a Client ID before connecting.';
      return;
    }
    if (clientId.trim() !== (authStatus.client_id || '')) {
      await handleSaveCredentials();
    }
    isSigningIn = true;
    authErrorMsg = null;
    try {
      await googleCalendarAuthStart();
      await loadData();
      await info('[calendar_settings] Google authentication successful');
    } catch (e) {
      authErrorMsg = `Sign-in failed: ${e}`;
      await logError(`[calendar_settings] Google auth failed: ${e}`);
    } finally {
      isSigningIn = false;
    }
  }

  async function handleSignOut() {
    try {
      await googleCalendarSignOut();
      await loadData();
      await info('[calendar_settings] Signed out of Google');
    } catch (e) {
      await logError(`[calendar_settings] Sign-out failed: ${e}`);
    }
  }

  async function handleSyncNow() {
    if (!authStatus.is_signed_in) return;
    isSyncing = true;
    syncMsg = null;
    try {
      await googleCalendarSyncNow(getMondayYmd());
      syncMsg = 'Synced successfully!';
      setTimeout(() => {
        syncMsg = null;
      }, 3000);
    } catch (e) {
      syncMsg = `Sync failed: ${e}`;
    } finally {
      isSyncing = false;
    }
  }

  async function handleToggleLocal() {
    localVisible = !localVisible;
    try {
      await calendarSetLocalVisible(localVisible);
    } catch (e) {
      await logError(`[calendar_settings] Failed to save local visibility: ${e}`);
    }
  }

  async function handleToggleCalendar(calId: string, currentVal: boolean) {
    const newVal = !currentVal;
    calendars = calendars.map((c) => (c.id === calId ? { ...c, is_visible: newVal } : c));
    try {
      await googleCalendarToggleVisibility(calId, newVal);
    } catch (e) {
      await logError(`[calendar_settings] Failed to toggle calendar visibility: ${e}`);
    }
  }

  async function handleSelectSyncedCalendar(calId: string) {
    const isCurrentlySynced = calendars.find((c) => c.id === calId)?.is_synced;
    const targetId = isCurrentlySynced ? null : calId;

    calendars = calendars.map((c) => ({
      ...c,
      is_synced: targetId === c.id,
      is_visible: targetId === c.id ? true : c.is_visible,
    }));

    try {
      await googleCalendarSetSyncedCalendar(targetId);
      if (targetId) {
        await handleSyncNow();
      }
    } catch (e) {
      await logError(`[calendar_settings] Failed to set synced calendar: ${e}`);
    }
  }

  async function handleSelectEventsCalendar(calId: string) {
    const isCurrentlySelected = calendars.find((c) => c.id === calId)?.is_events_synced;
    const targetId = isCurrentlySelected ? null : calId;

    calendars = calendars.map((c) => ({
      ...c,
      is_events_synced: targetId === c.id,
    }));

    try {
      await googleCalendarSetEventsCalendar(targetId);
    } catch (e) {
      await logError(`[calendar_settings] Failed to set events calendar: ${e}`);
    }
  }

  function openGoogleConsole() {
    openUrl('https://console.cloud.google.com/apis/credentials');
  }
</script>

<div class="section">
  <!-- 1. Google Account Connection -->
  <div class="group-heading">Google Account</div>

  <div class="account-card">
    <div class="account-status-row">
      <div class="account-info">
        <span class="status-dot" class:connected={authStatus.is_signed_in}></span>
        <div class="account-text">
          {#if authStatus.is_signed_in}
            <span class="account-title">Connected</span>
            <span class="account-subtitle">{authStatus.email || 'Google Calendar active'}</span>
          {:else}
            <span class="account-title">Not Connected</span>
            <span class="account-subtitle">Sign in to sync timetables and view overlays</span>
          {/if}
        </div>
      </div>

      <div class="account-actions">
        {#if authStatus.is_signed_in}
          <button
            class="action-btn sync-btn"
            onclick={handleSyncNow}
            disabled={isSyncing}
            title="Sync calendar events now"
          >
            <span class="btn-icon" class:spinning={isSyncing}>🔄</span>
            <span>{isSyncing ? 'Syncing...' : 'Sync Now'}</span>
          </button>
          <button class="action-btn disconnect-btn" onclick={handleSignOut}>
            Disconnect
          </button>
        {:else}
          <button
            class="action-btn connect-btn"
            onclick={handleSignIn}
            disabled={isSigningIn}
          >
            {#if isSigningIn}Connecting...{:else}Connect with Google{/if}
          </button>
        {/if}
      </div>
    </div>

    {#if authErrorMsg}
      <div class="error-notice">{authErrorMsg}</div>
    {/if}
    {#if syncMsg}
      <div class="feedback-notice" class:success={!syncMsg.includes('failed')}>
        {syncMsg}
      </div>
    {/if}
  </div>

  <!-- 2. Two-Way Sync Configuration (when signed in) -->
  {#if authStatus.is_signed_in && calendars.length > 0}
    <div class="group-heading">Two-Way Synchronization</div>
    <div class="desc-text">
      Select <strong>one</strong> Google Calendar to sync bidirectionally with Pomotroid. Session names and hours will be synchronized with Google Calendar, while your rounds, tags, and study goals stay private on this device.
    </div>

    <div class="cals-list">
      {#each calendars as cal (cal.id)}
        <div class="cal-row" class:is-synced={cal.is_synced}>
          <div class="cal-left">
            <span class="cal-dot" style="background-color: {cal.background_color};"></span>
            <div class="cal-meta">
              <span class="cal-name">{cal.summary}</span>
              {#if cal.primary}
                <span class="primary-pill">Primary</span>
              {/if}
            </div>
          </div>

          <button
            class="btn-sync-select"
            class:active={cal.is_synced}
            onclick={() => handleSelectSyncedCalendar(cal.id)}
            title={cal.is_synced ? 'Click to deselect' : 'Set as Two-Way Sync Calendar'}
          >
            {#if cal.is_synced}
              <span class="active-badge">✓ Two-Way Synced</span>
            {:else}
              <span class="select-badge">Set as Sync Calendar</span>
            {/if}
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- 3. Academic Events Calendar -->
  {#if authStatus.is_signed_in && calendars.length > 0}
    <div class="group-heading">Academic Events Calendar</div>
    <div class="desc-text">
      Select a Google Calendar to sync academic events (exams, assignments, projects).
      Events on this calendar with titles matching <strong>Subject:Event</strong> or <strong>Subject - Event</strong> will be automatically imported.
    </div>

    <div class="cals-list">
      {#each calendars as cal (cal.id)}
        <div class="cal-row" class:is-synced={cal.is_events_synced}>
          <div class="cal-left">
            <span class="cal-dot" style="background-color: {cal.background_color};"></span>
            <div class="cal-meta">
              <span class="cal-name">{cal.summary}</span>
              {#if cal.primary}
                <span class="primary-pill">Primary</span>
              {/if}
            </div>
          </div>

          <button
            class="btn-sync-select"
            class:active={cal.is_events_synced}
            onclick={() => handleSelectEventsCalendar(cal.id)}
            title={cal.is_events_synced ? 'Click to deselect' : 'Set as Events Calendar'}
          >
            {#if cal.is_events_synced}
              <span class="active-badge">✓ Events Calendar</span>
            {:else}
              <span class="select-badge">Set as Events Calendar</span>
            {/if}
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- 4. Calendar Visibility & Overlays -->
  <div class="group-heading">Calendar Visibility</div>
  <div class="desc-text">
    Choose which calendars appear in the Planning timetable grid.
  </div>

  <SettingsToggle
    label="Local Calendar"
    description="Study blocks created locally and stored only on this device."
    checked={localVisible}
    onclick={handleToggleLocal}
  />

  {#if authStatus.is_signed_in && calendars.length > 0}
    {#each calendars as cal (cal.id)}
      <button
        type="button"
        class="cal-toggle-row"
        onclick={() => handleToggleCalendar(cal.id, cal.is_visible)}
      >
        <div class="cal-toggle-label">
          <span class="cal-dot" style="background-color: {cal.background_color};"></span>
          <div class="cal-toggle-info">
            <span class="cal-title">{cal.summary}</span>
            <span class="cal-sub">{cal.is_synced ? 'Two-way synced calendar' : 'Overlay calendar'}</span>
          </div>
        </div>

        <span class="toggle" class:on={cal.is_visible} aria-checked={cal.is_visible} role="switch"></span>
      </button>
    {/each}
  {/if}

  <!-- 4. OAuth Credentials -->
  <div class="group-heading">Google Cloud OAuth Credentials</div>
  <div class="desc-text">
    Google requires a registered OAuth Client ID to authenticate from your desktop browser.
  </div>

  <div class="credentials-card">
    <div class="instructions-header">
      <button
        class="instructions-toggle"
        onclick={() => (showInstructions = !showInstructions)}
        type="button"
      >
        <span>{showInstructions ? '▼' : '▶'} How to set up Google Cloud credentials</span>
      </button>
      <button class="btn-console-link" onclick={openGoogleConsole} type="button">
        Open Google Cloud Console ↗
      </button>
    </div>

    {#if showInstructions}
      <div class="instructions-body">
        <ol>
          <li>Open <strong>Google Cloud Console → APIs & Services → Credentials</strong>.</li>
          <li>Ensure the <strong>Google Calendar API</strong> is enabled under <strong>APIs & Services → Enabled APIs</strong>.</li>
          <li>Click <strong>Create Credentials → OAuth client ID</strong>.</li>
          <li>Select Application type: <strong>Desktop app</strong>.</li>
          <li>Copy the generated <strong>Client ID</strong> and paste it below. (Client Secret is optional for desktop apps).</li>
        </ol>
      </div>
    {/if}

    <div class="form-row">
      <label for="gcal-client-id" class="input-label">
        Client ID <span class="required">*</span>
      </label>
      <input
        id="gcal-client-id"
        class="text-input"
        type="text"
        bind:value={clientId}
        placeholder="e.g. 123456789-abcdef.apps.googleusercontent.com"
        disabled={isSavingCreds}
      />
    </div>

    <div class="form-row">
      <label for="gcal-client-secret" class="input-label">
        Client Secret <span class="optional-label">(optional)</span>
      </label>
      <input
        id="gcal-client-secret"
        class="text-input"
        type="password"
        bind:value={clientSecret}
        placeholder={authStatus.has_client_secret ? '••••••••  (Saved)' : 'Leave blank if not needed'}
        disabled={isSavingCreds}
      />
    </div>

    {#if credsErrorMsg}
      <div class="error-notice">{credsErrorMsg}</div>
    {/if}
    {#if credsSavedMsg}
      <div class="feedback-notice success">{credsSavedMsg}</div>
    {/if}

    <div class="credentials-footer">
      <button
        class="save-creds-btn"
        onclick={handleSaveCredentials}
        disabled={isSavingCreds}
        type="button"
      >
        {isSavingCreds ? 'Saving...' : 'Save Credentials'}
      </button>
    </div>
  </div>
</div>

<style>
  .section {
    display: flex;
    flex-direction: column;
    padding-bottom: 30px;
  }

  .group-heading {
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.7;
    padding: 20px 20px 8px;
  }

  .desc-text {
    font-size: 0.8rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.75;
    padding: 0 20px 12px;
    line-height: 1.45;
  }

  /* ── Account Card ────────────────────────────────────────── */
  .account-card {
    background: var(--color-hover, rgba(255, 255, 255, 0.03));
    border: 1px solid color-mix(in oklch, var(--color-foreground) 12%, transparent);
    border-radius: 6px;
    margin: 0 20px 10px;
    padding: 14px 16px;
  }

  .account-status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .account-info {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--color-foreground-darker, #71717a);
    flex-shrink: 0;
  }

  .status-dot.connected {
    background: #22c55e;
    box-shadow: 0 0 8px rgba(34, 197, 94, 0.5);
  }

  .account-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .account-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--color-foreground);
  }

  .account-subtitle {
    font-size: 0.78rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.7;
  }

  .account-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .action-btn {
    border-radius: 5px;
    font-size: 0.8rem;
    font-weight: 500;
    padding: 6px 14px;
    cursor: pointer;
    transition: background 0.15s, opacity 0.15s;
    border: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .connect-btn {
    background: var(--color-accent, #4285f4);
    color: #fff;
    font-weight: 600;
  }

  .connect-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .sync-btn {
    background: color-mix(in oklch, var(--color-accent, #4285f4) 15%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-accent, #4285f4) 30%, transparent);
    color: var(--color-accent, #4285f4);
  }

  .sync-btn:hover:not(:disabled) {
    background: color-mix(in oklch, var(--color-accent, #4285f4) 25%, transparent);
  }

  .disconnect-btn {
    background: transparent;
    border: 1px solid color-mix(in oklch, var(--color-foreground) 20%, transparent);
    color: var(--color-foreground-darker, var(--color-foreground));
  }

  .disconnect-btn:hover {
    color: #ef4444;
    border-color: rgba(239, 68, 68, 0.4);
  }

  .btn-icon.spinning {
    display: inline-block;
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

  .error-notice {
    margin-top: 10px;
    padding: 8px 12px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 4px;
    font-size: 0.78rem;
    color: #f87171;
  }

  .feedback-notice {
    margin-top: 10px;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    font-size: 0.78rem;
    color: var(--color-foreground);
  }

  .feedback-notice.success {
    background: rgba(34, 197, 94, 0.1);
    border-color: rgba(34, 197, 94, 0.3);
    color: #22c55e;
  }

  /* ── Calendars List ───────────────────────────────────────── */
  .cals-list {
    margin: 0 20px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .cal-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--color-hover, rgba(255, 255, 255, 0.03));
    border: 1px solid color-mix(in oklch, var(--color-foreground) 10%, transparent);
    border-radius: 6px;
    transition: border-color 0.15s;
  }

  .cal-row.is-synced {
    border-color: var(--color-accent, #4285f4);
    background: color-mix(in oklch, var(--color-accent, #4285f4) 8%, transparent);
  }

  .cal-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .cal-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .cal-meta {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .cal-name {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--color-foreground);
  }

  .primary-pill {
    font-size: 0.68rem;
    padding: 2px 6px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    color: var(--color-foreground-darker, var(--color-foreground));
  }

  .btn-sync-select {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .active-badge {
    font-size: 0.75rem;
    font-weight: 600;
    color: #22c55e;
    background: rgba(34, 197, 94, 0.15);
    padding: 4px 10px;
    border-radius: 4px;
    display: inline-block;
  }

  .select-badge {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.7;
    background: rgba(255, 255, 255, 0.06);
    padding: 4px 10px;
    border-radius: 4px;
    transition: opacity 0.15s, color 0.15s;
    display: inline-block;
  }

  .select-badge:hover {
    opacity: 1;
    color: var(--color-foreground);
  }

  /* ── Calendar Toggle Row ─────────────────────────────────── */
  .cal-toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 10px 20px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--color-separator);
    cursor: pointer;
    text-align: left;
    transition: background 0.12s;
  }

  .cal-toggle-row:hover {
    background: var(--color-hover);
  }

  /* Pill toggle */
  .toggle {
    position: relative;
    width: 34px;
    height: 18px;
    border-radius: 9px;
    background: color-mix(in oklch, var(--color-foreground) 22%, transparent);
    flex-shrink: 0;
    transition: background 0.2s;
  }

  .toggle::after {
    content: '';
    position: absolute;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-foreground-darker, var(--color-foreground));
    top: 3px;
    left: 3px;
    transition:
      transform 0.2s,
      background 0.2s;
  }

  .toggle.on {
    background: var(--color-accent);
  }

  .toggle.on::after {
    transform: translateX(16px);
    background: var(--color-background);
  }

  .cal-toggle-label {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .cal-toggle-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .cal-title {
    font-size: 0.85rem;
    color: var(--color-foreground);
  }

  .cal-sub {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.65;
  }

  /* ── Credentials Card ────────────────────────────────────── */
  .credentials-card {
    background: var(--color-hover, rgba(255, 255, 255, 0.03));
    border: 1px solid color-mix(in oklch, var(--color-foreground) 12%, transparent);
    border-radius: 6px;
    margin: 0 20px 20px;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .instructions-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .instructions-toggle {
    background: none;
    border: none;
    color: var(--color-accent, #4285f4);
    font-size: 0.8rem;
    cursor: pointer;
    padding: 0;
    text-align: left;
  }

  .btn-console-link {
    background: none;
    border: none;
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.8;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .btn-console-link:hover {
    opacity: 1;
    text-decoration: underline;
  }

  .instructions-body {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 6px;
    padding: 10px 14px;
    font-size: 0.75rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    line-height: 1.5;
  }

  .instructions-body ol {
    margin: 0;
    padding-left: 18px;
  }

  .instructions-body li {
    margin-bottom: 4px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-label {
    font-size: 0.8rem;
    color: var(--color-foreground);
    font-weight: 500;
  }

  .required {
    color: #ef4444;
  }

  .optional-label {
    font-weight: 400;
    opacity: 0.6;
    font-size: 0.75rem;
  }

  .text-input {
    background: var(--color-hover);
    border: 1px solid color-mix(in oklch, var(--color-foreground) 18%, transparent);
    border-radius: 4px;
    color: var(--color-foreground);
    font-size: 0.82rem;
    font-family: monospace;
    padding: 6px 10px;
    outline: none;
    transition: border-color 0.15s;
    width: 100%;
    box-sizing: border-box;
  }

  .text-input:focus {
    border-color: var(--color-accent, #4285f4);
  }

  .credentials-footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 4px;
  }

  .save-creds-btn {
    background: color-mix(in oklch, var(--color-accent, #4285f4) 15%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-accent, #4285f4) 35%, transparent);
    border-radius: 5px;
    color: var(--color-accent, #4285f4);
    font-size: 0.8rem;
    font-weight: 600;
    padding: 6px 16px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }

  .save-creds-btn:hover:not(:disabled) {
    background: color-mix(in oklch, var(--color-accent, #4285f4) 25%, transparent);
  }

  .save-creds-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
