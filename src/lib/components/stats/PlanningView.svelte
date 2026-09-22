<script lang="ts">
  import { onMount } from 'svelte';
  import {
    subjectsGetAll,
    scheduleGetAll,
    scheduleAddBlock,
    scheduleDeleteBlock,
    scheduleUpdateBlock,
    googleCalendarGetStatus,
    googleCalendarAuthStart,
    googleCalendarSignOut,
    googleCalendarGetCalendars,
    googleCalendarToggleVisibility,
    googleCalendarSetSyncedCalendar,
    googleCalendarSyncNow,
    googleCalendarGetOverlayEvents,
    calendarGetLocalVisible,
    calendarSetLocalVisible,
  } from '$lib/ipc';
  import type { SubjectStats, ScheduledBlock, GoogleAuthStatus, GoogleCalendarItem, GoogleOverlayEvent } from '$lib/types';
  import { error as logError } from '@tauri-apps/plugin-log';
  import { settings } from '$lib/stores/settings';
  import WeeklyCalendar from './WeeklyCalendar.svelte';
  import { openSettingsWindow } from '$lib/utils/windows';

  let subjects = $state<SubjectStats[]>([]);
  let blocks = $state<ScheduledBlock[]>([]);
  let loading = $state(true);

  // Calendar states
  let showLocalCalendar = $state(true);
  let authStatus = $state<GoogleAuthStatus>({
    is_signed_in: false,
    email: null,
    client_id: null,
    has_client_secret: false,
  });
  let calendars = $state<GoogleCalendarItem[]>([]);
  let overlayEvents = $state<GoogleOverlayEvent[]>([]);
  let syncedCalendar = $derived(calendars.find(c => c.is_synced) || null);
  let showSyncedCalendar = $derived(
    syncedCalendar ? syncedCalendar.is_visible : calendars.some(c => c.is_visible)
  );

  let weekOffset = $state(0);
  let isSigningIn = $state(false);
  let isSyncing = $state(false);
  let loadingCals = $state(false);
  let calError = $state<string | null>(null);

  function getMondayYmd(offsetWeeks: number = 0): string {
    const d = new Date();
    const day = d.getDay();
    const diffToMonday = (day + 6) % 7;
    d.setDate(d.getDate() - diffToMonday + (offsetWeeks * 7));
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const dayNum = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${dayNum}`;
  }

  let currentMondayYmd = $derived(getMondayYmd(weekOffset));

  onMount(() => {
    let mounted = true;

    const handleFocus = async () => {
      try {
        const [localVis, gStatus] = await Promise.all([
          calendarGetLocalVisible().catch(() => true),
          googleCalendarGetStatus().catch(() => authStatus),
        ]);
        if (!mounted) return;
        showLocalCalendar = localVis;
        const wasSignedIn = authStatus.is_signed_in;
        authStatus = gStatus;
        if (gStatus.is_signed_in) {
          await loadGoogleData();
          if (syncedCalendar) {
            blocks = await googleCalendarSyncNow(currentMondayYmd);
          }
        } else if (wasSignedIn && !gStatus.is_signed_in) {
          calendars = [];
          overlayEvents = [];
          blocks = await scheduleGetAll();
        }
      } catch (e) {
        logError(`Failed to refresh on window focus: ${e}`);
      }
    };

    window.addEventListener('focus', handleFocus);

    (async () => {
      try {
        const [subjectsData, blocksData, localVis, gStatus] = await Promise.all([
          subjectsGetAll(),
          scheduleGetAll(),
          calendarGetLocalVisible().catch(() => true),
          googleCalendarGetStatus().catch(() => ({
            is_signed_in: false,
            email: null,
            client_id: null,
            has_client_secret: false,
          })),
        ]);
        if (!mounted) return;

        subjects = subjectsData.sort((a, b) => {
          const aHasGoal = a.weekly_goal != null;
          const bHasGoal = b.weekly_goal != null;
          if (aHasGoal && !bHasGoal) return -1;
          if (!aHasGoal && bHasGoal) return 1;
          return a.name.localeCompare(b.name);
        });
        blocks = blocksData;
        showLocalCalendar = localVis;
        authStatus = gStatus;

        if (gStatus.is_signed_in) {
          await loadGoogleData();
        }
      } catch (e) {
        logError(`Failed to load planning data: ${e}`);
      } finally {
        if (mounted) loading = false;
      }
    })();

    return () => {
      mounted = false;
      window.removeEventListener('focus', handleFocus);
    };
  });

  async function loadGoogleData() {
    loadingCals = true;
    calError = null;
    try {
      calendars = await googleCalendarGetCalendars();
      if (calendars.length > 0 && !calendars.some(c => c.is_synced)) {
        const primary = calendars.find(c => c.primary) || calendars[0];
        if (primary) {
          await googleCalendarSetSyncedCalendar(primary.id);
          calendars = calendars.map(c => ({
            ...c,
            is_synced: c.id === primary.id,
            is_visible: c.id === primary.id ? true : c.is_visible
          }));
        }
      }
      overlayEvents = await googleCalendarGetOverlayEvents(currentMondayYmd);
    } catch (e: any) {
      const msg = e?.message || String(e);
      calError = msg;
      logError(`Failed to load Google Calendar data: ${msg}`);
    } finally {
      loadingCals = false;
    }
  }

  async function handleToggleLocalCalendar() {
    showLocalCalendar = !showLocalCalendar;
    try {
      await calendarSetLocalVisible(showLocalCalendar);
      if (!showLocalCalendar && !syncedCalendar && calendars.length > 0) {
        const gCal = calendars.find(c => c.is_visible) || calendars[0];
        if (gCal) {
          await handleSelectSyncedCalendar(gCal.id);
        }
      }
    } catch (e) {
      logError(`Failed to save local calendar visibility: ${e}`);
    }
  }

  async function handleSignInGoogle() {
    if (!authStatus.client_id) {
      await openSettingsWindow('calendar');
      return;
    }
    isSigningIn = true;
    try {
      await googleCalendarAuthStart();
      authStatus = await googleCalendarGetStatus();
      await loadGoogleData();
      if (syncedCalendar) {
        await handleSyncNow();
      }
    } catch (e) {
      logError(`Google auth error: ${e}`);
      alert(`Google sign in failed: ${e}`);
    } finally {
      isSigningIn = false;
    }
  }

  async function handleSignOutGoogle() {
    try {
      await googleCalendarSignOut();
      authStatus = {
        is_signed_in: false,
        email: null,
        client_id: authStatus.client_id,
        has_client_secret: authStatus.has_client_secret,
      };
      calendars = [];
      overlayEvents = [];
      blocks = await scheduleGetAll();
    } catch (e) {
      logError(`Sign out error: ${e}`);
    }
  }

  async function handleToggleCalendar(calId: string, currentVal: boolean) {
    const newVal = !currentVal;
    calendars = calendars.map(c => c.id === calId ? { ...c, is_visible: newVal } : c);

    if (newVal && !showLocalCalendar && !syncedCalendar) {
      await handleSelectSyncedCalendar(calId);
      return;
    }

    try {
      await googleCalendarToggleVisibility(calId, newVal);
      overlayEvents = await googleCalendarGetOverlayEvents(currentMondayYmd);
    } catch (e) {
      logError(`Toggle calendar visibility failed: ${e}`);
    }
  }

  async function handleSelectSyncedCalendar(calId: string) {
    const isCurrentlySynced = calendars.find(c => c.id === calId)?.is_synced;
    const targetId = isCurrentlySynced ? null : calId;

    calendars = calendars.map(c => ({
      ...c,
      is_synced: targetId === c.id,
      is_visible: targetId === c.id ? true : c.is_visible
    }));

    try {
      await googleCalendarSetSyncedCalendar(targetId);
      if (targetId) {
        await handleSyncNow();
      }
    } catch (e) {
      logError(`Set synced calendar failed: ${e}`);
    }
  }

  async function handleSyncNow() {
    if (!authStatus.is_signed_in) return;
    isSyncing = true;
    try {
      blocks = await googleCalendarSyncNow(currentMondayYmd);
      overlayEvents = await googleCalendarGetOverlayEvents(currentMondayYmd);
    } catch (e) {
      logError(`Sync failed: ${e}`);
    } finally {
      isSyncing = false;
    }
  }

  async function handleWeekChange(newMondayYmd: string, offset: number) {
    weekOffset = offset;
    if (authStatus.is_signed_in) {
      try {
        overlayEvents = await googleCalendarGetOverlayEvents(newMondayYmd);
        if (syncedCalendar) {
          blocks = await googleCalendarSyncNow(newMondayYmd);
        }
      } catch (e) {
        logError(`Failed to update events for week: ${e}`);
      }
    }
  }

  async function handleBlockAdd(day: number, startMin: number, endMin: number, subject: string) {
    try {
      let calType: 'google' | 'local' = 'local';
      let targetGCal = syncedCalendar;

      if (!showLocalCalendar) {
        calType = 'google';
        if (!targetGCal) {
          targetGCal = calendars.find(c => c.is_visible) || calendars[0] || null;
          if (targetGCal) {
            await handleSelectSyncedCalendar(targetGCal.id);
          }
        }
      } else if (targetGCal && targetGCal.is_visible) {
        calType = 'google';
      }

      if (calType === 'google' && targetGCal && !targetGCal.is_visible) {
        await handleToggleCalendar(targetGCal.id, false);
      } else if (calType === 'local' && !showLocalCalendar) {
        showLocalCalendar = true;
        await calendarSetLocalVisible(true);
      }

      const id = await scheduleAddBlock(
        subject,
        day,
        startMin,
        endMin,
        null,
        null,
        null,
        calType,
        currentMondayYmd
      );

      blocks = [...blocks, { 
        id, 
        subject, 
        day_of_week: day, 
        start_minute: startMin, 
        end_minute: endMin,
        subject_topic: null,
        study_type: null,
        round_tags: null,
        calendar_type: calType,
        google_calendar_id: calType === 'google' ? (targetGCal?.id || null) : null,
        google_event_id: null,
      }];
    } catch (e) {
      logError(`Failed to add block: ${e}`);
      alert(`Failed to add block: ${e}`);
    }
  }

  async function handleBlockDelete(id: number) {
    const prevBlocks = blocks;
    blocks = blocks.filter(b => b.id !== id);
    try {
      await scheduleDeleteBlock(id);
    } catch (e) {
      blocks = prevBlocks;
      logError(`Failed to delete block: ${e}`);
      alert(`Failed to delete block: ${e}`);
    }
  }

  async function handleBlockUpdate(id: number, day: number, startMin: number, endMin: number) {
    const prevBlocks = blocks;
    blocks = blocks.map(b => b.id === id ? { ...b, day_of_week: day, start_minute: startMin, end_minute: endMin } : b);
    try {
      await scheduleUpdateBlock(
        id,
        day,
        startMin,
        endMin,
        null,
        null,
        null,
        currentMondayYmd
      );
    } catch (e) {
      blocks = prevBlocks;
      logError(`Failed to update block: ${e}`);
      alert(`Failed to update block: ${e}`);
    }
  }

  function calculateAllocatedRounds(subjectName: string): number {
    const subjectBlocks = blocks.filter(b => {
      if (b.subject !== subjectName) return false;
      if (b.calendar_type === 'google') return true;
      return showLocalCalendar;
    });
    if (subjectBlocks.length === 0) return 0;
    
    const workMins = $settings.time_work_secs / 60;
    const shortBreakMins = $settings.short_breaks_enabled ? ($settings.time_short_break_secs / 60) : 0;
    const longBreakMins = $settings.long_breaks_enabled ? ($settings.time_long_break_secs / 60) : 0;
    const interval = $settings.long_break_interval;
    
    let totalRounds = 0;

    for (const block of subjectBlocks) {
      let remainingMins = block.end_minute - block.start_minute;
      let roundsInBlock = 0;
      let cyclePosition = 1;

      while (remainingMins >= workMins) {
        remainingMins -= workMins;
        roundsInBlock++;
        if (remainingMins <= 0) break;

        if (interval > 0 && cyclePosition % interval === 0) {
          remainingMins -= longBreakMins;
          cyclePosition = 1;
        } else {
          remainingMins -= shortBreakMins;
          cyclePosition++;
        }
      }
      
      totalRounds += roundsInBlock;
    }
    
    return totalRounds;
  }

  function handleDragStart(e: DragEvent, subject: SubjectStats) {
    if (e.dataTransfer) {
      e.dataTransfer.setData('application/json', JSON.stringify({ type: 'subject', data: subject.name }));
      e.dataTransfer.effectAllowed = 'copy';

      const dragGhost = document.createElement('div');
      dragGhost.textContent = subject.name;
      dragGhost.style.backgroundColor = 'var(--color-focus-round)';
      dragGhost.style.color = 'var(--color-background)';
      dragGhost.style.padding = '4px 12px';
      dragGhost.style.borderRadius = '4px';
      dragGhost.style.fontSize = '12px';
      dragGhost.style.fontWeight = '600';
      dragGhost.style.boxShadow = '0 2px 8px rgba(0,0,0,0.2)';
      dragGhost.style.position = 'absolute';
      dragGhost.style.top = '-1000px';
      dragGhost.style.left = '-1000px';
      dragGhost.style.opacity = '0.9';
      
      document.body.appendChild(dragGhost);
      e.dataTransfer.setDragImage(dragGhost, 10, 10);

      setTimeout(() => {
        if (document.body.contains(dragGhost)) {
          document.body.removeChild(dragGhost);
        }
      }, 0);
    }
  }
</script>

<div class="planning-view">
  <!-- Left Sidebar: Calendars & Subjects -->
  <aside class="sidebar">
    <!-- Section 1: Calendars -->
    <div class="sidebar-section calendars-section">
      <div class="section-header">
        <span class="section-title">Calendars</span>
        <button
          class="btn-icon"
          onclick={() => openSettingsWindow('calendar')}
          title="Configure Calendar in Settings"
          aria-label="Configure Calendar in Settings"
        >
          ⚙️
        </button>
      </div>

      <div class="calendars-list">
        <!-- Local Calendar -->
        <div class="calendar-item">
          <label class="cal-checkbox-label">
            <input
              type="checkbox"
              checked={showLocalCalendar}
              onchange={handleToggleLocalCalendar}
            />
            <span class="cal-dot local-dot"></span>
            <span class="cal-name">Local</span>
          </label>
          <span class="cal-count">
            {blocks.filter(b => b.calendar_type !== 'google').length}
          </span>
        </div>

        <!-- Google Calendar Section -->
        {#if !authStatus.is_signed_in}
          <div class="google-auth-card">
            <div class="google-card-header">
              <span class="google-icon">📅</span>
              <span class="google-title">Google Calendar</span>
            </div>
            <p class="google-card-desc">Sign in via browser to sync study timetable and display overlays.</p>
            <button
              class="btn-google-signin"
              onclick={handleSignInGoogle}
              disabled={isSigningIn}
            >
              {#if isSigningIn}Connecting...{:else}Sign in with Google{/if}
            </button>
            <button
              class="btn-settings-link"
              onclick={() => openSettingsWindow('calendar')}
              type="button"
            >
              Configure in Settings
            </button>
          </div>
        {:else}
          <div class="google-account-header">
            <div class="account-info">
              <span class="user-email" title={authStatus.email || ''}>{authStatus.email || 'Connected'}</span>
            </div>
            <div class="account-actions">
              <button
                class="btn-icon"
                onclick={loadGoogleData}
                disabled={loadingCals}
                title="Refresh calendar list"
                class:spinning={loadingCals}
              >
                🔄
              </button>
              <button
                class="btn-icon"
                onclick={handleSyncNow}
                disabled={isSyncing}
                title="Sync two-way calendar now"
                class:spinning={isSyncing}
              >
                ⚡
              </button>
              <button
                class="btn-icon text-btn"
                onclick={handleSignOutGoogle}
                title="Disconnect Google Account"
              >
                Sign out
              </button>
            </div>
          </div>

          <div class="google-calendars-sublist">
            {#if loadingCals && calendars.length === 0}
              <div class="calendars-status">Loading calendars...</div>
            {:else if calendars.length === 0}
              <div class="calendars-empty-notice">
                <p class="empty-notice-title">No calendars found</p>
                <p class="empty-notice-desc">
                  {#if calError}
                    {calError}
                  {:else}
                    If you just connected, re-authenticate to grant calendar list permissions.
                  {/if}
                </p>
                <div class="notice-actions">
                  <button class="btn-notice-action primary" onclick={handleSignInGoogle} disabled={isSigningIn}>
                    🔑 {isSigningIn ? 'Connecting...' : 'Re-authenticate with Google'}
                  </button>
                  <button class="btn-notice-action" onclick={loadGoogleData} disabled={loadingCals}>
                    🔄 Retry Loading
                  </button>
                </div>
              </div>
            {:else}
              {#if syncedCalendar}
                <div class="active-sync-info">
                  <span class="sync-icon-small">⇄</span>
                  <span>Syncing sessions to: <strong>{syncedCalendar.summary}</strong></span>
                </div>
              {/if}
              {#each calendars as cal (cal.id)}
                <div class="calendar-item google-cal-item" class:is-synced={cal.is_synced}>
                  <label class="cal-checkbox-label">
                    <input
                      type="checkbox"
                      checked={cal.is_visible}
                      onchange={() => handleToggleCalendar(cal.id, cal.is_visible)}
                    />
                    <span class="cal-dot" style="background-color: {cal.background_color};"></span>
                    <span class="cal-name" title={cal.summary}>{cal.summary}</span>
                  </label>

                  <!-- Two-way sync button / badge -->
                  <button
                    class="btn-sync-toggle"
                    class:active={cal.is_synced}
                    onclick={() => handleSelectSyncedCalendar(cal.id)}
                    title={cal.is_synced ? 'Click to deselect Two-Way Sync' : 'Click to select as Two-Way Sync calendar'}
                  >
                    {#if cal.is_synced}
                      <span class="sync-badge">✓ 2-way</span>
                    {:else}
                      <span class="make-sync-btn">Set 2-way</span>
                    {/if}
                  </button>
                </div>
              {/each}
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- Section 2: Subjects -->
    <div class="sidebar-section subjects-section">
      <div class="section-header">
        <span class="section-title">Subjects</span>
      </div>

      <div class="subjects-list">
        {#if loading}
          <div class="empty">Loading...</div>
        {:else if subjects.length === 0}
          <div class="empty">No subjects available.</div>
        {:else}
          {#each subjects as subject (subject.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div 
              class="subject-item"
              draggable="true"
              ondragstart={(e) => handleDragStart(e, subject)}
            >
              <div class="subject-main">
                <span class="subject-name">{subject.name}</span>
                <span class="drag-handle" title="Drag onto calendar">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="9" cy="5" r="1"/>
                    <circle cx="9" cy="12" r="1"/>
                    <circle cx="9" cy="19" r="1"/>
                    <circle cx="15" cy="5" r="1"/>
                    <circle cx="15" cy="12" r="1"/>
                    <circle cx="15" cy="19" r="1"/>
                  </svg>
                </span>
              </div>
              
              {#if subject.weekly_goal}
                {@const mockAllocated = calculateAllocatedRounds(subject.name)} 
                {@const goal = subject.weekly_goal}
                {@const overAllocated = mockAllocated > goal}
                <div class="allocation">
                  <div class="allocation-label">
                    <span class="allocated-text" class:over={overAllocated}>
                      {mockAllocated} / {goal} <span class="unit">rounds allocated</span>
                    </span>
                  </div>
                  <div class="progress-track" class:over={overAllocated}>
                    <div 
                      class="progress-fill" 
                      class:over={overAllocated}
                      style="width: {Math.min(100, (mockAllocated / goal) * 100)}%"
                    ></div>
                  </div>
                </div>
              {:else}
                <div class="no-goal">No weekly goal set</div>
              {/if}
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </aside>

  <!-- Right Main Area: Weekly Calendar -->
  <main class="calendar-area">
    <WeeklyCalendar 
      {blocks}
      {overlayEvents}
      {showLocalCalendar}
      {showSyncedCalendar}
      syncedCalendarSummary={syncedCalendar?.summary || null}
      {weekOffset}
      {isSyncing}
      onBlockAdd={handleBlockAdd}
      onBlockDelete={handleBlockDelete}
      onBlockUpdate={handleBlockUpdate}
      onWeekChange={handleWeekChange}
      onSyncClick={handleSyncNow}
    />
  </main>
</div>

<style>
  .planning-view {
    display: flex;
    height: 100%;
    gap: 1rem;
    padding: 1rem;
    color: var(--color-text);
  }

  /* ── Sidebar ───────────────────────────────────── */
  .sidebar {
    width: 270px;
    display: flex;
    flex-direction: column;
    background: transparent;
    border-right: 1px solid var(--color-separator);
    overflow: hidden;
  }

  .sidebar-section {
    display: flex;
    flex-direction: column;
  }

  .calendars-section {
    border-bottom: 1px solid var(--color-separator);
    padding-bottom: 10px;
    max-height: 48%;
    flex-shrink: 0;
  }

  .subjects-section {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    padding-top: 10px;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
  }

  .section-title {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 700;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .btn-icon {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 0.9rem;
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .btn-icon:hover {
    color: var(--color-foreground, #fff);
  }

  .text-btn {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, #a1a1aa);
  }

  .text-btn:hover {
    color: #ef4444;
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .calendars-list {
    padding: 4px 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
  }

  .calendar-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px;
    border-radius: 6px;
    background: transparent;
    font-size: 0.85rem;
  }

  .calendar-item:hover {
    background: var(--color-hover, rgba(255, 255, 255, 0.04));
  }

  .cal-checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    overflow: hidden;
    flex: 1;
  }

  .cal-checkbox-label input {
    cursor: pointer;
    margin: 0;
  }

  .cal-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .local-dot {
    background: var(--color-focus-round, #4285f4);
  }

  .cal-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cal-count {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, #a1a1aa);
    padding-left: 6px;
  }

  .google-auth-card {
    background: var(--color-background-light, rgba(255, 255, 255, 0.03));
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.08));
    border-radius: 8px;
    padding: 10px;
    margin-top: 4px;
  }

  .google-card-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    font-size: 0.85rem;
    margin-bottom: 4px;
  }

  .google-card-desc {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, #a1a1aa);
    margin: 0 0 8px 0;
    line-height: 1.3;
  }

  .btn-google-signin {
    width: 100%;
    background: var(--color-focus-round, #4285f4);
    border: none;
    color: #fff;
    font-weight: 600;
    font-size: 0.8rem;
    padding: 6px 10px;
    border-radius: 6px;
    cursor: pointer;
  }

  .btn-google-signin:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-google-signin:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-settings-link {
    width: 100%;
    background: transparent;
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.15));
    color: var(--color-foreground-darker, #a1a1aa);
    font-size: 0.75rem;
    font-weight: 500;
    padding: 5px 10px;
    border-radius: 6px;
    cursor: pointer;
    margin-top: 6px;
    transition: color 0.15s, border-color 0.15s;
  }

  .btn-settings-link:hover {
    color: var(--color-foreground, #fff);
    border-color: var(--color-foreground-darker, #a1a1aa);
  }

  .google-account-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px;
    background: var(--color-background-light, rgba(255, 255, 255, 0.02));
    border-radius: 6px;
    margin-top: 4px;
  }

  .user-email {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, #a1a1aa);
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }

  .account-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .google-calendars-sublist {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 4px;
  }

  .active-sync-info {
    font-size: 0.72rem;
    color: var(--color-foreground-darker, #a1a1aa);
    padding: 5px 8px;
    background: rgba(66, 133, 244, 0.1);
    border: 1px solid rgba(66, 133, 244, 0.2);
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 4px;
    line-height: 1.3;
  }

  .active-sync-info strong {
    color: var(--color-text, #fff);
  }

  .sync-icon-small {
    color: #4285f4;
    font-weight: bold;
    font-size: 0.85rem;
  }

  .btn-sync-toggle {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    font-size: 0.7rem;
    border-radius: 4px;
  }

  .sync-badge {
    background: rgba(34, 197, 94, 0.2);
    color: #22c55e;
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 600;
  }

  .make-sync-btn {
    color: var(--color-foreground-darker, #a1a1aa);
    opacity: 0.5;
  }

  .make-sync-btn:hover {
    opacity: 1;
    color: var(--color-foreground, #fff);
  }

  .is-synced {
    background: rgba(34, 197, 94, 0.05);
  }

  .calendars-status {
    padding: 12px 8px;
    font-size: 0.8rem;
    color: var(--color-foreground-darker, #a1a1aa);
    text-align: center;
  }

  .calendars-empty-notice {
    padding: 10px;
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 6px;
    margin-top: 6px;
  }

  .empty-notice-title {
    font-size: 0.8rem;
    font-weight: 600;
    color: #f87171;
    margin: 0 0 4px 0;
  }

  .empty-notice-desc {
    font-size: 0.75rem;
    color: var(--color-foreground-darker, #a1a1aa);
    margin: 0 0 8px 0;
    line-height: 1.35;
  }

  .notice-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .btn-notice-action {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.15));
    color: var(--color-text, #fff);
    font-size: 0.75rem;
    font-weight: 500;
    padding: 5px 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s;
    text-align: center;
  }

  .btn-notice-action:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.15);
  }

  .btn-notice-action.primary {
    background: var(--color-focus-round, #4285f4);
    border-color: transparent;
    color: #fff;
    font-weight: 600;
  }

  .btn-notice-action.primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  /* ── Subjects List ─────────────────────────────── */
  .subjects-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .subject-item {
    padding: 0.75rem;
    border-radius: 6px;
    background: transparent;
    cursor: grab;
    transition: all 0.1s;
    position: relative;
  }

  .subject-item:hover {
    background: var(--color-hover);
  }

  .subject-item:active {
    cursor: grabbing;
  }

  .subject-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  .subject-name {
    font-weight: 500;
    font-size: 0.95rem;
  }

  .drag-handle {
    opacity: 0.3;
    display: flex;
    align-items: center;
  }

  .subject-item:hover .drag-handle {
    opacity: 0.8;
  }

  /* ── Progress Allocation ──────────────────────── */
  .allocation {
    display: flex;
    flex-direction: column;
  }

  .allocation-label {
    font-size: 0.75rem;
    color: var(--color-subtext);
  }

  .allocated-text {
    font-weight: 600;
  }

  .allocated-text.over {
    color: #ef4444;
  }

  .unit {
    font-weight: 400;
    opacity: 0.7;
  }

  .progress-track {
    position: absolute;
    bottom: 0;
    left: 12px;
    right: 12px;
    height: 2px;
    background: transparent;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-focus-round);
    border-radius: 2px;
    transition: width 0.3s;
  }

  .progress-fill.over {
    background: #ef4444;
  }

  .no-goal {
    font-size: 0.75rem;
    color: var(--color-subtext);
    font-style: italic;
    opacity: 0.7;
  }

  .empty {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--color-subtext);
    font-size: 0.85rem;
  }

  /* ── Main Area ─────────────────────────────────── */
  .calendar-area {
    flex: 1;
    background: var(--color-background);
    border-radius: 6px;
    display: flex;
    overflow: hidden;
  }
</style>
