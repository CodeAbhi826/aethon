<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';

  async function handleMinimize() {
    try {
      await invoke('window_minimize');
    } catch (e) {
      console.error('Minimize error:', e);
    }
  }

  async function handleMaximize() {
    try {
      await invoke('window_toggle_maximize');
    } catch (e) {
      console.error('Maximize error:', e);
    }
  }

  async function handleClose() {
    try {
      await invoke('window_close');
    } catch (e) {
      console.error('Close error:', e);
    }
  }

  let canvasEl = $state<HTMLCanvasElement>();
  let lyricsContainerEl = $state<HTMLDivElement>();
  let activeIndex = $state(-1);
  let userIsScrolling = $state(false);
  let userScrollTimeout: any = null;

  // View mode switcher: auto (side-by-side on wide screens), or explicit focus
  let activeView = $state<'auto' | 'turntable' | 'lyrics'>('auto');

  function handleUserScroll() {
    userIsScrolling = true;
    if (userScrollTimeout) clearTimeout(userScrollTimeout);
    userScrollTimeout = setTimeout(() => {
      userIsScrolling = false;
      if (activeIndex >= 0) {
        scrollToActiveLyric(activeIndex, false);
      }
    }, 4500);
  }

  function resumeSync() {
    userIsScrolling = false;
    if (userScrollTimeout) clearTimeout(userScrollTimeout);
    if (activeIndex >= 0) {
      scrollToActiveLyric(activeIndex, true);
    }
  }

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
        if (!userIsScrolling) {
          scrollToActiveLyric(foundIdx, false);
        }
      }
    }
  });

  function scrollToActiveLyric(idx: number, immediate: boolean) {
    if (!lyricsContainerEl || idx < 0) return;
    const lineEl = lyricsContainerEl.children[idx] as HTMLElement;
    if (lineEl) {
      const containerH = lyricsContainerEl.clientHeight;
      const targetTop = lineEl.offsetTop - (containerH / 2) + (lineEl.clientHeight / 2);
      lyricsContainerEl.scrollTo({
        top: Math.max(0, targetTop),
        behavior: immediate ? 'auto' : 'smooth',
      });
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
      const bars = 44;
      const barWidth = width / bars - 2;

      for (let i = 0; i < bars; i++) {
        let barHeight = 3;
        if (isPlaying) {
          const freq = Math.sin(phase + i * 0.28) * 0.5 + 0.5;
          const bassBoost = i < 10 ? Math.cos(phase * 1.6 + i * 0.12) * 0.4 + 0.6 : 0.5;
          barHeight = (freq * bassBoost * (height - 6)) + 3;
        }

        const x = i * (barWidth + 2);
        const y = height - barHeight;

        const grad = ctx.createLinearGradient(0, y, 0, height);
        grad.addColorStop(0, playerState.accentColor || '#d0bcff');
        grad.addColorStop(1, 'rgba(121, 220, 156, 0.4)');

        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.roundRect(x, y, barWidth, barHeight, 2.5);
        ctx.fill();
      }

      phase += isPlaying ? 0.08 : 0.01;
      animId = requestAnimationFrame(render);
    };

    render();

    return () => {
      cancelAnimationFrame(animId);
      if (userScrollTimeout) clearTimeout(userScrollTimeout);
    };
  });
</script>

