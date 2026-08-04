<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    onclick?: (event: MouseEvent) => void;
    disabled?: boolean;
    active?: boolean;
    icon?: boolean;
    stretch?: boolean;
    title?: string;
    class?: string;
    children?: Snippet;
  }

  let {
    onclick,
    disabled = false,
    active = false,
    icon = false,
    stretch = false,
    title,
    class: extraClass = "",
    children,
  }: Props = $props();
</script>

<button type="button" {title} {disabled} {onclick} class="btn {extraClass}" class:active class:icon class:stretch>
  {@render children?.()}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4em;
    font: inherit;
    font-weight: 600;
    color: var(--color-text);
    background: var(--color-panel-bg);
    border: 1px solid var(--color-accent-purple);
    border-radius: 999px;
    padding: 0.45em 1.2em;
    cursor: pointer;
    transition:
      box-shadow 150ms ease,
      border-color 150ms ease,
      background-color 150ms ease,
      color 150ms ease;
  }

  .btn:hover:not(:disabled) {
    border-color: var(--color-accent-purple-bright);
    box-shadow: var(--shadow-glow);
  }

  .btn:focus-visible {
    outline: 2px solid var(--color-accent-purple-bright);
    outline-offset: 2px;
  }

  .btn.active {
    background: var(--color-accent-purple);
    border-color: var(--color-accent-purple-bright);
    box-shadow: var(--shadow-glow);
    color: #fff;
  }

  .btn.icon {
    padding: 0;
    width: 2.1em;
    height: 2.1em;
    border-radius: 50%;
    flex: none;
  }

  .btn.stretch {
    width: 100%;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: default;
    box-shadow: none;
  }
</style>
