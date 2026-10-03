<script lang="ts">
  import { onMount } from 'svelte';
  import { playerState } from '../state.svelte';

  let canvasEl = $state<HTMLCanvasElement>();
  let lyricsContainerEl = $state<HTMLDivElement>();
  let activeIndex = $state(-1);

  // Sync lyrics highlight and auto-scroll
  $effect(() => {
    const pos = playerState.playback.position;
    if (playerState.lyrics.synced && playerState.lyrics.lines.length > 0) {
      let foundIdx = -1;
      for (let i = 0; i < playerState.lyrics.lines.length; i++) {
        if (pos >= playerState.lyrics.lines[i].time) {
          foundIdx = i;
        } else {
          break;
        }
      }
      if (foundIdx !== activeIndex) {
        activeIndex = foundIdx;
        scrollToActiveLyric(foundIdx);
      }
    }
  });

  function scrollToActiveLyric(idx: number) {
    if (!lyricsContainerEl || idx < 0) return;
    const lineEl = lyricsContainerEl.children[idx] as HTMLElement;
    if (lineEl) {
      lineEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }

  // 60FPS Ambient Audio Visualizer
  onMount(() => {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext('2d');
    if (!ctx) return;

    let animId: number;
    let phase = 0;

    const render = () => {
      const width = canvasEl.width;
      const height = canvasEl.height;
      ctx.clearRect(0, 0, width, height);

      const isPlaying = playerState.playback.is_playing;
      const bars = 48;
      const barWidth = width / bars - 2;

      for (let i = 0; i < bars; i++) {
        let barHeight = 4;
        if (isPlaying) {
          const freq = Math.sin(phase + i * 0.25) * 0.5 + 0.5;
          const bassBoost = i < 12 ? Math.cos(phase * 1.5 + i * 0.1) * 0.4 + 0.6 : 0.5;
          barHeight = (freq * bassBoost * (height - 8)) + 4;
        }

        const x = i * (barWidth + 2);
        const y = height - barHeight;

        // Gradient
        const grad = ctx.createLinearGradient(0, y, 0, height);
        grad.addColorStop(0, playerState.accentColor || '#d0bcff');
        grad.addColorStop(1, 'rgba(121, 220, 156, 0.4)');

        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.roundRect(x, y, barWidth, barHeight, 3);
        ctx.fill();
      }

      phase += isPlaying ? 0.08 : 0.01;
      animId = requestAnimationFrame(render);
    };

    render();

    return () => {
      cancelAnimationFrame(animId);
    };
  });
</script>

{#if playerState.isNowPlayingOpen}
  <div class="fixed inset-0 z-50 bg-[#0d0e11]/95 backdrop-blur-3xl flex flex-col p-6 overflow-hidden animate-fade-in select-none">
    
    <!-- Ambient glowing mesh background -->
    <div
      class="absolute -top-40 -left-40 w-[600px] h-[600px] rounded-full blur-[140px] opacity-25 pointer-events-none transition-colors duration-1000"
      style="background: radial-gradient(circle, {playerState.accentColor} 0%, transparent 70%);"
    ></div>

    <!-- Header bar -->
    <div class="flex items-center justify-between pb-4 border-b border-white/[0.06] z-10">
      <div class="flex items-center gap-3">
        <button
          onclick={() => playerState.isNowPlayingOpen = false}
          class="w-9 h-9 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white transition-colors"
          title="Minimize"
        >
          <span class="material-symbols-rounded text-[22px]">keyboard_arrow_down</span>
        </button>
        <span class="font-heading font-semibold text-sm text-[#9ea2b0] tracking-wide uppercase">Now Playing</span>
      </div>

      <div class="flex items-center gap-2">
        <span class="text-xs px-2.5 py-1 rounded-full bg-white/[0.07] text-white border border-white/[0.08] font-mono">
          {playerState.currentTrack?.format || 'Lossless'}
        </span>
        <button
          onclick={() => playerState.isEqualizerOpen = true}
          class="w-9 h-9 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-[#9ea2b0] hover:text-white transition-colors"
          title="Equalizer"
        >
          <span class="material-symbols-rounded text-[20px]">tune</span>
        </button>
      </div>
    </div>

    <!-- Main 2-Column Display: Artwork/Turntable + Lyrics -->
    <div class="flex-1 grid grid-cols-1 lg:grid-cols-2 gap-8 py-6 overflow-hidden z-10">
      
      <!-- Left Column: Spinning Vinyl / Turntable + Controls + Visualizer -->
      <div class="flex flex-col items-center justify-center gap-6 max-w-lg mx-auto w-full">
        
        <!-- Turntable Vinyl Container -->
        <div class="relative w-72 h-72 sm:w-80 sm:h-80 rounded-full flex items-center justify-center shadow-2xl p-2 bg-[#121316] border border-white/[0.1]">
          <!-- Vinyl body with subtle groove gradients -->
          <div
            class="relative w-full h-full rounded-full bg-gradient-to-tr from-[#1b1c20] via-[#101114] to-[#202227] flex items-center justify-center overflow-hidden {playerState.playback.is_playing ? 'animate-spin-slow' : ''}"
            style="animation-duration: 18s;"
          >
            <!-- Grooves -->
            <div class="absolute inset-4 rounded-full border border-white/[0.05]"></div>
            <div class="absolute inset-8 rounded-full border border-white/[0.04]"></div>
            <div class="absolute inset-14 rounded-full border border-white/[0.03]"></div>
            <div class="absolute inset-20 rounded-full border border-white/[0.05]"></div>

            <!-- Stationary specular light sheen overlay -->
            <div class="absolute inset-0 bg-gradient-to-tr from-white/[0.06] via-transparent to-white/[0.03] pointer-events-none rounded-full"></div>

            <!-- Center label with album cover art -->
            <div class="w-28 h-28 rounded-full overflow-hidden border-2 border-white/[0.15] shadow-inner relative z-10 flex items-center justify-center bg-[#252730]">
              {#if playerState.currentTrack?.cover_art}
                <img src={playerState.currentTrack.cover_art} alt="Vinyl Center" class="w-full h-full object-cover" />
              {:else}
                <span class="material-symbols-rounded text-white/50 text-[32px]">album</span>
              {/if}
              <!-- Center spindle hole -->
              <div class="absolute w-5 h-5 rounded-full bg-[#121316] border border-white/20"></div>
            </div>
          </div>
        </div>

        <!-- Track metadata -->
        <div class="text-center px-4 w-full">
          <h2 class="font-heading font-bold text-2xl text-white truncate mb-1">
            {playerState.currentTrack?.title || 'No Track Selected'}
          </h2>
          <p class="text-[#9ea2b0] text-sm truncate font-medium">
            {playerState.currentTrack?.artist || 'Select a song from your library or search'}
          </p>
        </div>

        <!-- 60FPS Audio Visualizer Canvas -->
        <div class="w-full px-6">
          <canvas bind:this={canvasEl} width="360" height="42" class="w-full h-11 rounded-lg"></canvas>
        </div>

        <!-- Scrubber -->
        <div class="w-full px-6 flex items-center gap-3 text-xs font-mono text-[#6b6e7c]">
          <span>{playerState.formattedTime.current}</span>
          <input
            type="range"
            min="0"
            max={playerState.playback.duration || (playerState.currentTrack?.duration ?? 100)}
            step="0.5"
            value={playerState.playback.position}
            oninput={(e) => playerState.seek(Number((e.target as HTMLInputElement).value))}
            class="flex-1 h-1.5 bg-white/[0.1] rounded-lg appearance-none cursor-pointer"
          />
          <span>{playerState.formattedTime.total}</span>
        </div>

        <!-- Transport Controls -->
        <div class="flex items-center justify-center gap-6">
          <button
            onclick={() => playerState.playPrevious()}
            class="text-[#9ea2b0] hover:text-white transition-colors"
            title="Previous"
          >
            <span class="material-symbols-rounded text-[28px]">skip_previous</span>
          </button>

          <button
            onclick={() => playerState.togglePlay()}
            class="w-14 h-14 rounded-full bg-white text-[#111215] flex items-center justify-center shadow-xl hover:scale-105 active:scale-95 transition-all"
            title={playerState.playback.is_playing ? 'Pause' : 'Play'}
          >
            <span class="material-symbols-rounded text-[32px] filled">
              {playerState.playback.is_playing ? 'pause' : 'play_arrow'}
            </span>
          </button>

          <button
            onclick={() => playerState.playNext()}
            class="text-[#9ea2b0] hover:text-white transition-colors"
            title="Next"
          >
            <span class="material-symbols-rounded text-[28px]">skip_next</span>
          </button>
        </div>

      </div>

      <!-- Right Column: Synchronized Karaoke Lyrics (LRCLIB) -->
      <div class="h-full flex flex-col bg-white/[0.02] border border-white/[0.05] rounded-3xl p-6 overflow-hidden">
        <div class="flex items-center justify-between pb-3 mb-2 border-b border-white/[0.05]">
          <span class="font-heading font-semibold text-xs tracking-wider uppercase text-[#9ea2b0]">
            Synchronized Lyrics (LRCLIB)
          </span>
          {#if playerState.lyrics.synced}
            <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
              Synced
            </span>
          {/if}
        </div>

        <div bind:this={lyricsContainerEl} class="flex-1 overflow-y-auto pr-2 space-y-4 py-8 scroll-smooth">
          {#if playerState.lyrics.synced && playerState.lyrics.lines.length > 0}
            {#each playerState.lyrics.lines as line, i}
              {@const isCur = i === activeIndex}
              <button
                onclick={() => playerState.seek(line.time)}
                class="w-full text-left font-heading text-lg sm:text-xl font-semibold transition-all duration-300 py-1.5 px-3 rounded-xl hover:bg-white/[0.04] {isCur ? 'text-white scale-[1.02] bg-white/[0.06] shadow-sm' : 'text-[#6b6e7c] hover:text-[#9ea2b0]'}"
              >
                {line.text}
              </button>
            {/each}
          {:else if playerState.lyrics.plain_text}
            <div class="text-[#9ea2b0] whitespace-pre-line text-sm leading-relaxed">
              {playerState.lyrics.plain_text}
            </div>
          {:else}
            <div class="h-full flex flex-col items-center justify-center text-center text-[#6b6e7c] py-20">
              <span class="material-symbols-rounded text-[40px] mb-2 opacity-50">lyrics</span>
              <p class="text-sm font-medium">No synced lyrics available for this song.</p>
              <p class="text-xs text-[#4c4f5e] mt-1">Playback is still active and streaming lossless audio.</p>
            </div>
          {/if}
        </div>
      </div>

    </div>
  </div>
{/if}

<style>
  @keyframes spinSlow {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
  .animate-spin-slow {
    animation: spinSlow 20s linear infinite;
  }
  .animate-fade-in {
    animation: fadeIn 0.25s cubic-bezier(0.2, 0, 0, 1);
  }
  @keyframes fadeIn {
    from { opacity: 0; transform: scale(0.98); }
    to { opacity: 1; transform: scale(1); }
  }
</style>
