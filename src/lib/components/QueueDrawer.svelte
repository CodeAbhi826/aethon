<script lang="ts">
  import { playerState } from '../state.svelte';
  import CupertinoSwitch from './CupertinoSwitch.svelte';

  function handlePlayNow(index: number) {
    const track = playerState.queue[index];
    if (track) {
      playerState.removeFromQueue(index);
      playerState.playTrack(track);
    }
  }

  function handleMoveUp(index: number) {
    if (index <= 0) return;
    const item = playerState.queue[index];
    const newQueue = [...playerState.queue];
    newQueue[index] = newQueue[index - 1];
    newQueue[index - 1] = item;
    playerState.queue = newQueue;
  }

  function handleMoveDown(index: number) {
    if (index >= playerState.queue.length - 1) return;
    const item = playerState.queue[index];
    const newQueue = [...playerState.queue];
    newQueue[index] = newQueue[index + 1];
    newQueue[index + 1] = item;
    playerState.queue = newQueue;
  }
</script>

{#if playerState.isQueueOpen}
  <!-- Backdrop -->
  <div
    role="button"
    tabindex="0"
    onclick={() => playerState.isQueueOpen = false}
    onkeydown={(e) => { if (e.key === 'Escape') playerState.isQueueOpen = false; }}
    class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 transition-opacity duration-300"
  ></div>

  <!-- Sliding Queue Panel -->
  <div class="fixed top-0 right-0 h-full w-full sm:w-[420px] m3-acrylic border-l border-white/10 z-50 shadow-2xl flex flex-col justify-between overflow-hidden animate-slide-in select-none">
    
    <!-- Header -->
    <div class="p-4 border-b border-white/5 flex items-center justify-between">
      <div class="flex items-center gap-2.5">
        <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-2xl">queue_music</span>
        <div>
          <h2 class="font-display font-bold text-base text-white">Playing Next</h2>
          <p class="text-[11px] text-[var(--md-sys-on-surface-variant)]/70">
            {playerState.queue.length} {playerState.queue.length === 1 ? 'song' : 'songs'} in queue
          </p>
        </div>
      </div>

      <div class="flex items-center gap-1">
        {#if playerState.queue.length > 0}
          <button
            onclick={() => playerState.clearQueue()}
            class="px-2.5 py-1 rounded-full text-xs font-medium text-rose-400 hover:bg-rose-500/10 transition-colors"
            title="Clear all queued songs"
          >
            Clear
          </button>
        {/if}
        <button
          onclick={() => playerState.isQueueOpen = false}
          class="w-8 h-8 rounded-full flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
        >
          <span class="material-symbols-rounded text-xl">close</span>
        </button>
      </div>
    </div>

    <!-- Scrollable Queue Body -->
    <div class="flex-1 overflow-y-auto p-4 space-y-5 scrollbar-thin">
      
      <!-- Currently Playing Platter -->
      {#if playerState.currentTrack}
        <div class="space-y-2">
          <div class="text-[10px] font-bold uppercase tracking-wider text-[var(--md-sys-outline)] font-display flex items-center gap-1.5">
            <span class="w-1.5 h-1.5 rounded-full bg-[var(--md-sys-primary)] animate-ping"></span>
            <span>Now Playing</span>
          </div>

          <div class="p-3 rounded-2xl bg-[var(--md-sys-primary-container)]/25 border border-[var(--md-sys-primary)]/40 flex items-center gap-3.5 shadow-sm">
            <div class="relative w-12 h-12 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
              {#if playerState.currentTrack.cover_art}
                <img loading="lazy" decoding="async" src={playerState.currentTrack.cover_art} alt={playerState.currentTrack.title} class="w-full h-full object-cover" />
              {:else}
                <div class="w-full h-full flex items-center justify-center bg-white/5">
                  <span class="material-symbols-rounded text-white/40">music_note</span>
                </div>
              {/if}
              <div class="absolute inset-0 bg-black/40 flex items-center justify-center">
                <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-xl filled">
                  {playerState.playback.is_playing ? 'equalizer' : 'pause'}
                </span>
              </div>
            </div>

            <div class="min-w-0 flex-1">
              <div class="text-xs font-semibold text-[var(--md-sys-primary)] truncate">
                {playerState.currentTrack.title}
              </div>
              <div class="text-[11px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
                <span class="truncate">{playerState.currentTrack.artist}</span>
                {#if playerState.currentTrack.duration > 0}
                  <span class="text-white/30 shrink-0">•</span>
                  <span class="font-mono text-[10px] text-white/50 shrink-0">
                    {Math.floor(playerState.currentTrack.duration / 60)}:{Math.floor(playerState.currentTrack.duration % 60) < 10 ? '0' : ''}{Math.floor(playerState.currentTrack.duration % 60)}
                  </span>
                {/if}
              </div>
            </div>

            <span class="text-[9px] font-mono px-2 py-0.5 rounded-full bg-white/10 text-white/70 shrink-0 hidden xs:inline-block">
              {playerState.currentTrack.format || 'Opus 256k'}
            </span>
          </div>
        </div>
      {/if}

      <!-- Next in Queue List -->
      <div class="space-y-2">
        <div class="flex items-center justify-between text-[10px] font-bold uppercase tracking-wider text-[var(--md-sys-outline)] font-display">
          <span>Up Next</span>
          <span>{playerState.queue.length} upcoming</span>
        </div>

        {#if playerState.queue.length > 0}
          <div class="space-y-2">
            {#each playerState.queue as track, idx}
              <div
                class="flex items-center justify-between p-2.5 rounded-2xl bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.05] transition-all group"
              >
                <div class="flex items-center gap-3 min-w-0 flex-1 pr-2">
                  <div class="relative w-10 h-10 rounded-lg overflow-hidden shadow shrink-0 border border-white/10">
                    {#if track.cover_art}
                      <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover" />
                    {:else}
                      <div class="w-full h-full flex items-center justify-center bg-white/5">
                        <span class="material-symbols-rounded text-white/40 text-sm">music_note</span>
                      </div>
                    {/if}
                    <button
                      onclick={() => handlePlayNow(idx)}
                      class="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity cursor-pointer"
                      title="Play now"
                    >
                      <span class="material-symbols-rounded text-white text-base filled">play_arrow</span>
                    </button>
                  </div>

                  <div class="min-w-0 flex-1">
                    <div class="text-xs font-semibold text-white truncate group-hover:text-[var(--md-sys-primary)] transition">
                      {track.title}
                    </div>
                    <div class="text-[10px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
                      <span class="truncate">{track.artist}</span>
                      {#if track.duration > 0}
                        <span class="text-white/30 shrink-0">•</span>
                        <span class="font-mono text-[9px] text-white/50 shrink-0">
                          {Math.floor(track.duration / 60)}:{Math.floor(track.duration % 60) < 10 ? '0' : ''}{Math.floor(track.duration % 60)}
                        </span>
                      {/if}
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-1 shrink-0 opacity-40 group-hover:opacity-100 transition-opacity">
                  <button
                    onclick={() => handleMoveUp(idx)}
                    disabled={idx === 0}
                    class="p-1 rounded text-white/60 hover:text-white disabled:opacity-20 transition"
                    title="Move Up"
                  >
                    <span class="material-symbols-rounded text-sm">arrow_upward</span>
                  </button>
                  <button
                    onclick={() => handleMoveDown(idx)}
                    disabled={idx === playerState.queue.length - 1}
                    class="p-1 rounded text-white/60 hover:text-white disabled:opacity-20 transition"
                    title="Move Down"
                  >
                    <span class="material-symbols-rounded text-sm">arrow_downward</span>
                  </button>
                  <button
                    onclick={() => playerState.removeFromQueue(idx)}
                    class="p-1 rounded text-rose-400 hover:text-rose-300 transition"
                    title="Remove from queue"
                  >
                    <span class="material-symbols-rounded text-sm">close</span>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="p-6 rounded-2xl bg-white/[0.02] border border-white/[0.04] text-center space-y-2">
            <span class="material-symbols-rounded text-3xl text-white/30">playlist_play</span>
            <p class="text-xs font-medium text-white/60">Queue is empty</p>
            <p class="text-[11px] text-white/40 leading-relaxed">
              Add songs using "Play Next" or let Autoplay Radio seamlessly discover similar tracks.
            </p>
          </div>
        {/if}
      </div>

      <!-- Autoplay Radio Section -->
      <div class="p-4 rounded-2xl bg-white/[0.03] border border-white/[0.05] space-y-3">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="material-symbols-rounded text-amber-400 text-xl">bolt</span>
            <div>
              <div class="text-xs font-bold text-white">Autoplay Radio</div>
              <div class="text-[10px] text-white/50">Keep music playing when queue ends</div>
            </div>
          </div>
          <button
            onclick={() => playerState.autoplayRadio = !playerState.autoplayRadio}
            class="w-10 h-6 rounded-full transition-colors relative cursor-pointer {playerState.autoplayRadio ? 'bg-[var(--md-sys-primary)]' : 'bg-white/20'}"
          >
            <div class="w-4 h-4 rounded-full bg-white absolute top-1 transition-transform {playerState.autoplayRadio ? 'left-5' : 'left-1'}"></div>
          </button>
        </div>
      </div>

    </div>

    <!-- Footer Summary -->
    <div class="p-3 border-t border-white/5 text-center text-[10px] text-white/40">
      Aethon Fluid Queue Engine • GPU Accelerated
    </div>
  </div>
{/if}

<style>
  @keyframes slideIn {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }
  .animate-slide-in {
    animation: slideIn 0.25s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }
</style>
