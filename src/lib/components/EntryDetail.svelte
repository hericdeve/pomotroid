<script lang="ts">
  import { onMount } from 'svelte';
  import { getSessionSubjects, getSessionTopics, getSessionStudyTypes } from '$lib/ipc';
  import type { UpdateSessionPayload } from '$lib/types';
  import AutocompleteInput from './AutocompleteInput.svelte';

  interface Props {
    payload?: UpdateSessionPayload;
    onsave?: (payload: UpdateSessionPayload) => void;
  }

  let {
    payload = $bindable({
      subject: '',
      subject_topic: '',
      study_type: 'None / Uncategorized',
      notes: ''
    }),
    onsave
  }: Props = $props();

  let subjectOptions = $state<string[]>([]);
  let topicOptions = $state<string[]>([]);
  
  const DEFAULT_STUDY_TYPES = [
    'None / Uncategorized',
    'Exercise',
    'Reading',
    'Review',
    'Classroom',
    'Video',
    'Flash Cards'
  ];
  let studyTypeOptions = $state<string[]>([...DEFAULT_STUDY_TYPES]);

  // Convert incoming nulls to defaults for form fields
  let localSubject = $state(payload.subject || '');
  let localTopic = $state(payload.subject_topic || '');
  let localStudyType = $state(payload.study_type || 'None / Uncategorized');
  let localNotes = $state(payload.notes || '');

  onMount(async () => {
    try {
      subjectOptions = await getSessionSubjects();
      if (localSubject) {
        topicOptions = await getSessionTopics(localSubject);
      } else {
        topicOptions = await getSessionTopics();
      }
      const fetchedTypes = await getSessionStudyTypes();
      if (fetchedTypes && fetchedTypes.length > 0) {
        // Merge fetched types with defaults, avoiding duplicates
        const unique = new Set([...DEFAULT_STUDY_TYPES, ...fetchedTypes]);
        studyTypeOptions = Array.from(unique);
      }
    } catch (err) {
      console.error('Failed to fetch subjects/topics:', err);
    }
  });

  async function handleSubjectCommit(val: string) {
    localSubject = val;
    try {
      if (val) {
        topicOptions = await getSessionTopics(val);
      } else {
        topicOptions = await getSessionTopics();
      }
    } catch (err) {
      console.error(err);
    }
    triggerSave();
  }
  
  function handleTopicCommit(val: string) {
    localTopic = val;
    triggerSave();
  }
  
  function handleTypeCommit(val: string) {
    localStudyType = val;
    handleBlur();
  }
  
  function handleTypeChange() {
    // The bind:value updates localStudyType before this fires
    triggerSave();
  }
  
  function handleNotesBlur() {
    triggerSave();
  }

  function triggerSave() {
    const finalPayload: UpdateSessionPayload = {
      subject: localSubject.trim() || null,
      subject_topic: localTopic.trim() || null,
      study_type: localStudyType === 'None / Uncategorized' ? null : localStudyType,
      notes: localNotes.trim() || null
    };
    // Sync back up if bound
    payload = finalPayload;
    onsave?.(finalPayload);
  }
</script>

<div class="entry-detail">
  <div class="form-group">
    <label>Subject</label>
    <AutocompleteInput
      bind:value={localSubject}
      options={subjectOptions}
      placeholder="e.g. Mathematics"
      oncommit={handleSubjectCommit}
    />
  </div>
  
  <div class="form-group">
    <label>Topic</label>
    <AutocompleteInput
      bind:value={localTopic}
      options={topicOptions}
      placeholder="e.g. Linear Algebra"
      oncommit={handleTopicCommit}
    />
  </div>

  <div class="form-group">
    <label>Study Type</label>
    <AutocompleteInput
      bind:value={localStudyType}
      options={studyTypeOptions}
      placeholder="e.g. Exercise"
      oncommit={handleTypeCommit}
    />
  </div>

  <div class="form-group">
    <label>Notes</label>
    <textarea 
      bind:value={localNotes} 
      placeholder="Any additional notes..." 
      onblur={handleNotesBlur}
    ></textarea>
  </div>
</div>

<style>
  .entry-detail {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    padding: 20px;
    background: var(--color-background);
    border-radius: 8px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  
  label {
    font-size: 0.85rem;
    color: var(--color-foreground-darker, var(--color-foreground));
    letter-spacing: 0.02em;
    font-weight: 500;
  }
  
  .custom-select, textarea {
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
    -webkit-appearance: none;
    appearance: none;
  }
  
  .custom-select:focus, textarea:focus {
    border-color: var(--color-accent);
    background: color-mix(in oklch, var(--color-foreground) 15%, transparent);
  }
  
  textarea {
    resize: vertical;
    min-height: 80px;
  }
  
  .custom-select option {
    background: var(--color-background-light);
    color: var(--color-foreground);
  }
</style>
