<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import { sanitizeArtist } from '../types';

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

  // Lyric timing offset and translation controls
  let isOffsetOpen = $state(false);
  let isRomanizedMode = $state(false);

  const audioQualityLabel = $derived.by(() => {
    const fmt = playerState.currentTrack?.format;
    if (!fmt) return 'Lossless';
    if (fmt.toLowerCase().includes('lossless')) return fmt;
    return `Lossless • ${fmt}`;
  });

  function adjustOffset(delta: number) {
    settingsState.lyricOffsetSeconds = Math.round((settingsState.lyricOffsetSeconds + delta) * 10) / 10;
    settingsState.scheduleSave();
  }

  function resetOffset() {
    settingsState.lyricOffsetSeconds = 0.0;
    settingsState.scheduleSave();
  }

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

  function getWordTokens(text: string): string[] {
    const trimmed = text.trim();
    if (!trimmed) return [];
    const words = trimmed.split(/\s+/).filter(Boolean);
    if (words.length <= 1 && /[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uff66-\uff9f]/.test(trimmed)) {
      return Array.from(trimmed);
    }
    return words;
  }

  // Calculate word-by-word progressive text fill for active line
  function getWordProgress(lineIdx: number, wordIdx: number, totalWords: number, lineTime: number): number {
    if (lineIdx !== activeIndex || totalWords === 0) return 0;
    const nextLineTime = playerState.lyrics.lines[lineIdx + 1]?.time ?? (lineTime + 4.0);
    const lineDur = Math.max(0.2, nextLineTime - lineTime);
    const curPos = playerState.playback.position + settingsState.lyricOffsetSeconds;
    const lineProgress = Math.min(1, Math.max(0, (curPos - lineTime) / lineDur));
    const wordStart = wordIdx / totalWords;
    const wordEnd = (wordIdx + 1) / totalWords;
    if (lineProgress <= wordStart) return 0;
    if (lineProgress >= wordEnd) return 1;
    return (lineProgress - wordStart) / (wordEnd - wordStart);
  }

  // Sync lyrics highlight and auto-scroll (with user offset)
  $effect(() => {
    const pos = playerState.playback.position + settingsState.lyricOffsetSeconds;
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
      class="absolute -top-40 -left-40 w-[600px] h-[600px] rounded-full blur-[140px] opacity-25 pointer-events-none transition-colors duration-1000 gpu-layer"
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
            title="Split Layout"
          >
            Split
          </button>
          <button
            type="button"
            onclick={() => activeView = 'turntable'}
            class="px-2.5 py-1 rounded-full text-[11px] font-medium transition-all cursor-pointer {activeView === 'turntable' ? 'bg-white/15 text-white shadow-sm font-semibold' : 'text-white/60 hover:text-white'}"
            title="Vinyl Focus"
          >
            Vinyl
          </button>
          <button
            type="button"
            onclick={() => activeView = 'lyrics'}
            class="px-2.5 py-1 rounded-full text-[11px] font-medium transition-all cursor-pointer {activeView === 'lyrics' ? 'bg-white/15 text-white shadow-sm font-semibold' : 'text-white/60 hover:text-white'}"
            title="Focus Lyrics"
          >
            Lyrics
          </button>
        </div>

        <!-- Dedicated drag region spacer (isolated so buttons never get intercepted) -->
        <div data-tauri-drag-region class="flex-1 h-8 cursor-default"></div>
      </div>

      <!-- Right: Equalizer + Format + Window Traffic Controls -->
      <div class="flex items-center gap-1.5 shrink-0">
        {#if settingsState.showAudioQualityBadge && playerState.currentTrack}
          <span class="border border-white/20 text-white/60 text-[10px] tracking-wider uppercase px-2 py-0.5 rounded-full font-medium hidden sm:inline-block">
            {audioQualityLabel}
          </span>
        {/if}
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
          class="flex flex-col items-center justify-between min-h-0 {activeView === 'auto' ? 'w-full md:w-[48%] lg:w-[44%] shrink-0' : 'w-full max-w-lg mx-auto'} bg-white/[0.02] border border-white/[0.05] rounded-3xl p-3 sm:p-5"
        >
          
          <!-- Vinyl Album Platter Section -->
          <div class="flex-1 flex items-center justify-center w-full min-h-0 my-1 sm:my-2">
            <div class="relative vinyl-platter flex items-center justify-center">
              <div
                class="vinyl-disc rounded-full shadow-2xl relative flex items-center justify-center {playerState.playback.is_playing ? 'spinning' : ''}"
              >
                <!-- Vinyl groove lines -->
                <div class="absolute inset-0 rounded-full vinyl-grooves pointer-events-none"></div>

                <!-- Vinyl specular reflection sheen -->
                <div class="absolute inset-0 rounded-full vinyl-sheen pointer-events-none"></div>

                <!-- Center album artwork sticker -->
                <div class="vinyl-center rounded-full overflow-hidden shadow-inner border-2 border-[#121318] z-10 relative">
                  {#if playerState.currentTrack?.cover_art}
                    <img
                      src={playerState.currentTrack.cover_art}
                      alt={playerState.currentTrack.title}
                      class="w-full h-full object-cover"
                    />
                  {:else}
                    <div class="w-full h-full bg-[#1b1c22] flex items-center justify-center text-white/40">
                      <span class="material-symbols-rounded text-2xl">music_note</span>
                    </div>
                  {/if}
                  <!-- Center hole spindle -->
                  <div class="absolute inset-0 m-auto w-3 h-3 rounded-full bg-[#0a0a0c] border border-white/20"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- Bottom: Track Info + 60FPS Ambient Visualizer + Integrated Controls -->
          <div class="w-full max-w-sm flex flex-col items-center shrink-0 space-y-2.5">
            <!-- Title & Artist -->
            <div class="text-center w-full px-2 flex flex-col items-center">
              <h2 class="font-display font-bold text-base sm:text-xl text-white truncate max-w-full">
                {playerState.currentTrack?.title || 'No Track Selected'}
              </h2>
              <p class="text-xs sm:text-sm text-[#9ea2b0] truncate mt-0.5 max-w-full">
                {playerState.currentTrack ? sanitizeArtist(playerState.currentTrack.artist) : 'Select a song to start listening'}
              </p>
              {#if settingsState.showAudioQualityBadge && playerState.currentTrack}
                <div class="mt-2 flex items-center justify-center">
                  <span class="border border-white/20 text-white/60 text-[10px] tracking-wider uppercase px-2 py-0.5 rounded-full font-medium">
                    {audioQualityLabel}
                  </span>
                </div>
              {/if}
            </div>

            <!-- Mini 60FPS Canvas Visualizer -->
            <div class="w-full h-6 flex items-center justify-center">
              <canvas bind:this={canvasEl} width="320" height="24" class="w-full h-full opacity-70"></canvas>
            </div>

            <!-- Scrubber Track & Durations -->
            <div class="w-full space-y-1">
              <div class="relative w-full h-1.5 bg-white/[0.08] hover:bg-white/[0.15] rounded-full overflow-hidden cursor-pointer group transition-colors">
                <div
                  class="absolute left-0 top-0 bottom-0 bg-[var(--md-sys-primary)] rounded-full transition-all duration-100"
                  style="width: {playerState.progressPercent}%;"
                ></div>
                <input
                  type="range"
                  min="0"
                  max={playerState.playback.duration || 100}
                  value={playerState.playback.position}
                  oninput={(e) => playerState.seek(parseFloat(e.currentTarget.value))}
                  class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                />
              </div>
              <div class="flex justify-between text-[11px] font-mono text-[#9ea2b0]">
                <span>{playerState.formattedTime.current}</span>
                <span>{playerState.formattedTime.total}</span>
              </div>
            </div>

            <!-- Essential Playback Transport Controls -->
            <div class="flex items-center justify-center gap-5 sm:gap-6 pt-1">
              <button
                onclick={() => playerState.playPrevious()}
                class="text-[#9ea2b0] hover:text-white transition-colors cursor-pointer"
                title="Previous"
              >
                <span class="material-symbols-rounded text-[24px] sm:text-[28px]">skip_previous</span>
              </button>

              <button
                onclick={() => playerState.togglePlay()}
                class="w-11 h-11 sm:w-13 sm:h-13 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] shadow-lg shadow-[var(--md-sys-primary)]/20 hover:scale-105 active:scale-95 transition-all flex items-center justify-center cursor-pointer"
                title={playerState.isLoading ? 'Buffering stream...' : playerState.playback.is_playing ? 'Pause' : 'Play'}
              >
                {#if playerState.isLoading}
                  <svg class="w-6 h-6 animate-spin [animation-timing-function:steps(8)] text-current" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <line x1="12" y1="2" x2="12" y2="6" stroke-width="2.5" stroke-linecap="round" class="opacity-100" />
                    <line x1="19.07" y1="4.93" x2="16.24" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-90" />
                    <line x1="22" y1="12" x2="18" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-75" />
                    <line x1="19.07" y1="19.07" x2="16.24" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-60" />
                    <line x1="12" y1="22" x2="12" y2="18" stroke-width="2.5" stroke-linecap="round" class="opacity-45" />
                    <line x1="4.93" y1="19.07" x2="7.76" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-30" />
                    <line x1="2" y1="12" x2="6" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-20" />
                    <line x1="4.93" y1="4.93" x2="7.76" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-15" />
                  </svg>
                {:else}
                  {#if playerState.playback.is_playing}
                    <svg viewBox="0 0 24 24" class="w-6 h-6 sm:w-7 sm:h-7 fill-current" xmlns="http://www.w3.org/2000/svg">
                      <rect x="6" y="4" width="4" height="16" rx="1.5" />
                      <rect x="14" y="4" width="4" height="16" rx="1.5" />
                    </svg>
                  {:else}
                    <svg viewBox="0 0 24 24" class="w-6 h-6 sm:w-7 sm:h-7 fill-current translate-x-0.5" xmlns="http://www.w3.org/2000/svg">
                      <path d="M8 5.14v13.72a1 1 0 0 0 1.5.86l11.04-6.86a1 1 0 0 0 0-1.72L9.5 4.28A1 1 0 0 0 8 5.14z"/>
                    </svg>
                  {/if}
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

      <!-- Right Column: Fluid Editorial Synced Lyrics Studio -->
      {#if activeView === 'auto' || activeView === 'lyrics'}
        <div
          class="flex-1 flex flex-col bg-white/[0.02] border border-white/[0.05] rounded-3xl p-3 sm:p-5 overflow-hidden min-h-0 relative {activeView === 'auto' ? 'w-full md:flex-1' : 'w-full max-w-4xl mx-auto'} gpu-layer"
        >
          <!-- Lyrics Studio Toolbar -->
          <div class="flex items-center justify-between pb-2.5 mb-1 border-b border-white/[0.05] shrink-0">
            <div class="flex items-center gap-2">
              <span class="font-heading font-semibold text-xs tracking-wider uppercase text-[#9ea2b0] flex items-center gap-1.5">
                <span class="material-symbols-rounded text-base text-[var(--md-sys-primary)]">lyrics</span>
                Fluid Synced Lyrics
              </span>
              {#if playerState.lyrics.synced}
                <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
                  Live
                </span>
              {/if}
            </div>

            <!-- Sync adjustment & translation buttons -->
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

              <!-- Romanized / Translate Toggle -->
              <button
                type="button"
                onclick={() => isRomanizedMode = !isRomanizedMode}
                class="px-2 py-0.5 rounded-full text-[10px] font-semibold flex items-center gap-1 border transition-all cursor-pointer {isRomanizedMode ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] border-[var(--md-sys-primary)]' : 'bg-white/[0.06] text-white/70 hover:text-white border-white/10'}"
                title="Toggle Romanized / Phonetic View"
              >
                <span class="material-symbols-rounded text-[12px]">translate</span>
                <span>Phonetic</span>
              </button>

              <!-- Manual Sync Offset Button -->
              <button
                type="button"
                onclick={() => isOffsetOpen = !isOffsetOpen}
                class="px-2 py-0.5 rounded-full text-[10px] font-semibold flex items-center gap-1 border transition-all cursor-pointer {Math.abs(settingsState.lyricOffsetSeconds) > 0.05 ? 'bg-amber-500/20 text-amber-300 border-amber-500/30' : 'bg-white/[0.06] text-white/70 hover:text-white border-white/10'}"
                title="Adjust Lyric Sync Timing"
              >
                <span class="material-symbols-rounded text-[12px]">tune</span>
                <span>{settingsState.lyricOffsetSeconds > 0 ? '+' : ''}{settingsState.lyricOffsetSeconds.toFixed(1)}s</span>
              </button>
            </div>
          </div>

          <!-- Quick Timing Offset Slider Bar (Expandable) -->
          {#if isOffsetOpen}
            <div class="flex items-center justify-between bg-black/40 border border-white/10 rounded-2xl px-3.5 py-2 mb-2 animate-fade-in text-xs text-white/80">
              <span class="text-[11px] text-white/60 font-medium">Sync Offset</span>
              
              <div class="flex items-center gap-2">
                <button
                  onclick={() => adjustOffset(-0.5)}
                  class="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-[10px] font-mono cursor-pointer"
                >
                  -0.5s
                </button>
                <button
                  onclick={() => adjustOffset(-0.1)}
                  class="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-[10px] font-mono cursor-pointer"
                >
                  -0.1s
                </button>

                <input
                  type="range"
                  min="-2.0"
                  max="2.0"
                  step="0.1"
                  bind:value={settingsState.lyricOffsetSeconds}
                  oninput={() => settingsState.scheduleSave()}
                  class="w-24 sm:w-32 accent-[var(--md-sys-primary)] cursor-pointer"
                />

                <span class="font-mono text-xs w-11 text-center font-bold text-[var(--md-sys-primary)]">
                  {settingsState.lyricOffsetSeconds > 0 ? '+' : ''}{settingsState.lyricOffsetSeconds.toFixed(1)}s
                </span>

                <button
                  onclick={() => adjustOffset(0.1)}
                  class="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-[10px] font-mono cursor-pointer"
                >
                  +0.1s
                </button>
                <button
                  onclick={() => adjustOffset(0.5)}
                  class="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-[10px] font-mono cursor-pointer"
                >
                  +0.5s
                </button>

                <button
                  onclick={resetOffset}
                  class="ml-1 text-[10px] px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-white/70 hover:text-white cursor-pointer"
                  title="Reset offset to 0.0s"
                >
                  Reset
                </button>
              </div>
            </div>
          {/if}

          <!-- Lyric Scroll Container -->
          <div
            bind:this={lyricsContainerEl}
            onwheel={handleUserScroll}
            ontouchmove={handleUserScroll}
            role="region"
            aria-label="Synchronized lyrics view"
            class="flex-1 overflow-y-auto px-2 sm:px-4 py-6 space-y-1.5 sm:space-y-2 lyrics-scroll-container scrollbar-none contain-gpu"
          >
            {#if playerState.lyrics.synced && playerState.lyrics.lines.length > 0}
              {#each playerState.lyrics.lines as line, i}
                {@const isCur = i === activeIndex}
                <button
                  type="button"
                  onclick={() => { playerState.seek(line.time - settingsState.lyricOffsetSeconds); resumeSync(); }}
                  class="w-full text-left font-display font-bold tracking-tight transition-all duration-300 py-1.5 sm:py-2 px-3 sm:px-4 rounded-2xl block text-xl sm:text-2xl md:text-3xl leading-snug cursor-pointer select-none gpu-lyric {isCur ? 'active-lyric' : (settingsState.blurInactiveLines ? 'inactive-lyric-blur' : 'inactive-lyric-sharp')}"
                  style={isCur ? `--lyric-glow: ${playerState.accentColor || 'var(--md-sys-primary)'};` : ''}
                >
                  {#if isCur && settingsState.wordByWordTracking}
                    {@const words = getWordTokens(line.text)}
                    <span class="inline-flex flex-wrap gap-x-2">
                      {#each words as word, wIdx}
                        {@const wp = getWordProgress(i, wIdx, words.length, line.time)}
                        <span
                          class="word-token transition-all duration-150 inline-block {wp >= 1 ? 'word-sung' : (wp <= 0 ? 'word-unsung' : 'word-singing')}"
                          style={wp > 0 && wp < 1 ? `background: linear-gradient(90deg, #ffffff ${wp * 100}%, rgba(255, 255, 255, 0.4) ${wp * 100}%); -webkit-background-clip: text; -webkit-text-fill-color: transparent;` : ''}
                        >
                          {word}
                        </span>
                      {/each}
                    </span>
                  {:else}
                    <span>{line.text}</span>
                  {/if}
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
  .animate-fade-in {
    animation: fadeIn 0.22s cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes fadeIn {
    from { opacity: 0; transform: scale(0.985); }
    to { opacity: 1; transform: scale(1); }
  }

  /* GPU Composite-Only Accelerated Lyrics */
  .contain-gpu {
    contain: layout style;
  }
  .gpu-layer {
    will-change: transform, opacity;
    transform: translate3d(0, 0, 0);
  }
  .gpu-lyric {
    contain: layout style;
    will-change: transform, opacity;
    transform: translate3d(0, 0, 0);
  }

  /* Inactive Lyric with Focus Line Blur Enabled */
  .inactive-lyric-blur {
    filter: blur(1.5px);
    opacity: 0.35;
    transform: translate3d(0, 0, 0) scale(0.98);
    color: rgba(255, 255, 255, 0.7);
    transform-origin: left center;
    transition: all 0.35s cubic-bezier(0.16, 1, 0.3, 1);
  }
  .inactive-lyric-blur:hover {
    filter: blur(0px);
    opacity: 0.85;
    transform: translate3d(0, 0, 0) scale(1.01);
    color: #ffffff;
  }

  /* Inactive Lyric with Focus Line Blur Disabled (Sharp + Dimmed) */
  .inactive-lyric-sharp {
    filter: none;
    opacity: 0.45;
    transform: translate3d(0, 0, 0) scale(0.98);
    color: rgba(255, 255, 255, 0.7);
    transform-origin: left center;
    transition: all 0.35s cubic-bezier(0.16, 1, 0.3, 1);
  }
  .inactive-lyric-sharp:hover {
    opacity: 0.85;
    transform: translate3d(0, 0, 0) scale(1.01);
    color: #ffffff;
  }

  /* Active Lyric Focus Glow */
  .active-lyric {
    filter: none;
    opacity: 1;
    transform: translate3d(0, 0, 0) scale(1.03);
    color: #ffffff;
    text-shadow: 0 0 24px var(--lyric-glow, #d0bcff), 0 0 48px var(--lyric-glow, #d0bcff);
    transform-origin: left center;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    transition: all 0.35s cubic-bezier(0.16, 1, 0.3, 1);
  }

  /* Word-by-Word Karaoke Tokens */
  .word-sung {
    color: #ffffff;
    text-shadow: 0 0 16px var(--lyric-glow, #d0bcff);
  }
  .word-unsung {
    color: rgba(255, 255, 255, 0.4);
  }
  .word-singing {
    filter: drop-shadow(0 0 12px var(--lyric-glow, #d0bcff));
  }

  .lyrics-scroll-container {
    scroll-behavior: smooth;
  }
</style>
