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
    separatorClass = 'text-white/40 font-normal select-none',
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

{#if artists.length <= 1}
  <button
    type="button"
    onclick={(e) => handleClick(e, artists[0] || artist)}
    class="{className} {buttonClass} truncate"
    title={artists[0] || artist}
  >
    {artists[0] || artist || 'Unknown Artist'}
  </button>
{:else}
  <span class="inline-flex flex-wrap items-center gap-x-1 {className} truncate">
    {#each artists as art, idx}
      <button
        type="button"
        onclick={(e) => handleClick(e, art)}
        class="{buttonClass} truncate"
        title={art}
      >
        {art}
      </button>{#if idx < artists.length - 1}<span class="{separatorClass}">,</span>{/if}
    {/each}
  </span>
{/if}