{#if playerState.isNowPlayingOpen}
  <div class="fixed inset-0 z-50 bg-[#0d0e11]/95 backdrop-blur-3xl flex flex-col p-3 sm:p-5 overflow-hidden animate-fade-in select-none">
    
    <!-- Ambient glowing mesh background -->
    <div
      class="absolute -top-40 -left-40 w-[600px] h-[600px] rounded-full blur-[140px] opacity-25 pointer-events-none transition-colors duration-1000"
      style="background: radial-gradient(circle, {playerState.accentColor} 0%, transparent 70%);"
    ></div>

    <!-- Header bar with isolated drag region & window controls preserved -->
    <header class="flex items-center justify-between pb-2.5 border-b border-white/[0.06] z-20 shrink-0 h-11">
      <!-- Left: Minimize Sheet + Title -->
      <div class="flex items-center gap-2.5 shrink-0">
        <button
          onclick={() => playerState.isNowPlayingOpen = false}
          class="w-8 h-8 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white transition-colors cursor-pointer"
          title="Minimize Sheet (Esc)"
        >
          <span class="material-symbols-rounded text-[20px]">keyboard_arrow_down</span>
        </button>
        <span class="font-heading font-semibold text-xs sm:text-sm text-[#9ea2b0] tracking-wide uppercase">Now Playing</span>
      </div>

      <!-- Center: View Mode Segmented Switch + Drag Region -->
      <div class="flex items-center gap-2 px-2 flex-1 justify-center min-w-0">
        <!-- Responsive segmented pills -->
        <div class="flex items-center bg-black/40 border border-white/10 rounded-full p-0.5 text-xs">
          <button
            type="button"
            onclick={() => activeView = 'auto'}
            class="px-2.5 py-1 rounded-full text-[11px] font-medium transition-all cursor-pointer {activeView === 'auto' ? 'bg-white/15 text-white shadow-sm font-semibold' : 'text-white/60 hover:text-white'}"
            title="Auto / Split View"
          >
            Split
          </button>
          <button
            type="button"
            onclick={() => activeView = 'turntable'}
            class="px-2.5 py-1 rounded-full text-[11px] font-medium transition-all cursor-pointer {activeView === 'turntable' ? 'bg-white/15 text-white shadow-sm font-semibold' : 'text-white/60 hover:text-white'}"
            title="Vinyl Artwork Focus"
          >
            Vinyl
          </button>
          <button
            type="button"
            onclick={() => activeView = 'lyrics'}
            class="px-2.5 py-1 rounded-full text-[11px] font-medium transition-all cursor-pointer {activeView === 'lyrics' ? 'bg-white/15 text-white shadow-sm font-semibold' : 'text-white/60 hover:text-white'}"
            title="Apple Music Lyrics Focus"
          >
            Lyrics
          </button>
        </div>

        <!-- Dedicated drag region spacer (isolated so buttons never get intercepted) -->
        <div data-tauri-drag-region class="flex-1 h-8 cursor-default"></div>
      </div>

      <!-- Right: Equalizer + Format + Window Traffic Controls -->
      <div class="flex items-center gap-1.5 shrink-0">
        <span class="text-[11px] px-2 py-0.5 rounded-full bg-white/[0.07] text-white border border-white/[0.08] font-mono hidden sm:inline-block">
          {playerState.currentTrack?.format || 'Lossless'}
        </span>
        <button
          onclick={() => playerState.isEqualizerOpen = true}
          class="w-7 h-7 sm:w-8 sm:h-8 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-[#9ea2b0] hover:text-white transition-colors cursor-pointer"
          title="10-Band Equalizer"
        >
          <span class="material-symbols-rounded text-[18px]">tune</span>
        </button>

        <div class="h-3.5 w-[1px] bg-white/10 mx-1"></div>

        <!-- Window Traffic Controls (Preserved in Fullscreen) -->
        <div class="flex items-center gap-0.5">
          <button
            onclick={handleMinimize}
            aria-label="Minimize Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-white/[0.08] transition-colors cursor-pointer"
            title="Minimize Window"
          >
            <span class="material-symbols-rounded text-[15px]">remove</span>
          </button>
          <button
            onclick={handleMaximize}
            aria-label="Maximize Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-white/[0.08] transition-colors cursor-pointer"
            title="Maximize Window"
          >
            <span class="material-symbols-rounded text-[14px]">crop_square</span>
          </button>
          <button
            onclick={handleClose}
            aria-label="Close Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-red-500/80 transition-colors cursor-pointer"
            title="Close Window"
          >
            <span class="material-symbols-rounded text-[15px]">close</span>
          </button>
        </div>
      </div>
    </header>

    <!-- Main Content Area: Responsive Grid / Flex based on View Mode and Window Height -->
    <div class="flex-1 flex overflow-hidden z-10 min-h-0 py-2 sm:py-4 gap-4 lg:gap-8">
      
      <!-- Left Column: Spinning Vinyl / Turntable + Controls + Visualizer -->
      {#if activeView === 'auto' || activeView === 'turntable'}
        <div
          class="flex flex-col items-center justify-between mx-auto w-full min-h-0 transition-all {activeView === 'turntable' ? 'max-w-xl' : activeView === 'auto' ? 'hidden md:flex flex-1 max-w-md lg:max-w-lg' : 'hidden'}"
        >
          <!-- Vinyl Container: perfectly locked 1:1 circle that never distorts -->
          <div class="flex items-center justify-center w-full py-2 shrink-0">
            <div class="vinyl-platter relative rounded-full flex items-center justify-center shadow-2xl p-2 bg-[#101114] border border-white/10">
              <!-- Vinyl body with spinning groove patterns -->
              <div
                class="vinyl-disc relative rounded-full overflow-hidden flex items-center justify-center {playerState.playback.is_playing ? 'spinning' : ''}"
              >
                <!-- Concentric micro-grooves -->
                <div class="vinyl-grooves absolute inset-0 rounded-full pointer-events-none"></div>

                <!-- Photorealistic stationary double-fan sheen -->
                <div class="vinyl-sheen absolute inset-0 rounded-full pointer-events-none z-10"></div>

                <!-- Center label with album cover art -->
                <div class="vinyl-center rounded-full overflow-hidden border-2 border-white/20 shadow-inner relative z-20 flex items-center justify-center bg-[#252730]">
                  {#if playerState.currentTrack?.cover_art}
                    <img src={playerState.currentTrack.cover_art} alt="Vinyl Center" class="w-full h-full object-cover" />
                  {:else}
                    <span class="material-symbols-rounded text-white/50 text-[24px]">album</span>
                  {/if}
                  <!-- Center spindle hole -->
                  <div class="absolute w-3.5 h-3.5 rounded-full bg-[#121316] border border-white/30 z-30"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- Bottom controls cluster: Track metadata + Visualizer + Scrubber + Buttons -->
          <div class="w-full shrink-0 space-y-2.5 sm:space-y-3.5 px-2 max-w-md">
            <!-- Track metadata -->
            <div class="text-center px-2">
              <h2 class="font-heading font-extrabold text-base sm:text-lg md:text-xl text-white truncate leading-tight">
                {playerState.currentTrack?.title || 'No Track Selected'}
              </h2>
              <p class="text-[#9ea2b0] text-xs sm:text-sm truncate font-medium mt-0.5">
                {playerState.currentTrack?.artist || 'Select a song from your library or search'}
              </p>
            </div>

            <!-- 60FPS Audio Visualizer Canvas -->
            <div class="w-full px-2">
              <canvas bind:this={canvasEl} width="340" height="32" class="w-full h-6 sm:h-8 rounded-lg"></canvas>
            </div>

            <!-- Scrubber -->
            <div class="w-full px-2 flex items-center gap-2.5 text-[11px] font-mono text-[#6b6e7c]">
              <span class="w-8 text-right">{playerState.formattedTime.current}</span>
              <input
                type="range"
                min="0"
                max={playerState.playback.duration || (playerState.currentTrack?.duration ?? 100)}
                step="0.5"
                value={playerState.playback.position}
                oninput={(e) => playerState.seek(Number((e.target as HTMLInputElement).value))}
                class="flex-1 h-1 sm:h-1.5 bg-white/[0.1] rounded-lg appearance-none cursor-pointer"
              />
              <span class="w-8">{playerState.formattedTime.total}</span>
            </div>

            <!-- Transport Controls -->
            <div class="flex items-center justify-center gap-4 sm:gap-6 pt-0.5 pb-1">
              <button
                onclick={() => playerState.playPrevious()}
                class="text-[#9ea2b0] hover:text-white transition-colors cursor-pointer"
                title="Previous"
              >
                <span class="material-symbols-rounded text-[24px] sm:text-[28px]">skip_previous</span>
              </button>

              <button
                onclick={() => playerState.togglePlay()}
                class="w-11 h-11 sm:w-13 sm:h-13 rounded-full bg-white text-[#111215] flex items-center justify-center shadow-xl hover:scale-105 active:scale-95 transition-all cursor-pointer"
                title={playerState.isLoading ? 'Buffering stream...' : playerState.playback.is_playing ? 'Pause' : 'Play'}
              >
                {#if playerState.isLoading}
                  <div class="w-5 h-5 sm:w-6 sm:h-6 border-2 border-[#111215] border-t-transparent rounded-full animate-spin"></div>
                {:else}
                  <span class="material-symbols-rounded text-[26px] sm:text-[30px] filled">
                    {playerState.playback.is_playing ? 'pause' : 'play_arrow'}
                  </span>
                {/if}
              </button>

              <button
                onclick={() => playerState.playNext()}
                class="text-[#9ea2b0] hover:text-white transition-colors cursor-pointer"
                title="Next"
              >
                <span class="material-symbols-rounded text-[24px] sm:text-[28px]">skip_next</span>
              </button>
            </div>
          </div>

        </div>
      {/if}

      <!-- Right Column: Apple Music-Grade Synced Lyrics (LRCLIB) -->
      {#if activeView === 'auto' || activeView === 'lyrics'}
        <div
          class="flex-1 flex flex-col bg-white/[0.02] border border-white/[0.05] rounded-3xl p-3 sm:p-5 overflow-hidden min-h-0 relative {activeView === 'auto' ? 'w-full md:flex-1' : 'w-full max-w-4xl mx-auto'}"
        >
          <div class="flex items-center justify-between pb-2.5 mb-1 border-b border-white/[0.05] shrink-0">
            <span class="font-heading font-semibold text-xs tracking-wider uppercase text-[#9ea2b0] flex items-center gap-1.5">
              <span class="material-symbols-rounded text-base text-[var(--md-sys-primary)]">lyrics</span>
              Synchronized Lyrics (LRCLIB)
            </span>
            <div class="flex items-center gap-2">
              {#if userIsScrolling}
                <button
                  type="button"
                  onclick={resumeSync}
                  class="text-[10px] font-semibold px-2.5 py-0.5 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center gap-1 animate-pulse cursor-pointer shadow-sm"
                  title="Resume live lyric auto-scroll"
                >
                  <span class="material-symbols-rounded text-[12px]">sync</span>
                  Resume Sync
                </button>
              {/if}
              {#if playerState.lyrics.synced}
                <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
                  Live Synced
                </span>
              {/if}
            </div>
          </div>

          <div
            bind:this={lyricsContainerEl}
            onwheel={handleUserScroll}
            ontouchmove={handleUserScroll}
            role="region"
            aria-label="Synchronized lyrics view"
            class="flex-1 overflow-y-auto px-2 sm:px-4 py-6 space-y-1.5 sm:space-y-2 lyrics-scroll-container scrollbar-none"
          >
            {#if playerState.lyrics.synced && playerState.lyrics.lines.length > 0}
              {#each playerState.lyrics.lines as line, i}
                {@const isCur = i === activeIndex}
                <button
                  type="button"
                  onclick={() => { playerState.seek(line.time); resumeSync(); }}
                  class="w-full text-left font-display font-bold tracking-tight transition-all duration-300 py-1.5 sm:py-2 px-3 sm:px-4 rounded-2xl block text-xl sm:text-2xl md:text-3xl leading-snug cursor-pointer select-none {isCur ? 'active-lyric' : 'inactive-lyric'}"
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
                <span class="material-symbols-rounded text-[48px] mb-2 opacity-40">lyrics</span>
                <p class="text-sm sm:text-base font-semibold text-white/80">No synced lyrics available for this song.</p>
                <p class="text-xs text-[#9ea2b0]/70 mt-1">Playback is active with bit-perfect PipeWire output.</p>
              </div>
            {/if}
          </div>
        </div>
      {/if}

    </div>
  </div>
{/if}

<style>
  /* Photorealistic Circular Vinyl Record Geometry */
  .vinyl-platter {
    width: min(280px, 34vh);
    height: min(280px, 34vh);
    aspect-ratio: 1 / 1;
  }
  .vinyl-disc {
    width: 100%;
    height: 100%;
    aspect-ratio: 1 / 1;
    background: #0d0e12;
    will-change: transform;
    transform: translateZ(0);
    backface-visibility: hidden;
  }
  .vinyl-disc.spinning {
    animation: spinSlow 20s linear infinite;
  }
  .vinyl-grooves {
    background: repeating-radial-gradient(
      circle at center,
      #1c1d22 0px,
      #1c1d22 2px,
      #0e0f13 3px,
      #0e0f13 5px
    );
    opacity: 0.9;
  }
  .vinyl-sheen {
    background: conic-gradient(
      from 0deg at 50% 50%,
      rgba(255, 255, 255, 0.07) 0deg,
      transparent 45deg,
      rgba(255, 255, 255, 0.12) 90deg,
      transparent 135deg,
      rgba(255, 255, 255, 0.07) 180deg,
      transparent 225deg,
      rgba(255, 255, 255, 0.12) 270deg,
      transparent 315deg,
      rgba(255, 255, 255, 0.07) 360deg
    );
  }
  .vinyl-center {
    width: 35%;
    height: 35%;
    aspect-ratio: 1 / 1;
  }

  @keyframes spinSlow {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
  .animate-spin-slow {
    animation: spinSlow 20s linear infinite;
  }
  .animate-fade-in {
    animation: fadeIn 0.22s cubic-bezier(0.2, 0, 0, 1);
  }
  @keyframes fadeIn {
    from { opacity: 0; transform: scale(0.985); }
    to { opacity: 1; transform: scale(1); }
  }

  /* Apple Music Synced Lyrics Typography & Glow */
  .inactive-lyric {
    filter: blur(1.5px);
    opacity: 0.35;
    transform: scale(0.98);
    color: rgba(255, 255, 255, 0.7);
    transform-origin: left center;
    transition: all 0.35s cubic-bezier(0.25, 1, 0.5, 1);
  }
  .inactive-lyric:hover {
    filter: blur(0px);
    opacity: 0.85;
    transform: scale(1.01);
    color: #ffffff;
  }
  .active-lyric {
    filter: blur(0px);
    opacity: 1;
    transform: scale(1.04);
    color: #ffffff;
    text-shadow: 0 0 25px var(--lyric-glow, #d0bcff), 0 0 50px var(--lyric-glow, #d0bcff);
    transform-origin: left center;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    transition: all 0.35s cubic-bezier(0.25, 1, 0.5, 1);
  }
  .lyrics-scroll-container {
    scroll-behavior: smooth;
  }
</style>
