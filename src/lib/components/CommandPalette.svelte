<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { LogicalSize } from '@tauri-apps/api/dpi';
  import { emit } from '@tauri-apps/api/event';
  import { getSessionSubjects, getSessionTopics, getSessionStudyTypes } from '$lib/ipc';
  import { parse, completeSuggestion, fieldLabel, type ParsedPalette, type ActiveField } from '$lib/utils/paletteParser';

  const STUDY_TYPES_FALLBACK = ['Exercise', 'Reading', 'Review', 'Classroom', 'Video', 'Flash Cards'];
  const COMMAND_SUGGESTIONS = ['focus'];
  const PALETTE_COLLAPSED_H = 62;
  const PALETTE_EXPANDED_H = 340;

  let input = $state('');
  let parsed = $derived(parse(input));
  let inputEl: HTMLInputElement | undefined;
  let dropdownEl: HTMLDivElement | undefined;
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

  // Resize the window when dropdown opens/closes
  $effect(() => {
    const win = getCurrentWebviewWindow();
    if (showDropdown) {
      const itemH = Math.min(filteredSuggestions.length, 7) * 40 + 8;
      win.setSize(new LogicalSize(640, PALETTE_COLLAPSED_H + 8 + itemH));
    } else {
      win.setSize(new LogicalSize(640, PALETTE_COLLAPSED_H));
    }
  });

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
      submit();
    }
  }

  function selectSuggestion(value: string) {
    input = completeSuggestion(input, parsed, value);
    activeIndex = -1;
    inputEl?.focus();
  }

  async function submit() {
    if (!parsed.canSubmit) return;
    // Emit event to the main window which will set pending tags and start the timer
    await emit('palette:start', {
      subject: parsed.subject,
      subject_topic: parsed.topic,
      study_type: parsed.studyType,
      notes: parsed.note,
    });
    close();
  }

  function close() {
    getCurrentWebviewWindow().close();
  }

  // Close on blur (click outside)
  function handleWindowBlur() {
    close();
  }

  onMount(() => {
    inputEl?.focus();
    window.addEventListener('blur', handleWindowBlur);
    return () => window.removeEventListener('blur', handleWindowBlur);
  });

  // Chip data derived from committed values
  let chips = $derived(buildChips(parsed));

  function buildChips(p: ParsedPalette): { label: string; value: string }[] {
    const out: { label: string; value: string }[] = [];
    if (p.subject && p.activeField !== 'subject') out.push({ label: 'Subject', value: p.subject });
    if (p.topic) out.push({ label: 'Topic', value: p.topic });
    if (p.studyType) out.push({ label: 'Type', value: p.studyType });
    if (p.note) out.push({ label: 'Note', value: p.note });
    return out;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="palette" onkeydown={handleKeyDown}>
  <div class="input-row">
    <span class="prompt">&gt;</span>
    <div class="chips">
      {#each chips as chip}
        <span class="chip">
          <span class="chip-label">{chip.label}</span>
          <span class="chip-value">{chip.value}</span>
        </span>
      {/each}
    </div>
    <input
      bind:this={inputEl}
      bind:value={input}
      oninput={handleInput}
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
      <span class="hint-enter" aria-hidden="true">↵</span>
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

<style>
  .palette {
    display: flex;
    flex-direction: column;
    background: var(--color-background);
    border: 1px solid color-mix(in oklch, var(--color-foreground) 12%, transparent);
    border-radius: 12px;
    overflow: hidden;
    box-shadow:
      0 8px 32px color-mix(in oklch, black 40%, transparent),
      0 2px 8px color-mix(in oklch, black 20%, transparent);
    font-family: 'Mona Sans', system-ui, sans-serif;
    height: 100dvh;
  }

  .input-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 16px;
    height: 62px;
    flex-shrink: 0;
  }

  .prompt {
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--color-accent);
    flex-shrink: 0;
    user-select: none;
  }

  .chips {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
    flex-wrap: nowrap;
    max-width: 280px;
    overflow: hidden;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    background: color-mix(in oklch, var(--color-accent) 12%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-accent) 25%, transparent);
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 0.68rem;
    white-space: nowrap;
  }

  .chip-label {
    color: var(--color-foreground-darker, var(--color-foreground));
    opacity: 0.7;
    text-transform: uppercase;
    font-size: 0.6rem;
    letter-spacing: 0.04em;
  }

  .chip-value {
    color: var(--color-foreground);
    font-weight: 500;
  }

  .cmd-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--color-foreground);
    font-size: 1rem;
    font-family: inherit;
    caret-color: var(--color-accent);
    min-width: 0;
  }

  .cmd-input::placeholder {
    color: color-mix(in oklch, var(--color-foreground) 30%, transparent);
  }

  .hint-enter {
    font-size: 0.75rem;
    color: color-mix(in oklch, var(--color-foreground) 40%, transparent);
    flex-shrink: 0;
    border: 1px solid color-mix(in oklch, var(--color-foreground) 15%, transparent);
    border-radius: 4px;
    padding: 2px 5px;
    font-family: 'Mona Sans Mono', monospace;
  }

  .divider {
    height: 1px;
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
    flex-shrink: 0;
  }

  .dropdown {
    overflow-y: auto;
    padding: 4px 0;
    flex: 1;
  }

  .dropdown-section-label {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: color-mix(in oklch, var(--color-foreground) 40%, transparent);
    padding: 6px 16px 2px;
    user-select: none;
  }

  .dropdown-item {
    padding: 9px 16px;
    font-size: 0.875rem;
    cursor: pointer;
    color: var(--color-foreground);
    transition: background var(--transition-snappy, 0.12s ease);
    border-radius: 0;
  }

  .dropdown-item.active,
  .dropdown-item:hover {
    background: color-mix(in oklch, var(--color-foreground) 8%, transparent);
  }

  .dropdown-item :global(strong) {
    color: var(--color-accent);
    font-weight: 600;
  }
</style>
