<script lang="ts">
  import './CommandPalette.css';
  import { onMount } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { LogicalSize } from '@tauri-apps/api/dpi';
  import { emit } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { info, error as logError } from '@tauri-apps/plugin-log';
  import { getSessionSubjects, getSessionTopics, getSessionStudyTypes } from '$lib/ipc';
  import { parse, completeSuggestion, fieldLabel, type ParsedPalette, type ActiveField } from '$lib/utils/paletteParser';

  const STUDY_TYPES_FALLBACK = ['Exercise', 'Reading', 'Review', 'Classroom', 'Video', 'Flash Cards'];
  const COMMAND_SUGGESTIONS = ['focus'];
  const PALETTE_COLLAPSED_H = 62;
  const PALETTE_EXPANDED_H = 340;

  let input = $state('');
  let parsed = $derived(parse(input));
  let inputEl: HTMLInputElement | undefined;
  let dropdownEl: HTMLDivElement | undefined = $state();
  let activeIndex = $state(-1);

  // Autocomplete data
  let subjects = $state<string[]>([]);
  let topics = $state<string[]>([]);
  let studyTypes = $state<string[]>(STUDY_TYPES_FALLBACK);

  // Computed suggestions for the current active field
  let suggestions = $derived(getSuggestions(parsed));
  let filteredSuggestions = $derived(
    suggestions.filter(s => s.toLowerCase().includes(parsed.activeQuery.toLowerCase()))
  );
  let showDropdown = $derived(filteredSuggestions.length > 0 && input.length > 0);

  function getSuggestions(p: ParsedPalette): string[] {
    if (p.activeField === 'command') return COMMAND_SUGGESTIONS;
    if (p.activeField === 'subject') return subjects;
    if (p.activeField === 'topic') return topics;
    if (p.activeField === 'studyType') return studyTypes;
    return [];
  }
  // Load autocomplete data
  async function loadSuggestions(p: ParsedPalette) {
    if (p.activeField === 'subject') {
      subjects = await getSessionSubjects().catch(() => []);
    } else if (p.activeField === 'topic') {
      topics = await getSessionTopics(p.subject || undefined).catch(() => []);
    } else if (p.activeField === 'studyType') {
      const custom = await getSessionStudyTypes().catch(() => []);
      studyTypes = [...new Set([...STUDY_TYPES_FALLBACK, ...custom])];
    }
  }

  let lastActiveField: ActiveField | null = null;
  $effect(() => {
    if (parsed.activeField !== lastActiveField) {
      lastActiveField = parsed.activeField;
      loadSuggestions(parsed);
    }
  });

  function handleInput() {
    activeIndex = -1;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      close();
      return;
    }

    if (showDropdown) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        activeIndex = Math.min(activeIndex + 1, filteredSuggestions.length - 1);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        activeIndex = Math.max(activeIndex - 1, -1);
        return;
      }
      if ((e.key === 'Tab' || e.key === 'Enter') && activeIndex >= 0) {
        e.preventDefault();
        selectSuggestion(filteredSuggestions[activeIndex]);
        return;
      }
      if (e.key === 'Tab' && activeIndex === -1 && filteredSuggestions.length > 0) {
        e.preventDefault();
        selectSuggestion(filteredSuggestions[0]);
        return;
      }
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      if (showDropdown && activeIndex >= 0) {
        selectSuggestion(filteredSuggestions[activeIndex]);
      } else {
        submit();
      }
    }
  }

  function selectSuggestion(value: string) {
    input = completeSuggestion(input, parsed, value);
    activeIndex = -1;
    inputEl?.focus();
  }

  async function submit() {
    info(`SUBMIT CALLED: ${JSON.stringify(parsed)}`);
    if (!parsed.canSubmit) return;
    try {
      await invoke('palette_submit', {
        subject: parsed.subject,
        subjectTopic: parsed.topic,
        studyType: parsed.studyType,
        notes: parsed.note,
      });
    } catch (e) {
      logError(`Failed to submit palette: ${e}`);
    }
  }

  async function close() {
    try {
      await invoke('palette_close');
      input = '';
    } catch (e) {
      logError(`Failed to close palette: ${e}`);
      getCurrentWebviewWindow().hide();
      input = '';
    }
  }

  // Close on blur (click outside)
  function handleWindowBlur() {
    close();
  }

  onMount(() => {
    inputEl?.focus();
  });
</script>

<svelte:window onblur={handleWindowBlur} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="palette">
  <div class="input-row">
    <span class="prompt">&gt;</span>
    <input
      bind:this={inputEl}
      bind:value={input}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      class="cmd-input"
      type="text"
      spellcheck="false"
      autocomplete="off"
      placeholder="focus subject -t topic -s study-type -n note…"
      aria-label="Command palette input"
      aria-autocomplete="list"
      aria-controls="palette-dropdown"
      aria-activedescendant={activeIndex >= 0 ? `pal-item-${activeIndex}` : undefined}
    />
    {#if parsed.canSubmit}
      <!-- svelte-ignore a11y_consider_explicit_label -->
      <button class="submit-btn" onclick={submit}>
        <span class="hint-enter" aria-hidden="true">↵</span>
      </button>
    {/if}
  </div>

  {#if showDropdown}
    <div class="divider" role="separator"></div>
    <div
      id="palette-dropdown"
      class="dropdown"
      bind:this={dropdownEl}
      role="listbox"
      aria-label={fieldLabel(parsed.activeField)}
    >
      <div class="dropdown-section-label" aria-hidden="true">{fieldLabel(parsed.activeField)}</div>
      {#each filteredSuggestions.slice(0, 7) as suggestion, i}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          id="pal-item-{i}"
          class="dropdown-item"
          class:active={i === activeIndex}
          role="option"
          tabindex="-1"
          aria-selected={i === activeIndex}
          onmousedown={() => selectSuggestion(suggestion)}
          onmouseenter={() => activeIndex = i}
        >
          {@html highlightMatch(suggestion, parsed.activeQuery)}
        </div>
      {/each}
    </div>
  {/if}
</div>

<script lang="ts" module>
  function highlightMatch(text: string, query: string): string {
    if (!query) return escHtml(text);
    const idx = text.toLowerCase().indexOf(query.toLowerCase());
    if (idx === -1) return escHtml(text);
    return (
      escHtml(text.slice(0, idx)) +
      `<strong>${escHtml(text.slice(idx, idx + query.length))}</strong>` +
      escHtml(text.slice(idx + query.length))
    );
  }
  function escHtml(s: string) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }
</script>


