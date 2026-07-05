<script lang="ts">
  import { onMount } from 'svelte';
  import { subjectsGetAll, subjectCreate, subjectDelete } from '$lib/ipc';
  import type { SubjectStats } from '$lib/types';
  import { error as logError } from '@tauri-apps/plugin-log';

  let subjects = $state<SubjectStats[]>([]);
  let newSubjectName = $state('');
  let loading = $state(true);

  async function loadSubjects() {
    try {
      loading = true;
      subjects = await subjectsGetAll();
    } catch (e) {
      logError(`Failed to load subjects: ${e}`);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadSubjects();
  });

  async function handleCreate() {
    const name = newSubjectName.trim();
    if (!name) return;
    try {
      await subjectCreate(name);
      newSubjectName = '';
      await loadSubjects();
    } catch (e) {
      logError(`Failed to create subject: ${e}`);
      alert(`Failed to create subject: ${e}`);
    }
  }

  async function handleDelete(name: string) {
    if (!confirm(`Are you sure you want to delete the subject "${name}"?`)) return;
    try {
      await subjectDelete(name);
      await loadSubjects();
    } catch (e) {
      logError(`Failed to delete subject: ${e}`);
      alert(`Failed to delete subject: ${e}`);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleCreate();
    }
  }
</script>

<div class="subjects-view">
  <div class="header">
    <div class="input-group">
      <input 
        type="text" 
        bind:value={newSubjectName} 
        placeholder="New subject name..." 
        onkeydown={handleKeyDown}
      />
      <button class="btn-create" onclick={handleCreate} disabled={!newSubjectName.trim()}>
        Add Subject
      </button>
    </div>
  </div>

  <div class="subjects-list">
    {#if loading}
      <div class="empty">Loading...</div>
    {:else if subjects.length === 0}
      <div class="empty">No subjects found.</div>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Subject</th>
            <th>Pomodoros</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each subjects as subject (subject.id)}
            <tr>
              <td class="name-col">{subject.name}</td>
              <td class="count-col">
                <span class="badge" class:zero={subject.pomodoro_count === 0}>
                  {subject.pomodoro_count}
                </span>
              </td>
              <td class="actions-col">
                <button 
                  class="btn-delete" 
                  disabled={subject.pomodoro_count > 0} 
                  onclick={() => handleDelete(subject.name)}
                  title={subject.pomodoro_count > 0 ? 'Cannot delete subject with existing pomodoros' : 'Delete subject'}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="3 6 5 6 21 6"></polyline>
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                    <line x1="10" y1="11" x2="10" y2="17"></line>
                    <line x1="14" y1="11" x2="14" y2="17"></line>
                  </svg>
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<style>
  .subjects-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 1rem;
    gap: 1rem;
    color: var(--color-text);
  }

  .header {
    display: flex;
    justify-content: flex-end;
  }

  .input-group {
    display: flex;
    gap: 0.5rem;
  }

  input {
    background: var(--color-background);
    border: 1px solid var(--color-subtext);
    color: var(--color-text);
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    font-size: 0.85rem;
    width: 200px;
    transition: border-color 0.2s;
  }
  
  input:focus {
    outline: none;
    border-color: var(--color-focus-round);
  }

  .btn-create {
    background: var(--color-focus-round);
    color: var(--color-background);
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .btn-create:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-create:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .subjects-list {
    flex: 1;
    overflow-y: auto;
    border: 1px solid var(--color-subtext);
    border-radius: 6px;
    background: var(--color-background);
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th, td {
    padding: 0.75rem 1rem;
    text-align: left;
    border-bottom: 1px solid var(--color-subtext);
    font-size: 0.9rem;
  }

  th {
    font-weight: 600;
    color: var(--color-text);
    background: var(--color-background);
    position: sticky;
    top: 0;
    z-index: 1;
    border-bottom: 2px solid var(--color-subtext);
  }

  .name-col {
    width: 60%;
    font-weight: 500;
  }

  .count-col {
    width: 20%;
  }

  .actions-col {
    width: 20%;
    text-align: right;
  }

  .badge {
    display: inline-block;
    background: var(--color-focus-round);
    color: var(--color-background);
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 0.8rem;
    font-weight: 600;
  }
  
  .badge.zero {
    background: var(--color-subtext);
    color: var(--color-background);
  }

  .btn-delete {
    background: transparent;
    border: none;
    color: var(--color-text);
    opacity: 0.6;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }

  .btn-delete:hover:not(:disabled) {
    opacity: 1;
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }

  .btn-delete:disabled {
    opacity: 0.2;
    cursor: not-allowed;
  }

  .empty {
    padding: 2rem;
    text-align: center;
    color: var(--color-subtext);
    font-size: 0.9rem;
  }
</style>
