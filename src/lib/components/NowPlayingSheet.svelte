<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { playerState } from '../state.svelte';

  const appWindow = typeof window !== 'undefined' ? getCurrentWindow() : null;

  async function handleMinimize() {
    await appWindow?.minimize();
  }

  async function handleMaximize() {
    await appWindow?.toggleMaximize();
  }

  async function handleClose() {
    await appWindow?.close();
  }

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
      if (!canvasEl) return;
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
  <div class="fixed inset-0 z-50 bg-[#0d0e11]/95 backdrop-blur-3xl flex flex-col p-4 sm:p-6 overflow-hidden animate-fade-in select-none">
    
    <!-- Ambient glowing mesh background -->
    <div
      class="absolute -top-40 -left-40 w-[600px] h-[600px] rounded-full blur-[140px] opacity-25 pointer-events-none transition-colors duration-1000"
      style="background: radial-gradient(circle, {playerState.accentColor} 0%, transparent 70%);"
    ></div>

    <!-- Header bar with drag region & window controls preserved -->
    <div data-tauri-drag-region class="flex items-center justify-between pb-3 border-b border-white/[0.06] z-10 shrink-0">
      <div class="flex items-center gap-3">
        <button
          onclick={() => playerState.isNowPlayingOpen = false}
          class="w-9 h-9 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white transition-colors"
          title="Minimize Sheet"
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

        <div class="h-4 w-[1px] bg-white/10 mx-1"></div>

        <!-- Window Traffic Controls (Preserved in Fullscreen) -->
        <div class="flex items-center gap-1">
          <button
            onclick={handleMinimize}
            aria-label="Minimize Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-white/[0.08] transition-colors"
            title="Minimize Window"
          >
            <span class="material-symbols-rounded text-[16px]">remove</span>
          </button>
          <button
            onclick={handleMaximize}
            aria-label="Maximize Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-white/[0.08] transition-colors"
            title="Maximize Window"
          >
            <span class="material-symbols-rounded text-[15px]">crop_square</span>
          </button>
          <button
            onclick={handleClose}
            aria-label="Close Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-red-500/80 transition-colors"
            title="Close Window"
          >
            <span class="material-symbols-rounded text-[16px]">close</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Main 2-Column Display: Artwork/Turntable + Apple Music Synced Lyrics -->
    <div class="flex-1 grid grid-cols-1 lg:grid-cols-2 gap-4 lg:gap-8 py-3 lg:py-6 overflow-hidden z-10 min-h-0">
      
      <!-- Left Column: Spinning Vinyl / Turntable + Controls + Visualizer -->
      <div class="flex flex-col items-center justify-center gap-3 sm:gap-5 max-w-lg mx-auto w-full min-h-0 overflow-y-auto scrollbar-none">
        
        <!-- Turntable Vinyl Container (Dynamically scaled for floating and fullscreen) -->
        <div class="relative w-44 h-44 sm:w-56 sm:h-56 md:w-64 md:h-64 lg:w-80 lg:h-80 rounded-full flex items-center justify-center shadow-2xl p-2 bg-[#121316] border border-white/[0.1] shrink-0 transition-all">
          <!-- Vinyl body with subtle groove gradients -->
          <div
            class="relative w-full h-full rounded-full bg-gradient-to-tr from-[#1b1c20] via-[#101114] to-[#202227] flex items-center justify-center overflow-hidden {playerState.playback.is_playing ? 'animate-spin-slow' : ''}"
            style="animation-duration: 18s;"
          >
            <!-- Grooves -->
            <div class="absolute inset-3 sm:inset-4 rounded-full border border-white/[0.05]"></div>
            <div class="absolute inset-6 sm:inset-8 rounded-full border border-white/[0.04]"></div>
            <div class="absolute inset-10 sm:inset-14 rounded-full border border-white/[0.03]"></div>
            <div class="absolute inset-14 sm:inset-20 rounded-full border border-white/[0.05]"></div>

            <!-- Stationary specular light sheen overlay -->
            <div class="absolute inset-0 bg-gradient-to-tr from-white/[0.06] via-transparent to-white/[0.03] pointer-events-none rounded-full"></div>

            <!-- Center label with album cover art -->
            <div class="w-16 h-16 sm:w-20 sm:h-20 md:w-24 md:h-24 lg:w-28 lg:h-28 rounded-full overflow-hidden border-2 border-white/[0.15] shadow-inner relative z-10 flex items-center justify-center bg-[#252730] transition-all">
              {#if playerState.currentTrack?.cover_art}
                <img src={playerState.currentTrack.cover_art} alt="Vinyl Center" class="w-full h-full object-cover" />
              {:else}
                <span class="material-symbols-rounded text-white/50 text-[28px] sm:text-[32px]">album</span>
              {/if}
              <!-- Center spindle hole -->
              <div class="absolute w-3.5 h-3.5 sm:w-5 sm:h-5 rounded-full bg-[#121316] border border-white/20"></div>
            </div>
          </div>
        </div>

        <!-- Track metadata -->
        <div class="text-center px-4 w-full">
          <h2 class="font-heading font-bold text-lg sm:text-xl md:text-2xl text-white truncate mb-0.5">
            {playerState.currentTrack?.title || 'No Track Selected'}
          </h2>
          <p class="text-[#9ea2b0] text-xs sm:text-sm truncate font-medium">
            {playerState.currentTrack?.artist || 'Select a song from your library or search'}
          </p>
        </div>

        <!-- 60FPS Audio Visualizer Canvas -->
        <div class="w-full px-4 sm:px-6">
          <canvas bind:this={canvasEl} width="360" height="42" class="w-full h-8 sm:h-10 rounded-lg"></canvas>
        </div>

        <!-- Scrubber -->
        <div class="w-full px-4 sm:px-6 flex items-center gap-3 text-xs font-mono text-[#6b6e7c]">
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
        <div class="flex items-center justify-center gap-5 sm:gap-6">
          <button
            onclick={() => playerState.playPrevious()}
            class="text-[#9ea2b0] hover:text-white transition-colors"
            title="Previous"
          >
            <span class="material-symbols-rounded text-[26px] sm:text-[28px]">skip_previous</span>
          </button>

          <button
            onclick={() => playerState.togglePlay()}
            class="w-12 h-12 sm:w-14 sm:h-14 rounded-full bg-white text-[#111215] flex items-center justify-center shadow-xl hover:scale-105 active:scale-95 transition-all"
            title={playerState.playback.is_playing ? 'Pause' : 'Play'}
          >
            <span class="material-symbols-rounded text-[28px] sm:text-[32px] filled">
              {playerState.playback.is_playing ? 'pause' : 'play_arrow'}
            </span>
          </button>

          <button
            onclick={() => playerState.playNext()}
            class="text-[#9ea2b0] hover:text-white transition-colors"
            title="Next"
          >
            <span class="material-symbols-rounded text-[26px] sm:text-[28px]">skip_next</span>
          </button>
        </div>

      </div>

      <!-- Right Column: Apple Music-Grade Synced Lyrics (LRCLIB) -->
      <div class="h-full flex flex-col bg-white/[0.02] border border-white/[0.05] rounded-3xl p-4 sm:p-6 overflow-hidden min-h-0">
        <div class="flex items-center justify-between pb-3 mb-2 border-b border-white/[0.05] shrink-0">
          <span class="font-heading font-semibold text-xs tracking-wider uppercase text-[#9ea2b0] flex items-center gap-1.5">
            <span class="material-symbols-rounded text-base text-[var(--md-sys-primary)]">lyrics</span>
            Synchronized Lyrics (LRCLIB)
          </span>
          {#if playerState.lyrics.synced}
            <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
              Live Synced
            </span>
          {/if}
        </div>

        <div
          bind:this={lyricsContainerEl}
          class="flex-1 overflow-y-auto pr-2 space-y-4 py-8 lyrics-scroll-container scrollbar-none"
        >
          {#if playerState.lyrics.synced && playerState.lyrics.lines.length > 0}
            {#each playerState.lyrics.lines as line, i}
              {@const isCur = i === activeIndex}
              <button
                type="button"
                onclick={() => playerState.seek(line.time)}
                class="w-full text-left font-display font-extrabold tracking-tight transition-all duration-300 py-2 px-3 sm:px-4 rounded-2xl block text-xl sm:text-2xl md:text-3xl leading-snug cursor-pointer {isCur ? 'active-lyric' : 'inactive-lyric hover:opacity-70'}"
                style={isCur ? `--lyric-glow: ${playerState.accentColor || 'var(--md-sys-primary)'};` : ''}
              >
                {line.text}
              </button>
            {/each}
          {:else if playerState.lyrics.plain_text}
            <div class="text-[#9ea2b0] whitespace-pre-line text-base sm:text-lg font-medium leading-relaxed p-4">
              {playerState.lyrics.plain_text}
            </div>
          {:else}
            <div class="h-full flex flex-col items-center justify-center text-center text-[#6b6e7c] py-16">
              <span class="material-symbols-rounded text-[44px] mb-2 opacity-40">lyrics</span>
              <p class="text-sm sm:text-base font-semibold text-white/80">No synced lyrics available for this song.</p>
              <p class="text-xs text-[#9ea2b0]/70 mt-1">Playback is active with bit-perfect PipeWire output.</p>
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

  /* Apple Music Synced Lyrics Typography & Glow */
  .inactive-lyric {
    filter: blur(1.5px);
    opacity: 0.35;
    transform: scale(0.98);
    color: rgba(255, 255, 255, 0.7);
    transform-origin: left center;
  }
  .active-lyric {
    filter: blur(0px);
    opacity: 1;
    transform: scale(1.05);
    color: #ffffff;
    text-shadow: 0 0 20px var(--lyric-glow, #d0bcff), 0 0 45px var(--lyric-glow, #d0bcff);
    transform-origin: left center;
  }
  .lyrics-scroll-container {
    scroll-behavior: smooth;
  }
</style>
