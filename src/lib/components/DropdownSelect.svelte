<script lang="ts">
  interface Props {
    value: string;
    options: string[];
    placeholder: string;
  }

  let { value = $bindable(), options, placeholder }: Props = $props();

  let showDropdown = $state(false);
  let containerElement: HTMLDivElement;
  
  let allOptions = $derived([{ value: '', label: placeholder }, ...options.map(o => ({ value: o, label: o }))]);
  let selectedLabel = $derived(value === '' ? placeholder : value);

  function toggle() {
    showDropdown = !showDropdown;
  }

  function selectOption(val: string) {
    value = val;
    showDropdown = false;
  }

  function onClickOutside(e: MouseEvent) {
    if (showDropdown && containerElement && !containerElement.contains(e.target as Node)) {
      showDropdown = false;
    }
  }
</script>

<svelte:window onmousedown={onClickOutside} />

<div class="dropdown-container" bind:this={containerElement}>
  <button type="button" class="dropdown-toggle" onclick={toggle}>
    <span>{selectedLabel}</span>
  </button>
  
  {#if showDropdown}
    <ul class="dropdown-menu">
      {#each allOptions as opt}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <li 
          class:active={opt.value === value} 
          onclick={() => selectOption(opt.value)}
        >
          {opt.label}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .dropdown-container {
    position: relative;
    flex: 1;
    min-width: 120px;
  }
  
  .dropdown-toggle {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: var(--color-hover);
    background-image: url("/src/lib/assets/dropdown.svg");
    background-repeat: no-repeat;
    background-position: right 10px top 50%;
    background-size: 10px auto;
    color: var(--color-foreground);
    border: 1px solid var(--color-separator);
    padding: 6px 30px 6px 12px;
    border-radius: 4px;
    font-size: 0.85rem;
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .dropdown-toggle span {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dropdown-menu {
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
    padding: 0;
    margin-bottom: 0;
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
  }
  
  li {
    padding: 8px 14px;
    cursor: pointer;
    font-size: 0.85rem;
    color: var(--color-foreground);
    transition: background var(--transition-snappy);
  }
  
  li:hover {
    background: var(--color-hover);
  }

  li.active {
    background: var(--color-focus-round);
    color: var(--color-background);
  }
</style>
