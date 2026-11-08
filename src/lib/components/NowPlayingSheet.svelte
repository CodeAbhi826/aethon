<script lang="ts">
  import { onDestroy } from 'svelte';
  import { fly, fade } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import { sanitizeArtist } from '../types';

  // Apple iOS / macOS sheet spring curve approximation
  function appleSheetEasing(t: number): number {
    return 1 - Math.pow(1 - t, 3.6);
  }

  function handleMinimize() {
    invoke('window_minimize').catch(() => {});
  }
  function handleMaximize() {
    invoke('window_toggle_maximize').catch(() => {});
  }
  function handleClose() {
    invoke('window_close').catch(() => {});
  }

  let canvasEl = $state<HTMLCanvasElement>();
  let lyricsContainerEl = $state<HTMLDivElement>();
  let animId: number | null = null;

  let activeView = $state<'auto' | 'turntable' | 'lyrics'>('auto');
  let userIsScrolling = $state(false);
  let scrollResumeTimeout: ReturnType<typeof setTimeout> | null = null;
  let isRomanizedMode = $state(true);
  let isOffsetOpen = $state(false);
  let isSourcePickerOpen = $state(false);
  let lastTrackId = $state<string | null>(null);
  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  const LYRICS_SOURCES: Array<{ id: typeof settingsState.lyricsSource; label: string }> = [
    { id: 'all', label: 'Auto' },
    { id: 'musixmatch', label: 'Musixmatch' },
    { id: 'apple', label: 'Apple Music' },
    { id: 'spotify', label: 'Spotify' },
    { id: 'ytm', label: 'YouTube Music' },
    { id: 'lrclib', label: 'LRCLIB' },
    { id: 'local_lrc', label: 'Local Only' },
  ];

  function cycleLyricsSource() {
    const idx = LYRICS_SOURCES.findIndex((s) => s.id === settingsState.lyricsSource);
    const nextIdx = (idx + 1) % LYRICS_SOURCES.length;
    selectLyricsSource(LYRICS_SOURCES[nextIdx].id);
  }

  function selectLyricsSource(source: typeof settingsState.lyricsSource) {
    settingsState.lyricsSource = source;
    settingsState.scheduleSave();
    if (playerState.currentTrack) {
      playerState.fetchLyrics(
        playerState.currentTrack.title,
        playerState.currentTrack.artist,
        playerState.currentTrack.duration,
        playerState.currentTrack.file_path,
        playerState.currentTrack.id
      );
    }
  }

  let audioQualityLabel = $derived.by(() => {
    const codec = (playerState.playback.codec || playerState.currentTrack?.format || '').toLowerCase();
    const sr = playerState.playback.sample_rate || playerState.currentTrack?.sample_rate;
    const isLossless =
      codec.includes('flac') ||
      codec.includes('alac') ||
      codec.includes('wav') ||
      codec.includes('pcm');
    if (isLossless) {
      return sr ? `Lossless • ${(sr / 1000).toFixed(1)}kHz` : 'Lossless';
    }
    const label = (playerState.playback.codec || playerState.currentTrack?.format || 'Opus').toUpperCase();
    return label.replace(' 256K', '').replace(' 320K', '');
  });

  let hiResArtwork = $derived.by(() => {
    const raw = playerState.currentTrack?.cover_art;
    if (!raw) return '';
    if (raw.includes('lh3.googleusercontent.com') && raw.includes('=')) {
      return raw.replace(/=w\d+-h\d+[^&?]*/, '=w544-h544-l90-rj');
    }
    return raw;
  });

  let scrollAnimId: number | null = null;
  let lastScrolledIndex = -1;

  $effect(() => {
    const currentId = playerState.currentTrack?.id ?? null;
    if (currentId !== lastTrackId) {
      lastTrackId = currentId;
      lastScrolledIndex = -1;
      userIsScrolling = false;
      if (scrollResumeTimeout) clearTimeout(scrollResumeTimeout);
      if (scrollAnimId !== null) {
        cancelAnimationFrame(scrollAnimId);
        scrollAnimId = null;
      }
      if (lyricsContainerEl) {
        lyricsContainerEl.scrollTop = 0;
      }
    }
  });

  function animateScrollTo(targetY: number) {
    if (!lyricsContainerEl) return;
    if (scrollAnimId !== null) {
      cancelAnimationFrame(scrollAnimId);
      scrollAnimId = null;
    }

    const startY = lyricsContainerEl.scrollTop;
    const distance = targetY - startY;
    if (Math.abs(distance) < 2) return;

    const startTime = performance.now();
    const duration = 380; // ms spring duration

    const step = (now: number) => {
      if (!lyricsContainerEl || userIsScrolling) {
        scrollAnimId = null;
        return;
      }
      const elapsed = now - startTime;
      const progress = Math.min(1, elapsed / duration);
      // Apple exponential spring easing
      const ease = 1 - Math.pow(1 - progress, 3.5);
      lyricsContainerEl.scrollTop = startY + distance * ease;

      if (progress < 1) {
        scrollAnimId = requestAnimationFrame(step);
      } else {
        lyricsContainerEl.scrollTop = targetY;
        scrollAnimId = null;
      }
    };

    scrollAnimId = requestAnimationFrame(step);
  }

  function handleUserScroll() {
    if (scrollAnimId !== null) {
      cancelAnimationFrame(scrollAnimId);
      scrollAnimId = null;
    }
    if (!playerState.lyrics.synced) return;
    userIsScrolling = true;
    if (scrollResumeTimeout) clearTimeout(scrollResumeTimeout);
    scrollResumeTimeout = setTimeout(() => {
      userIsScrolling = false;
      scrollToActiveLyric();
    }, 3500);
  }

  function resumeSync() {
    userIsScrolling = false;
    if (scrollResumeTimeout) clearTimeout(scrollResumeTimeout);
    scrollToActiveLyric();
  }

  function adjustOffset(delta: number) {
    settingsState.lyricOffsetSeconds = Math.round((settingsState.lyricOffsetSeconds + delta) * 10) / 10;
    settingsState.scheduleSave();
  }

  function resetOffset() {
    settingsState.lyricOffsetSeconds = 0.0;
    settingsState.scheduleSave();
  }

  let activeIndex = $derived.by(() => {
    if (!playerState.lyrics.synced || playerState.lyrics.lines.length === 0) return -1;
    const pos = playerState.playback.position + settingsState.lyricOffsetSeconds;
    let idx = -1;
    for (let i = 0; i < playerState.lyrics.lines.length; i++) {
      if (pos >= playerState.lyrics.lines[i].time) {
        idx = i;
      } else {
        break;
      }
    }
    return idx;
  });

  function getWordTokens(text: string): string[] {
    return text.trim().split(/\s+/).filter(Boolean);
  }

  function getWordProgress(lineIndex: number, wordIndex: number, totalWords: number, lineStart: number): number {
    const pos = playerState.playback.position + settingsState.lyricOffsetSeconds;
    const lines = playerState.lyrics.lines;
    const nextTime =
      lineIndex + 1 < lines.length
        ? lines[lineIndex + 1].time
        : Math.max(lineStart + 4.0, playerState.playback.duration || lineStart + 4.0);
    const lineDuration = Math.max(0.8, Math.min(8.0, (nextTime - lineStart) * 0.88));
    const lineProgress = Math.max(0, Math.min(1, (pos - lineStart) / lineDuration));
    const wordStart = wordIndex / totalWords;
    const wordEnd = (wordIndex + 1) / totalWords;
    if (lineProgress <= wordStart) return 0;
    if (lineProgress >= wordEnd) return 1;
    return (lineProgress - wordStart) / (wordEnd - wordStart);
  }

  function scrollToActiveLyric() {
    if (activeIndex >= 0 && lyricsContainerEl && !userIsScrolling) {
      const activeEl = lyricsContainerEl.querySelector(`[data-lyric-index="${activeIndex}"]`) as HTMLElement | null;
      if (activeEl) {
        const containerHeight = lyricsContainerEl.clientHeight;
        const targetScroll = Math.max(0, activeEl.offsetTop - containerHeight * 0.34 + activeEl.clientHeight / 2);
        animateScrollTo(targetScroll);
      }
    }
  }

  $effect(() => {
    if (activeIndex >= 0 && activeIndex !== lastScrolledIndex && !userIsScrolling && settingsState.autoScrollLyrics) {
      lastScrolledIndex = activeIndex;
      scrollToActiveLyric();
    }
  });

  $effect(() => {
    const canvas = canvasEl;
    const open = playerState.isNowPlayingOpen;
    const isPlaying = playerState.playback.is_playing;
    if (!open || !canvas || !settingsState.spectrumVisualizer) {
      if (animId !== null) {
        cancelAnimationFrame(animId);
        animId = null;
      }
      return;
    }

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const bars = 36;
    let phases = Array.from({ length: bars }, () => Math.random() * Math.PI * 2);

    const drawFrame = (playing: boolean) => {
      const w = canvas.width;
      const h = canvas.height;
      ctx.clearRect(0, 0, w, h);

      const barWidth = w / bars - 3;

      for (let i = 0; i < bars; i++) {
        if (playing) {
          phases[i] += 0.05 + (i % 5) * 0.015;
        }
        const centerFactor = 1 - Math.abs(i - bars / 2) / (bars / 2);
        const amp = playing
          ? (Math.sin(phases[i]) * 0.4 + 0.5) * centerFactor * h * 0.72 + 3
          : 3;

        const x = i * (barWidth + 3);
        const y = (h - amp) / 2;

        ctx.fillStyle = playerState.accentColor || '#d0bcff';
        ctx.globalAlpha = playing ? 0.35 + centerFactor * 0.45 : 0.2;
        ctx.beginPath();
        ctx.roundRect(x, y, barWidth, amp, 2);
        ctx.fill();
      }
    };

    if (!isPlaying) {
      if (animId !== null) {
        cancelAnimationFrame(animId);
        animId = null;
      }
      drawFrame(false); // Halt visualizer immediately when paused
      return;
    }

    const render = () => {
      drawFrame(true);
      animId = requestAnimationFrame(render);
    };

    animId = requestAnimationFrame(render);

    return () => {
      if (animId !== null) {
        cancelAnimationFrame(animId);
        animId = null;
      }
    };
  });

  onDestroy(() => {
    if (animId !== null) cancelAnimationFrame(animId);
    if (scrollAnimId !== null) cancelAnimationFrame(scrollAnimId);
    if (scrollResumeTimeout) clearTimeout(scrollResumeTimeout);
  });
