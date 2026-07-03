<script lang="ts">
  interface Props {
    value: string;
    options: string[];
    placeholder?: string;
    oncommit?: (val: string) => void;
  }

  let {
    value = $bindable(),
    options,
    placeholder = '',
    oncommit
  }: Props = $props();

  let showDropdown = $state(false);
  let filteredOptions = $derived(
    options.filter(opt => opt.toLowerCase().includes(value.toLowerCase()))
  );
  
  let inputElement: HTMLInputElement;
  let activeIndex = $state(-1);
  let originalValueOnFocus = $state('');
  
  function handleInput(e: Event) {
    value = (e.target as HTMLInputElement).value;
    showDropdown = true;
    activeIndex = -1;
  }
  
  function handleFocus() {
    originalValueOnFocus = value;
    showDropdown = true;
  }
  
  function handleBlur() {
    // Delay to allow mousedown on dropdown to fire before blur hides it
    setTimeout(() => {
      showDropdown = false;
      if (value !== originalValueOnFocus) {
        oncommit?.(value);
        originalValueOnFocus = value;
      }
    }, 150);
  }
  
  function selectOption(opt: string) {
    value = opt;
    showDropdown = false;
    if (value !== originalValueOnFocus) {
      oncommit?.(value);
      originalValueOnFocus = value;
    }
  }
  
  function handleKeyDown(e: KeyboardEvent) {
    if (!showDropdown) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        showDropdown = true;
        e.preventDefault();
      } else if (e.key === 'Enter') {
        inputElement.blur();
      }
      return;
    }
    
    if (e.key === 'ArrowDown') {
      activeIndex = (activeIndex + 1) % filteredOptions.length;
      e.preventDefault();
    } else if (e.key === 'ArrowUp') {
      activeIndex = activeIndex - 1;
      if (activeIndex < 0) activeIndex = filteredOptions.length - 1;
      e.preventDefault();
    } else if (e.key === 'Enter') {
      if (activeIndex >= 0 && activeIndex < filteredOptions.length) {
        selectOption(filteredOptions[activeIndex]);
      } else {
        showDropdown = false;
        inputElement.blur();
      }
      e.preventDefault();
    } else if (e.key === 'Escape') {
      showDropdown = false;
      e.preventDefault();
    }
  }
</script>

<div class="autocomplete-container">
  <input
    bind:this={inputElement}
    type="text"
    {value}
    {placeholder}
    oninput={handleInput}
    onfocus={handleFocus}
    onblur={handleBlur}
    onkeydown={handleKeyDown}
  />
  {#if showDropdown && filteredOptions.length > 0}
    <ul class="dropdown">
      {#each filteredOptions as opt, i}
        <!-- Use mousedown instead of click because mousedown fires before blur -->
        <li 
          class:active={i === activeIndex} 
          onmousedown={(e) => { e.preventDefault(); selectOption(opt); }}
        >
          {opt}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .autocomplete-container {
    position: relative;
    width: 100%;
  }
  
  input {
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
  }
  
  input:focus {
    border-color: var(--color-accent);
    background: color-mix(in oklch, var(--color-foreground) 15%, transparent);
  }
  
  .dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    background: var(--color-background-light);
    border: 1px solid var(--color-separator);
    border-radius: 4px;
    max-height: 200px;
    overflow-y: auto;
    z-index: 100;
    list-style: none;
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
  }
  
  li {
    padding: 8px 14px;
    cursor: pointer;
    font-size: 0.9rem;
    color: var(--color-foreground);
    transition: background var(--transition-snappy);
  }
  
  li:hover, li.active {
    background: var(--color-hover);
  }
</style>
