<script lang="ts">
  import { playerState } from '../state.svelte';
  import { splitArtists } from '../types';

  interface Props {
    artist: string;
    class?: string;
    buttonClass?: string;
    separatorClass?: string;
    onArtistClick?: (artist: string) => void;
  }

  let {
    artist,
    class: className = '',
    buttonClass = 'hover:underline hover:text-white transition-colors cursor-pointer text-left',
    separatorClass = 'text-white/45 font-normal select-none',
    onArtistClick,
  }: Props = $props();

  let artists = $derived(splitArtists(artist));

  function handleClick(e: MouseEvent, targetArtist: string) {
    e.stopPropagation();
    if (onArtistClick) {
      onArtistClick(targetArtist);
    } else {
      playerState.openArtist(targetArtist);
    }
  }
</script>

<!-- ArchiveTune Single-Line Truncated Artist Links: Zero space before comma ("Artist 1, Artist 2") and never wraps to 2 lines -->
{#if artists.length <= 1}
  <button
    type="button"
    onclick={(e) => handleClick(e, artists[0] || artist)}
    class="block truncate max-w-full {className} {buttonClass}"
    title={artists[0] || artist}
  >
    {artists[0] || artist || 'Unknown Artist'}
  </button>
{:else}
  <div class="block truncate whitespace-nowrap overflow-hidden max-w-full {className}" title={artists.join(', ')}>
    {#each artists as art, idx}
      <button
        type="button"
        onclick={(e) => handleClick(e, art)}
        class="inline {buttonClass}"
      >{art}</button>{#if idx < artists.length - 1}<span class="{separatorClass}">, </span>{/if}
    {/each}
  </div>
{/if}