</script>

{#if playerState.isNowPlayingOpen}
  <div
    role="dialog"
    aria-modal="true"
    aria-label="Now Playing"
    in:fly={{ y: 560, duration: 420, opacity: 0.2, easing: appleSheetEasing }}
    out:fly={{ y: 560, duration: 340, opacity: 0, easing: appleSheetEasing }}
    class="fixed inset-0 z-50 flex flex-col justify-between px-4 sm:px-8 py-3 sm:py-5 overflow-hidden select-none will-change-transform"
    style="background-color: #08080c; --artwork-glow: {playerState.accentColor || '#d0bcff'};"
  >
    <!-- Hardware GPU Composited Dual Radial Gradient Mesh Backdrop -->
    <div class="absolute inset-0 pointer-events-none overflow-hidden z-0">
      <!-- 100% Solid Deep Base ensuring underlying app is completely hidden -->
      <div class="absolute inset-0 bg-[#08080c]"></div>
      <div
        class="absolute -top-[15%] -left-[10%] w-[75vw] h-[75vw] max-w-[720px] max-h-[720px] rounded-full transition-colors duration-1000 gpu-layer opacity-40"
        style="background: radial-gradient(circle, var(--artwork-glow, #d0bcff) 0%, transparent 70%);"
      ></div>
      <div
        class="absolute -bottom-[20%] -right-[10%] w-[70vw] h-[70vw] max-w-[680px] max-h-[680px] rounded-full transition-colors duration-1000 gpu-layer opacity-30"
        style="background: radial-gradient(circle, var(--md-sys-mesh-2, rgba(41, 128, 185, 0.4)) 0%, transparent 70%);"
      ></div>
      <div class="absolute inset-0 bg-gradient-to-b from-[#08080c]/60 via-transparent to-[#08080c]"></div>
    </div>

    <!-- Top Bar -->
    <header class="flex items-center justify-between z-20 shrink-0 pb-1">
      <div class="flex items-center gap-2.5 flex-1 min-w-0">
        <button
          onclick={() => (playerState.isNowPlayingOpen = false)}
          class="w-8 h-8 rounded-full bg-white/[0.07] hover:bg-white/[0.14] active:scale-90 flex items-center justify-center text-white/70 hover:text-white transition-all cursor-pointer shrink-0"
          title="Collapse Now Playing"
        >
          <span class="material-symbols-rounded text-[20px]">keyboard_arrow_down</span>
        </button>

        <!-- Clean Mode Switcher: Split | Artwork | Lyrics -->
        <div class="flex items-center bg-white/[0.06] border border-white/[0.08] rounded-full p-0.5 shrink-0">
          <button
            type="button"
            onclick={() => (activeView = 'auto')}
            class="px-3 py-1 rounded-full text-[11px] font-medium transition-all cursor-pointer {activeView === 'auto'
              ? 'bg-white/15 text-white shadow-sm font-semibold'
              : 'text-white/55 hover:text-white'}"
          >
            Split
          </button>
          <button
            type="button"
            onclick={() => (activeView = 'turntable')}
            class="px-3 py-1 rounded-full text-[11px] font-medium transition-all cursor-pointer {activeView === 'turntable'
              ? 'bg-white/15 text-white shadow-sm font-semibold'
              : 'text-white/55 hover:text-white'}"
          >
            Artwork
          </button>
          <button
            type="button"
            onclick={() => (activeView = 'lyrics')}
            class="px-3 py-1 rounded-full text-[11px] font-medium transition-all cursor-pointer {activeView === 'lyrics'
              ? 'bg-white/15 text-white shadow-sm font-semibold'
              : 'text-white/55 hover:text-white'}"
          >
            Lyrics
          </button>
        </div>

        <div data-tauri-drag-region class="flex-1 h-8 cursor-default"></div>
      </div>

      <!-- Right Actions + Window Controls -->
      <div class="flex items-center gap-1.5 shrink-0">
        {#if settingsState.showAudioQualityBadge && playerState.currentTrack}
          <span class="bg-white/[0.07] border border-white/10 text-white/70 text-[10px] tracking-wider uppercase px-2.5 py-0.5 rounded-full font-semibold hidden sm:inline-block">
            {audioQualityLabel}
          </span>
        {/if}

        <button
          onclick={() => (playerState.isQueueOpen = !playerState.isQueueOpen)}
          class="w-8 h-8 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white/70 hover:text-white transition-colors cursor-pointer"
          title="Playing Next"
        >
          <span class="material-symbols-rounded text-[18px]">queue_music</span>
        </button>

        <button
          onclick={() => (playerState.isEqualizerOpen = true)}
          class="w-8 h-8 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white/70 hover:text-white transition-colors cursor-pointer"
          title="Equalizer"
        >
          <span class="material-symbols-rounded text-[18px]">tune</span>
        </button>

        <div class="h-3.5 w-[1px] bg-white/10 mx-1"></div>

        <div class="flex items-center gap-0.5">
          <button
            onclick={handleMinimize}
            aria-label="Minimize Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-white/60 hover:text-white hover:bg-white/[0.08] transition-colors cursor-pointer"
          >
            <span class="material-symbols-rounded text-[15px]">remove</span>
          </button>
          <button
            onclick={handleMaximize}
            aria-label="Maximize Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-white/60 hover:text-white hover:bg-white/[0.08] transition-colors cursor-pointer"
          >
            <span class="material-symbols-rounded text-[14px]">crop_square</span>
          </button>
          <button
            onclick={handleClose}
            aria-label="Close Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-white/60 hover:text-white hover:bg-red-500/80 transition-colors cursor-pointer"
          >
            <span class="material-symbols-rounded text-[15px]">close</span>
          </button>
        </div>
      </div>
    </header>

    <!-- Main Content Area -->
    <div class="flex-1 flex overflow-hidden z-10 min-h-0 py-2 sm:py-4 gap-6 lg:gap-12 items-center">
      <!-- Left Column: Apple Music / ArchiveTune V10 Editorial Floating Artwork + Transport -->
      {#if activeView === 'auto' || activeView === 'turntable'}
        <div
          class="flex flex-col items-center justify-between h-full min-h-0 {activeView === 'auto'
            ? 'w-full md:w-[46%] lg:w-[42%] shrink-0'
            : 'w-full max-w-md mx-auto'} py-2 px-2 sm:px-4"
        >
          <!-- Editorial Floating Squircle Artwork Showcase -->
          <div class="flex-1 flex items-center justify-center w-full min-h-0 my-1">
            <div
              class="editorial-artwork-frame relative rounded-[28px] overflow-hidden ring-1 ring-white/15 transition-all duration-500 [transition-timing-function:cubic-bezier(0.22,1,0.36,1)] {playerState.playback.is_playing
                ? 'scale-100 translate-y-0 artwork-shadow-playing'
                : 'scale-[0.86] translate-y-1 opacity-90 artwork-shadow-paused'}"
            >
              {#if hiResArtwork}
                <img
                  src={hiResArtwork}
                  alt={playerState.currentTrack?.title}
                  class="w-full h-full object-cover select-none"
                  draggable="false"
                />
              {:else}
                <div class="w-full h-full bg-gradient-to-br from-white/10 to-white/[0.02] flex items-center justify-center text-white/30">
                  <span class="material-symbols-rounded text-6xl">music_note</span>
                </div>
              {/if}

              <!-- Subtle top glass specular highlight -->
              <div class="absolute inset-0 bg-gradient-to-tr from-transparent via-transparent to-white/[0.07] pointer-events-none"></div>
            </div>
          </div>

          <!-- Track Metadata, Visualizer & Scrubber -->
          <div class="w-full max-w-sm flex flex-col shrink-0 space-y-3 pt-2">
            <!-- Title, Artist & Favorite Button -->
            <div class="flex items-center justify-between gap-3 w-full">
              <div class="min-w-0 flex-1">
                <h2 class="font-display font-bold text-lg sm:text-xl text-white truncate">
                  {playerState.currentTrack?.title || 'Not Playing'}
                </h2>
                <button
                  type="button"
                  onclick={() => {
                    if (playerState.currentTrack?.artist) {
                      playerState.isNowPlayingOpen = false;
                      playerState.openArtist(playerState.currentTrack.artist);
                    }
                  }}
                  class="text-xs sm:text-sm text-white/65 hover:text-white truncate mt-0.5 block text-left transition-colors cursor-pointer max-w-full"
                >
                  {playerState.currentTrack
                    ? sanitizeArtist(playerState.currentTrack.artist)
                    : 'Select a song'}
                </button>
              </div>

              {#if playerState.currentTrack}
                <div class="flex items-center gap-1 shrink-0">
                  <button
                    type="button"
                    onclick={() =>
                      playerState.currentTrack &&
                      playerState.toggleFavorite(playerState.currentTrack.id)}
                    class="w-9 h-9 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center transition-colors cursor-pointer {playerState.currentTrack.favorite
                      ? 'text-rose-400'
                      : 'text-white/60 hover:text-white'}"
                    title={playerState.currentTrack.favorite ? 'Remove from Favorites' : 'Add to Favorites'}
                  >
                    <span class="material-symbols-rounded text-[19px] {playerState.currentTrack.favorite ? 'filled' : ''}">
                      favorite
                    </span>
                  </button>
                  <button
                    type="button"
                    onclick={(e) =>
                      playerState.currentTrack &&
                      playerState.openContextMenu(playerState.currentTrack, e.clientX, e.clientY)}
                    class="w-9 h-9 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white/60 hover:text-white transition-colors cursor-pointer"
                    title="More Options"
                  >
                    <span class="material-symbols-rounded text-[19px]">more_horiz</span>
                  </button>
                </div>
              {/if}
            </div>

            {#if settingsState.spectrumVisualizer}
              <div class="w-full h-5 flex items-center justify-center">
                <canvas bind:this={canvasEl} width="320" height="20" class="w-full h-full opacity-75"></canvas>
              </div>
            {/if}

            <!-- Fluid Scrubber with Buffering Shimmer -->
            <div class="w-full space-y-1.5">
              <div class="relative w-full h-1.5 bg-white/[0.12] hover:h-2 rounded-full overflow-hidden cursor-pointer transition-all">
                {#if isBuffering}
                  <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/80 to-transparent animate-shimmer-bar"></div>
                {:else}
                  <div
                    class="absolute inset-0 bg-white rounded-full origin-left will-change-transform"
                    style="transform: scaleX({playerState.progressPercent / 100});"
                  ></div>
                {/if}
                <input
                  type="range"
                  min="0"
                  max={playerState.playback.duration || (playerState.currentTrack?.duration ?? 100) || 100}
                  value={playerState.playback.position}
                  oninput={(e) => playerState.seek(parseFloat(e.currentTarget.value))}
                  class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                />
              </div>
              <div class="flex justify-between text-[11px] font-mono text-white/50">
                <span>{playerState.formattedTime.current}</span>
                <span>{isBuffering ? 'Loading...' : playerState.formattedTime.total}</span>
              </div>
            </div>

            <!-- Transport Controls with Single ArchiveTune Circular Spinner -->
            <div class="flex items-center justify-between px-2 pt-1">
              <button
                onclick={() => playerState.toggleShuffle()}
                class="p-2 rounded-full transition-colors cursor-pointer {playerState.shuffleMode
                  ? 'text-[var(--md-sys-primary)] bg-white/10'
                  : 'text-white/45 hover:text-white'}"
                title="Shuffle"
              >
                <span class="material-symbols-rounded text-[20px]">shuffle</span>
              </button>

              <div class="flex items-center gap-6">
                <button
                  onclick={() => playerState.playPrevious()}
                  class="text-white/80 hover:text-white active:scale-90 transition-all cursor-pointer"
                  title="Previous"
                >
                  <span class="material-symbols-rounded text-[30px] filled">skip_previous</span>
                </button>

                <div class="relative flex items-center justify-center">
                  <button
                    onclick={() => playerState.togglePlay()}
                    class="sheet-squircle-btn w-14 h-14 {playerState.playback.is_playing ? 'is-playing' : ''} bg-white text-black flex items-center justify-center active:scale-90 hover:scale-105 cursor-pointer shadow-xl shadow-black/40"
                    title={isBuffering
                      ? 'Buffering...'
                      : playerState.playback.is_playing
                        ? 'Pause'
                        : 'Play'}
                  >
                    {#if isBuffering}
                      <svg class="w-7 h-7 archivetune-spinner-svg text-black" viewBox="0 0 36 36" fill="none">
                        <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3" />
                        <circle
                          class="archivetune-spinner-arc-secondary"
                          cx="18"
                          cy="18"
                          r="14"
                          stroke="currentColor"
                          stroke-width="3"
                        />
                        <circle
                          class="archivetune-spinner-arc"
                          cx="18"
                          cy="18"
                          r="14"
                          stroke="currentColor"
                          stroke-width="3"
                        />
                      </svg>
                    {:else if playerState.playback.is_playing}
                      <svg viewBox="0 0 960 960" class="w-7 h-7 fill-current" xmlns="http://www.w3.org/2000/svg">
                        <path d="M640,760Q607,760 583.5,736.5Q560,713 560,680L560,280Q560,247 583.5,223.5Q607,200 640,200L640,200Q673,200 696.5,223.5Q720,247 720,280L720,680Q720,713 696.5,736.5Q673,760 640,760ZM320,760Q287,760 263.5,736.5Q240,713 240,680L240,280Q240,247 263.5,223.5Q287,200 320,200L320,200Q353,200 376.5,223.5Q400,247 400,280L400,680Q400,713 376.5,736.5Q353,760 320,760Z" />
                      </svg>
                    {:else}
                      <svg viewBox="42.5 0 960 960" class="w-7 h-7 fill-current" xmlns="http://www.w3.org/2000/svg">
                        <path d="M320,687L320,273Q320,256 332,244.5Q344,233 360,233Q365,233 370.5,234.5Q376,236 381,239L707,446Q716,452 720.5,461Q725,470 725,480Q725,490 720.5,499Q716,508 707,514L381,721Q376,724 370.5,725.5Q365,727 360,727Q344,727 332,715.5Q320,704 320,687Z" />
                      </svg>
                    {/if}
                  </button>
                </div>

                <button
                  onclick={() => playerState.playNext()}
                  class="text-white/80 hover:text-white active:scale-90 transition-all cursor-pointer"
                  title="Next"
                >
                  <span class="material-symbols-rounded text-[30px] filled">skip_next</span>
                </button>
              </div>

              <button
                onclick={() => playerState.toggleRepeat()}
                class="p-2 rounded-full transition-colors cursor-pointer {playerState.repeatMode !== 'off'
                  ? 'text-[var(--md-sys-primary)] bg-white/10'
                  : 'text-white/45 hover:text-white'}"
                title="Repeat"
              >
                <span class="material-symbols-rounded text-[20px]">
                  {playerState.repeatMode === 'one' ? 'repeat_one' : 'repeat'}
                </span>
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Right Column: Apple Music / ArchiveTune Synced Lyrics + Romanization -->
      {#if activeView === 'auto' || activeView === 'lyrics'}
        <div
          class="flex-1 flex flex-col h-full overflow-hidden min-h-0 relative {activeView === 'auto'
            ? 'w-full md:flex-1'
            : 'w-full max-w-3xl mx-auto'} gpu-layer"
        >
          <div class="flex items-center justify-between pb-2 mb-1 shrink-0 px-3">
            <div class="flex items-center gap-2 relative">
              <button
                type="button"
                onclick={() => (isSourcePickerOpen = !isSourcePickerOpen)}
                class="text-[10px] font-medium px-2.5 py-0.5 rounded-full bg-white/[0.06] hover:bg-white/[0.12] text-white/70 border border-white/[0.08] hover:border-white/20 transition-all flex items-center gap-1 cursor-pointer"
                title="Change Lyrics Source Priority"
              >
                <span>{playerState.lyrics.provider || 'Lyrics'}</span>
                {#if settingsState.lyricsSource !== 'all'}
                  <span class="text-[9px] text-amber-400 font-mono uppercase">({settingsState.lyricsSource})</span>
                {/if}
                <span class="material-symbols-rounded text-[11px] opacity-60">unfold_more</span>
              </button>

              {#if isSourcePickerOpen}
                <!-- Backdrop to dismiss -->
                <button
                  type="button"
                  tabindex="-1"
                  aria-label="Close provider picker"
                  class="fixed inset-0 z-40 bg-transparent cursor-default border-none p-0 outline-none"
                  onclick={() => (isSourcePickerOpen = false)}
                ></button>
                <div class="absolute top-7 left-0 z-50 min-w-[170px] bg-[#1a1a1f]/95 backdrop-blur-2xl border border-white/15 rounded-xl shadow-2xl p-1.5 flex flex-col gap-0.5">
                  <div class="px-2 py-1 text-[9px] uppercase tracking-wider text-white/40 font-semibold">Lyrics Source Priority</div>
                  {#each LYRICS_SOURCES as src}
                    <button
                      type="button"
                      onclick={() => {
                        selectLyricsSource(src.id);
                        isSourcePickerOpen = false;
                      }}
                      class="px-2.5 py-1.5 rounded-lg text-left text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {settingsState.lyricsSource === src.id ? 'bg-white/15 text-white font-semibold' : 'text-white/70 hover:bg-white/10 hover:text-white'}"
                    >
                      <span>{src.label}</span>
                      {#if settingsState.lyricsSource === src.id}
                        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary,#60a5fa)]">check</span>
                      {/if}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>

            <div class="flex items-center gap-1.5">
              {#if userIsScrolling}
                <button
                  type="button"
                  onclick={resumeSync}
                  class="text-[11px] font-semibold px-3 py-1 rounded-full bg-white text-black flex items-center gap-1 cursor-pointer shadow-sm"
                >
                  <span class="material-symbols-rounded text-[13px]">my_location</span>
                  <span>Sync</span>
                </button>
              {/if}

              <button
                type="button"
                onclick={() => (isRomanizedMode = !isRomanizedMode)}
                class="px-2.5 py-1 rounded-full text-[11px] font-semibold flex items-center gap-1 border transition-all cursor-pointer {isRomanizedMode
                  ? 'bg-white/15 text-white border-white/25'
                  : 'bg-white/[0.04] text-white/50 hover:text-white border-white/10'}"
                title="Toggle Romanized Phonetic Lyrics"
              >
                <span class="font-sans text-[11px] font-bold">Aあ</span>
                <span>Romanized</span>
              </button>

              <button
                type="button"
                onclick={() => (isOffsetOpen = !isOffsetOpen)}
                class="px-2.5 py-1 rounded-full text-[11px] font-mono flex items-center gap-1 border transition-all cursor-pointer {Math.abs(settingsState.lyricOffsetSeconds) > 0.05
                  ? 'bg-amber-500/20 text-amber-300 border-amber-500/30'
                  : 'bg-white/[0.04] text-white/50 hover:text-white border-white/10'}"
                title="Timing Offset"
              >
                <span>{settingsState.lyricOffsetSeconds > 0 ? '+' : ''}{settingsState.lyricOffsetSeconds.toFixed(1)}s</span>
              </button>
            </div>
          </div>

          {#if isOffsetOpen}
            <div class="flex items-center justify-between bg-black/50 border border-white/10 rounded-2xl px-3.5 py-2 mx-3 mb-2 text-xs text-white/80">
              <span class="text-[11px] text-white/60 font-medium">Timing Offset</span>
              <div class="flex items-center gap-2">
                <button onclick={() => adjustOffset(-0.5)} class="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-[10px] font-mono cursor-pointer">-0.5s</button>
                <button onclick={() => adjustOffset(-0.1)} class="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-[10px] font-mono cursor-pointer">-0.1s</button>
                <span class="font-mono text-xs w-11 text-center font-bold text-white">
                  {settingsState.lyricOffsetSeconds > 0 ? '+' : ''}{settingsState.lyricOffsetSeconds.toFixed(1)}s
                </span>
                <button onclick={() => adjustOffset(0.1)} class="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-[10px] font-mono cursor-pointer">+0.1s</button>
                <button onclick={() => adjustOffset(0.5)} class="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-[10px] font-mono cursor-pointer">+0.5s</button>
                <button onclick={resetOffset} class="ml-1 text-[10px] px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-white/70 hover:text-white cursor-pointer">Reset</button>
              </div>
            </div>
          {/if}

          <div
            bind:this={lyricsContainerEl}
            onwheel={handleUserScroll}
            ontouchmove={handleUserScroll}
            role="region"
            aria-label="Lyrics"
            class="flex-1 overflow-y-auto px-3 sm:px-6 py-8 space-y-3 scrollbar-none contain-gpu"
          >
            {#if playerState.isLoadingLyrics}
              <div class="h-full flex flex-col items-center justify-center text-center text-white/50 py-16 gap-3">
                <svg class="w-8 h-8 archivetune-spinner-svg text-white/60" viewBox="0 0 24 24" fill="none">
                  <circle class="opacity-15" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                  <circle class="archivetune-spinner-arc-secondary" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                  <circle class="archivetune-spinner-arc" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                </svg>
                <p class="text-xs font-medium tracking-wide text-white/45 uppercase">Loading Lyrics...</p>
              </div>
            {:else if playerState.lyrics.synced && playerState.lyrics.lines.length > 0}
              <!-- Instrumental Intro Rhythm Dots (>= 4.0s before first line) -->
              {#if playerState.lyrics.lines[0].time >= 4.0}
                {@const isIntroActive = (playerState.playback.position + settingsState.lyricOffsetSeconds) < playerState.lyrics.lines[0].time}
                <div class="py-4 px-4 flex items-center gap-2 transition-opacity duration-300 {isIntroActive ? 'opacity-85' : 'opacity-20'}">
                  <span class="w-2.5 h-2.5 rounded-full bg-white interlude-dot-1 inline-block"></span>
                  <span class="w-2.5 h-2.5 rounded-full bg-white interlude-dot-2 inline-block"></span>
                  <span class="w-2.5 h-2.5 rounded-full bg-white interlude-dot-3 inline-block"></span>
                </div>
              {/if}

              {#each playerState.lyrics.lines as line, i}
                {@const dist = i - activeIndex}
                {@const isCur = dist === 0}
                
                <!-- Mid-song Instrumental Break Rhythm Dots (gap >= 10.0s between lines) -->
                {#if i > 0 && (line.time - playerState.lyrics.lines[i - 1].time) >= 10.0}
                  {@const curPos = playerState.playback.position + settingsState.lyricOffsetSeconds}
                  {@const isBreakActive = curPos >= playerState.lyrics.lines[i - 1].time + 2.0 && curPos < line.time}
                  <div class="py-3 px-4 flex items-center gap-2 transition-opacity duration-300 {isBreakActive ? 'opacity-85' : 'opacity-20'}">
                    <span class="w-2 h-2 rounded-full bg-white interlude-dot-1 inline-block"></span>
                    <span class="w-2 h-2 rounded-full bg-white interlude-dot-2 inline-block"></span>
                    <span class="w-2 h-2 rounded-full bg-white interlude-dot-3 inline-block"></span>
                  </div>
                {/if}

                <button
                  type="button"
                  data-lyric-index={i}
                  onclick={() => {
                    playerState.seek(line.time - settingsState.lyricOffsetSeconds);
                    resumeSync();
                  }}
                  class="w-full text-left font-display font-bold tracking-tight py-2.5 px-4 rounded-2xl block text-xl sm:text-2xl md:text-[28px] leading-snug cursor-pointer select-none gpu-lyric cascade-lyric {isCur
                    ? 'active-lyric'
                    : dist === 1
                      ? 'cascade-dist-1'
                      : dist === 2
                        ? 'cascade-dist-2'
                        : dist >= 3
                          ? 'cascade-dist-far'
                          : dist === -1
                            ? 'cascade-dist-past-1'
                            : 'cascade-dist-past-far'}"
                  style={isCur
                    ? `--lyric-glow: ${playerState.accentColor || 'var(--md-sys-primary)'};`
                    : ''}
                >
                  {#if isCur && settingsState.wordByWordTracking}
                    {@const words = getWordTokens(line.text)}
                    <span class="inline-flex flex-wrap gap-x-2">
                      {#each words as word, wIdx}
                        {@const wp = getWordProgress(i, wIdx, words.length, line.time)}
                        <span
                          class="word-token transition-all duration-150 inline-block will-change-transform {wp >= 1
                            ? 'word-sung'
                            : wp <= 0
                              ? 'word-unsung'
                              : 'word-singing'}"
                          style={wp > 0 && wp < 1
                            ? `opacity: ${0.45 + wp * 0.55}; transform: translate3d(0, -1.5px, 0);`
                            : ''}
                        >
                          {word}
                        </span>
                      {/each}
                    </span>
                  {:else}
                    <span class="block">{line.text}</span>
                  {/if}

                  {#if isRomanizedMode && line.romanized && line.romanized !== line.text}
                    <span
                      class="block text-sm sm:text-base font-sans font-medium tracking-normal mt-1 {isCur
                        ? 'text-white/80'
                        : 'text-white/45'}"
                    >
                      {line.romanized}
                    </span>
                  {/if}

                  {#if isRomanizedMode && line.translation}
                    <span
                      class="block text-xs sm:text-sm font-sans font-normal tracking-normal mt-0.5 {isCur
                        ? 'text-white/60'
                        : 'text-white/35'}"
                    >
                      {line.translation}
                    </span>
                  {/if}
                </button>
              {/each}
            {:else if playerState.lyrics.plain_text}
              <div class="text-white/75 whitespace-pre-line text-base sm:text-lg font-medium leading-relaxed px-4 py-2">
                {playerState.lyrics.plain_text}
              </div>
            {:else}
              <div class="h-full flex flex-col items-center justify-center text-center text-white/35 py-16">
                <span class="material-symbols-rounded text-[44px] mb-2 opacity-40">lyrics</span>
                <p class="text-sm font-medium text-white/55">Lyrics aren't available for this song</p>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .editorial-artwork-frame {
    width: min(340px, 40vh);
    height: min(340px, 40vh);
    aspect-ratio: 1 / 1;
    will-change: transform, box-shadow;
  }

  .artwork-shadow-playing {
    box-shadow:
      0 32px 80px -12px color-mix(in srgb, var(--artwork-glow, #d0bcff) 48%, transparent),
      0 18px 40px -10px rgba(0, 0, 0, 0.85);
  }

  .artwork-shadow-paused {
    box-shadow: 0 16px 36px -12px rgba(0, 0, 0, 0.75);
  }

  @keyframes shimmerBar {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(300%);
    }
  }
  .animate-shimmer-bar {
    animation: shimmerBar 1.25s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  }

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

  /* Distance-Graded GPU Spring Cascade (zero CPU filter passes) */
  .cascade-lyric {
    transform-origin: left center;
    transition: opacity 0.32s cubic-bezier(0.16, 1, 0.3, 1), transform 0.32s cubic-bezier(0.16, 1, 0.3, 1), color 0.25s ease;
  }

  .active-lyric {
    opacity: 1;
    transform: translate3d(0, 0, 0) scale(1.025);
    color: #ffffff;
    text-shadow: 0 0 28px color-mix(in srgb, var(--lyric-glow, #d0bcff) 55%, transparent);
    background: rgba(255, 255, 255, 0.055);
  }

  .cascade-dist-1 {
    opacity: 0.54;
    transform: translate3d(0, 0, 0) scale(0.985);
    color: rgba(255, 255, 255, 0.88);
  }

  .cascade-dist-2 {
    opacity: 0.34;
    transform: translate3d(0, 0, 0) scale(0.97);
    color: rgba(255, 255, 255, 0.8);
  }

  .cascade-dist-far {
    opacity: 0.20;
    transform: translate3d(0, 0, 0) scale(0.955);
    color: rgba(255, 255, 255, 0.7);
  }

  .cascade-dist-past-1 {
    opacity: 0.26;
    transform: translate3d(0, 0, 0) scale(0.97);
    color: rgba(255, 255, 255, 0.7);
  }

  .cascade-dist-past-far {
    opacity: 0.16;
    transform: translate3d(0, 0, 0) scale(0.955);
    color: rgba(255, 255, 255, 0.65);
  }

  .word-sung {
    color: #ffffff;
    text-shadow: 0 0 16px var(--lyric-glow, #d0bcff);
  }
  .word-unsung {
    color: rgba(255, 255, 255, 0.42);
  }
  .word-singing {
    filter: drop-shadow(0 0 10px var(--lyric-glow, #d0bcff));
  }

  .sheet-squircle-btn {
    border-radius: 50%;
    transition: border-radius 260ms cubic-bezier(0.2, 0, 0, 1), transform 150ms cubic-bezier(0.2, 0, 0, 1);
    will-change: border-radius, transform;
  }
  .sheet-squircle-btn.is-playing {
    border-radius: 28%;
  }
</style>
