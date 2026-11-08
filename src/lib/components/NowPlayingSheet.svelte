<script lang="ts">
  import { onDestroy } from 'svelte';
  import { fly } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import { sanitizeArtist, handleThumbnailError, handleThumbnailLoad } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from './ArtistLinks.svelte';

  // Apple iOS / macOS sheet spring curve
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

  const visualizerBars = Array.from({ length: 36 }, (_, i) => {
    const centerFactor = 1 - Math.abs(i - 17.5) / 17.5;
    const heightPercent = Math.max(20, Math.round((0.25 + 0.75 * centerFactor) * 100));
    const animVariant = (i % 4) + 1;
    const duration = (0.65 + (i % 7) * 0.1).toFixed(2);
    const delay = ((i % 5) * 0.09).toFixed(2);
    const opacity = (0.38 + centerFactor * 0.48).toFixed(2);
    return {
      id: i,
      height: `${heightPercent}%`,
      animClass: `anim-spectrum-${animVariant}`,
      duration: `${duration}s`,
      delay: `${delay}s`,
      opacity,
    };
  });

  let lyricsContainerEl = $state<HTMLDivElement>();
  let clockRafId: number | null = null;

  let activeView = $state<'auto' | 'turntable' | 'lyrics'>('auto');
  let userIsScrolling = $state(false);
  let scrollResumeTimeout: ReturnType<typeof setTimeout> | null = null;
  // Romanized phonetic lyrics OFF by default as requested
  let isRomanizedMode = $state(false);
  let isOffsetOpen = $state(false);
  let lastTrackId = $state<string | null>(null);
  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  // High-precision 60fps local clock interpolator for sub-16ms lyric sync accuracy
  let smoothPosition = $state(0);
  let lastAnchorPos = 0;
  let lastAnchorTime = 0;

  $effect(() => {
    const pos = playerState.playback.position;
    lastAnchorPos = pos;
    lastAnchorTime = typeof performance !== 'undefined' ? performance.now() : 0;
    smoothPosition = pos;
  });

  $effect(() => {
    const open = playerState.isNowPlayingOpen;
    const playing = playerState.playback.is_playing && !isBuffering;
    if (!open || !playing) {
      if (clockRafId !== null) {
        cancelAnimationFrame(clockRafId);
        clockRafId = null;
      }
      smoothPosition = playerState.playback.position;
      return;
    }

    lastAnchorPos = playerState.playback.position;
    lastAnchorTime = performance.now();

    const tick = (now: number) => {
      const elapsedSec = Math.max(0, (now - lastAnchorTime) / 1000);
      const maxDur = playerState.playback.duration || (playerState.currentTrack?.duration ?? 0);
      const nextPos = lastAnchorPos + Math.min(elapsedSec, 0.25);
      smoothPosition = maxDur > 0 ? Math.min(maxDur, nextPos) : nextPos;
      clockRafId = requestAnimationFrame(tick);
    };

    clockRafId = requestAnimationFrame(tick);

    return () => {
      if (clockRafId !== null) {
        cancelAnimationFrame(clockRafId);
        clockRafId = null;
      }
    };
  });

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
    return getHighResArtwork(
      playerState.currentTrack?.cover_art,
      playerState.currentTrack?.id,
      544
    );
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
    if (Math.abs(distance) < 1.5) return;

    const startTime = performance.now();
    const duration = 340;

    const step = (now: number) => {
      if (!lyricsContainerEl || userIsScrolling) {
        scrollAnimId = null;
        return;
      }
      const elapsed = now - startTime;
      const progress = Math.min(1, elapsed / duration);
      const ease = 1 - Math.pow(1 - progress, 3.4);
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
    // Slight 0.08s visual lead so the lyric line illuminates right as the vocal syllable begins
    const pos = smoothPosition + settingsState.lyricOffsetSeconds + 0.08;
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

  function scrollToActiveLyric() {
    if (activeIndex >= 0 && lyricsContainerEl && !userIsScrolling) {
      const activeEl = lyricsContainerEl.querySelector(`[data-lyric-index="${activeIndex}"]`) as HTMLElement | null;
      if (activeEl) {
        const containerHeight = lyricsContainerEl.clientHeight;
        const targetScroll = Math.max(0, activeEl.offsetTop - containerHeight * 0.36 + activeEl.clientHeight / 2);
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

  onDestroy(() => {

    if (clockRafId !== null) cancelAnimationFrame(clockRafId);
    if (scrollAnimId !== null) cancelAnimationFrame(scrollAnimId);
    if (scrollResumeTimeout) clearTimeout(scrollResumeTimeout);
  });
</script>

{#if playerState.isNowPlayingOpen}
  <div
    role="dialog"
    aria-modal="true"
    aria-label="Now Playing"
    in:fly={{ y: 480, duration: 320, opacity: 0, easing: appleSheetEasing }}
    out:fly={{ y: 480, duration: 250, opacity: 0, easing: appleSheetEasing }}
    class="fixed inset-0 z-50 bg-[#08080b] flex flex-col justify-between px-5 sm:px-8 py-3.5 sm:py-5 overflow-hidden select-none will-change-transform"
    style="background-color: #08080b; --artwork-glow: {playerState.accentColor || 'var(--md-sys-primary)'};"
  >
    <!-- Pure Zero-Blur GPU Radial Gradient Mesh Backdrop Blending Across Full Sheet -->
    <div
      class="absolute inset-0 pointer-events-none overflow-hidden z-0"
      style="background:
        radial-gradient(circle at 22% 32%, color-mix(in srgb, var(--artwork-glow, #d0bcff) 32%, transparent) 0%, transparent 58%),
        radial-gradient(circle at 76% 68%, color-mix(in srgb, var(--artwork-glow, #d0bcff) 20%, transparent) 0%, transparent 62%),
        #08080b;"
    ></div>

    <!-- Top Bar -->
    <header class="flex items-center justify-between z-20 shrink-0 pb-1">
      <div class="flex items-center gap-2.5 flex-1 min-w-0">
        <button
          onclick={() => (playerState.isNowPlayingOpen = false)}
          class="w-8 h-8 rounded-full bg-white/10 hover:bg-white/15 active:scale-90 flex items-center justify-center text-white/75 hover:text-white transition-transform cursor-pointer shrink-0"
          title="Collapse Now Playing"
        >
          <span class="material-symbols-rounded text-[20px]">keyboard_arrow_down</span>
        </button>

        <!-- Dynamic Theme-Tinted Mode Switcher: Split | Artwork | Lyrics -->
        <div class="flex items-center bg-white/[0.06] border border-white/10 rounded-full p-0.5 shrink-0">
          <button
            type="button"
            onclick={() => (activeView = 'auto')}
            class="px-3 py-1 rounded-full text-[11px] font-medium transition-colors cursor-pointer {activeView === 'auto'
              ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] shadow-sm font-semibold'
              : 'text-white/60 hover:text-white'}"
          >
            Split
          </button>
          <button
            type="button"
            onclick={() => (activeView = 'turntable')}
            class="px-3 py-1 rounded-full text-[11px] font-medium transition-colors cursor-pointer {activeView === 'turntable'
              ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] shadow-sm font-semibold'
              : 'text-white/60 hover:text-white'}"
          >
            Artwork
          </button>
          <button
            type="button"
            onclick={() => (activeView = 'lyrics')}
            class="px-3 py-1 rounded-full text-[11px] font-medium transition-colors cursor-pointer {activeView === 'lyrics'
              ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] shadow-sm font-semibold'
              : 'text-white/60 hover:text-white'}"
          >
            Lyrics
          </button>
        </div>

        <div data-tauri-drag-region class="flex-1 h-8 cursor-default"></div>
      </div>

      <!-- Right Actions + Window Controls -->
      <div class="flex items-center gap-1.5 shrink-0">
        {#if settingsState.showAudioQualityBadge && playerState.currentTrack}
          <span class="bg-white/10 border border-white/10 text-[var(--md-sys-primary)] text-[10px] tracking-wider uppercase px-2.5 py-0.5 rounded-full font-semibold hidden sm:inline-block">
            {audioQualityLabel}
          </span>
        {/if}

        <button
          onclick={() => (playerState.isQueueOpen = !playerState.isQueueOpen)}
          class="w-8 h-8 rounded-full bg-white/5 hover:bg-white/15 flex items-center justify-center text-white/70 hover:text-white transition-colors cursor-pointer"
          title="Playing Next"
        >
          <span class="material-symbols-rounded text-[18px]">queue_music</span>
        </button>

        <button
          onclick={() => (playerState.isEqualizerOpen = true)}
          class="w-8 h-8 rounded-full bg-white/5 hover:bg-white/15 flex items-center justify-center text-white/70 hover:text-white transition-colors cursor-pointer"
          title="Equalizer"
        >
          <span class="material-symbols-rounded text-[18px]">tune</span>
        </button>

        <div class="h-3.5 w-[1px] bg-white/10 mx-1"></div>

        <div class="flex items-center gap-0.5">
          <button
            onclick={handleMinimize}
            aria-label="Minimize Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-white/60 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          >
            <span class="material-symbols-rounded text-[15px]">remove</span>
          </button>
          <button
            onclick={handleMaximize}
            aria-label="Maximize Window"
            class="w-7 h-7 rounded-md flex items-center justify-center text-white/60 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
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
    <div class="flex-1 flex overflow-hidden z-10 min-h-0 py-2 gap-6 lg:gap-12 items-center">
      <!-- Left Column: Editorial Floating Artwork + Dynamic Theme-Tinted Transport -->
      {#if activeView === 'auto' || activeView === 'turntable'}
        <div
          class="flex flex-col items-center justify-center gap-4 h-full min-h-0 {activeView === 'auto'
            ? 'w-full md:w-[44%] lg:w-[42%] shrink-0'
            : 'w-full max-w-md mx-auto'} py-2 px-2 sm:px-4"
        >
          <!-- Editorial Floating Squircle Artwork Showcase -->
          <div class="flex items-center justify-center w-full min-h-0">
            <div
              class="editorial-artwork-frame relative rounded-[28px] overflow-hidden ring-1 ring-white/15 transition-transform duration-500 [transition-timing-function:cubic-bezier(0.22,1,0.36,1)] {playerState.playback.is_playing
                ? 'scale-100 translate-y-0 artwork-shadow-playing'
                : 'scale-[0.88] translate-y-1 opacity-90 artwork-shadow-paused'}"
            >
              {#if hiResArtwork}
                <img
                  src={hiResArtwork}
                  alt={playerState.currentTrack?.title}
                  onload={(e) => handleThumbnailLoad(e, playerState.currentTrack?.id)}
                  onerror={(e) => handleThumbnailError(e, playerState.currentTrack?.id)}
                  class="w-full h-full object-cover select-none"
                  draggable="false"
                />
              {:else}
                <div class="w-full h-full bg-gradient-to-br from-white/10 to-white/5 flex items-center justify-center text-white/30">
                  <span class="material-symbols-rounded text-6xl">music_note</span>
                </div>
              {/if}

              <!-- Subtle top glass specular highlight -->
              <div class="absolute inset-0 bg-gradient-to-tr from-transparent via-transparent to-white/10 pointer-events-none"></div>
            </div>
          </div>

          <!-- Track Metadata, Visualizer & Dynamic Theme Scrubber -->
          <div class="w-full max-w-[340px] flex flex-col shrink-0 space-y-2.5 pt-1">
            <!-- Title, Artist & Favorite Button -->
            <div class="flex items-center justify-between gap-3 w-full">
              <div class="min-w-0 flex-1">
                <h2 class="font-display font-bold text-lg sm:text-xl text-white truncate">
                  {playerState.currentTrack?.title || 'Not Playing'}
                </h2>
                <ArtistLinks
                  artist={playerState.currentTrack ? playerState.currentTrack.artist : ''}
                  class="mt-0.5 text-xs sm:text-sm text-[var(--md-sys-primary)]/90"
                  buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                  onArtistClick={(art) => {
                    playerState.isNowPlayingOpen = false;
                    playerState.openArtist(art);
                  }}
                />
              </div>

              {#if playerState.currentTrack}
                <div class="flex items-center gap-1.5 shrink-0">
                  <button
                    type="button"
                    onclick={() =>
                      playerState.currentTrack &&
                      playerState.toggleFavorite(playerState.currentTrack.id)}
                    class="w-9 h-9 rounded-full bg-white/5 hover:bg-white/15 flex items-center justify-center transition-colors cursor-pointer {playerState.currentTrack.favorite
                      ? 'text-[var(--md-sys-primary)]'
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
                    class="w-9 h-9 rounded-full bg-white/5 hover:bg-white/15 flex items-center justify-center text-white/60 hover:text-white transition-colors cursor-pointer"
                    title="More Options"
                  >
                    <span class="material-symbols-rounded text-[19px]">more_horiz</span>
                  </button>
                </div>
              {/if}
            </div>

            {#if settingsState.spectrumVisualizer}
              <div
                class="w-full h-5 flex items-center justify-between gap-[3px] px-1 pointer-events-none overflow-hidden"
                aria-hidden="true"
              >
                {#each visualizerBars as bar (bar.id)}
                  <div
                    class="spectrum-bar {bar.animClass} rounded-full"
                    class:playing={playerState.playback.is_playing}
                    style="
                      height: {bar.height};
                      animation-duration: {bar.duration};
                      animation-delay: {bar.delay};
                      opacity: {bar.opacity};
                    "
                  ></div>
                {/each}
              </div>
            {/if}

            <!-- Fluid Scrubber with Dynamic Song Theme Accent Fill -->
            <div class="w-full space-y-1.5">
              <div class="relative w-full h-1.5 bg-white/15 hover:h-2.5 rounded-full overflow-hidden cursor-pointer transition-[height]">
                <div
                  class="absolute inset-0 bg-[var(--md-sys-primary)] rounded-full origin-left will-change-transform"
                  style="transform: scaleX({playerState.progressPercent / 100}); box-shadow: 0 0 12px var(--artwork-glow);"
                ></div>
                <input
                  type="range"
                  min="0"
                  max={playerState.playback.duration || (playerState.currentTrack?.duration ?? 100) || 100}
                  value={playerState.playback.position}
                  oninput={(e) => playerState.seek(parseFloat(e.currentTarget.value))}
                  class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                />
              </div>
              <div class="flex justify-between text-[11px] font-mono text-white/55">
                <span class="text-[var(--md-sys-primary)] font-semibold">{playerState.formattedTime.current}</span>
                <span>{isBuffering ? 'Loading...' : playerState.formattedTime.total}</span>
              </div>
            </div>

            <!-- Transport Controls with Dynamic Song Theme Accent Button -->
            <div class="flex items-center justify-between px-2 pt-0.5">
              <button
                onclick={() => playerState.toggleShuffle()}
                class="p-2 rounded-full transition-colors cursor-pointer {playerState.shuffleMode
                  ? 'text-[var(--md-sys-primary)] bg-[var(--md-sys-primary-container)]/50'
                  : 'text-white/45 hover:text-white'}"
                title="Shuffle"
              >
                <span class="material-symbols-rounded text-[20px]">shuffle</span>
              </button>

              <div class="flex items-center gap-6">
                <button
                  onclick={() => playerState.playPrevious()}
                  class="text-white/85 hover:text-[var(--md-sys-primary)] active:scale-90 transition-all cursor-pointer"
                  title="Previous"
                >
                  <span class="material-symbols-rounded text-[30px] filled">skip_previous</span>
                </button>

                <div class="relative flex items-center justify-center">
                  <button
                    onclick={() => playerState.togglePlay()}
                    class="sheet-squircle-btn w-14 h-14 {playerState.playback.is_playing ? 'is-playing' : ''} bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center active:scale-90 hover:scale-105 cursor-pointer shadow-xl"
                    style="box-shadow: 0 10px 28px -6px color-mix(in srgb, var(--artwork-glow, #d0bcff) 55%, transparent);"
                    title={isBuffering
                      ? 'Buffering...'
                      : playerState.playback.is_playing
                        ? 'Pause'
                        : 'Play'}
                  >
                    {#if isBuffering}
                      <svg class="w-7 h-7 archivetune-spinner-svg text-current" viewBox="0 0 36 36" fill="none">
                        <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                        <circle
                          cx="18"
                          cy="18"
                          r="14"
                          stroke="currentColor"
                          stroke-opacity="0.38"
                          stroke-width="3.2"
                          stroke-dasharray="18 70"
                          stroke-dashoffset="-12"
                          stroke-linecap="round"
                        />
                        <circle
                          class="archivetune-spinner-arc"
                          cx="18"
                          cy="18"
                          r="14"
                          stroke="currentColor"
                          stroke-width="3.4"
                          stroke-linecap="round"
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
                  class="text-white/85 hover:text-[var(--md-sys-primary)] active:scale-90 transition-all cursor-pointer"
                  title="Next"
                >
                  <span class="material-symbols-rounded text-[30px] filled">skip_next</span>
                </button>
              </div>

              <button
                onclick={() => playerState.toggleRepeat()}
                class="p-2 rounded-full transition-colors cursor-pointer {playerState.repeatMode !== 'off'
                  ? 'text-[var(--md-sys-primary)] bg-[var(--md-sys-primary-container)]/50'
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

      <!-- Right Column: Seamless Borderless Apple Music / ArchiveTune Synced Lyrics (100% Automatic Best Synced) -->
      {#if activeView === 'auto' || activeView === 'lyrics'}
        <div
          class="flex-1 flex flex-col h-full overflow-hidden min-h-0 relative {activeView === 'auto'
            ? 'w-full md:flex-1'
            : 'w-full max-w-3xl mx-auto'} gpu-layer"
        >
          <div class="flex items-center justify-end pb-1 shrink-0 px-3 relative z-30 gap-1.5">
            {#if userIsScrolling}
              <button
                type="button"
                onclick={resumeSync}
                class="text-[11px] font-semibold px-3 py-1 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center gap-1 cursor-pointer shadow-sm"
              >
                <span class="material-symbols-rounded text-[13px]">my_location</span>
                <span>Sync</span>
              </button>
            {/if}

            {#if playerState.lyrics.has_romanized}
              <button
                type="button"
                onclick={() => (isRomanizedMode = !isRomanizedMode)}
                class="px-2.5 py-1 rounded-full text-[11px] font-semibold flex items-center gap-1 border transition-colors cursor-pointer {isRomanizedMode
                  ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] border-transparent'
                  : 'bg-white/5 text-white/55 hover:text-white border-white/10'}"
                title="Toggle Romanized Phonetic Lyrics"
              >
                <span class="font-sans text-[11px] font-bold">Aあ</span>
                <span>Romanized</span>
              </button>
            {/if}

            <button
              type="button"
              onclick={() => (isOffsetOpen = !isOffsetOpen)}
              class="px-2.5 py-1 rounded-full text-[11px] font-mono flex items-center gap-1 border transition-colors cursor-pointer {Math.abs(settingsState.lyricOffsetSeconds) > 0.05
                ? 'bg-amber-500/20 text-amber-300 border-amber-500/30'
                : 'bg-white/5 text-white/50 hover:text-white border-white/10'}"
              title="Timing Offset"
            >
              <span>{settingsState.lyricOffsetSeconds > 0 ? '+' : ''}{settingsState.lyricOffsetSeconds.toFixed(1)}s</span>
            </button>
          </div>

          {#if isOffsetOpen}
            <div class="flex items-center justify-between bg-black/45 border border-white/10 rounded-2xl px-3.5 py-2 mx-3 mb-2 text-xs text-white/80">
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
            class="flex-1 overflow-y-auto px-3 sm:px-6 pt-[18vh] pb-[46vh] space-y-3 scrollbar-none lyrics-viewport-mask"
          >
            {#if playerState.isLoadingLyrics}
              <div class="h-full flex flex-col items-center justify-center text-center text-white/50 py-16 gap-3.5">
                <svg class="w-8 h-8 archivetune-spinner-svg text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                  <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.16" stroke-width="3" />
                  <circle
                    cx="18"
                    cy="18"
                    r="14"
                    stroke="currentColor"
                    stroke-opacity="0.34"
                    stroke-width="3"
                    stroke-dasharray="18 70"
                    stroke-dashoffset="-12"
                    stroke-linecap="round"
                  />
                  <circle class="archivetune-spinner-arc" cx="18" cy="18" r="14" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" />
                </svg>
                <p class="text-xs font-medium tracking-wide text-white/55">
                  Loading synced lyrics...
                </p>
              </div>
            {:else if playerState.lyrics.synced && playerState.lyrics.lines.length > 0}
              <!-- Instrumental Intro Rhythm Dots (>= 4.0s before first line) -->
              {#if playerState.lyrics.lines[0].time >= 4.0}
                {@const isIntroActive = (smoothPosition + settingsState.lyricOffsetSeconds + 0.08) < playerState.lyrics.lines[0].time}
                <div class="py-4 px-2 flex items-center gap-2 transition-opacity duration-300 {isIntroActive ? 'opacity-95' : 'opacity-20'}">
                  <span class="w-2.5 h-2.5 rounded-full bg-[var(--md-sys-primary)] interlude-dot-1 inline-block"></span>
                  <span class="w-2.5 h-2.5 rounded-full bg-[var(--md-sys-primary)] interlude-dot-2 inline-block"></span>
                  <span class="w-2.5 h-2.5 rounded-full bg-[var(--md-sys-primary)] interlude-dot-3 inline-block"></span>
                </div>
              {/if}

              {#each playerState.lyrics.lines as line, i}
                {@const dist = i - activeIndex}
                {@const isCur = dist === 0}

                <!-- Mid-song Instrumental Break Rhythm Dots (gap >= 10.0s between lines) -->
                {#if i > 0 && (line.time - playerState.lyrics.lines[i - 1].time) >= 10.0}
                  {@const curPos = smoothPosition + settingsState.lyricOffsetSeconds + 0.08}
                  {@const isBreakActive = curPos >= playerState.lyrics.lines[i - 1].time + 2.0 && curPos < line.time}
                  <div class="py-3 px-2 flex items-center gap-2 transition-opacity duration-300 {isBreakActive ? 'opacity-95' : 'opacity-20'}">
                    <span class="w-2 h-2 rounded-full bg-[var(--md-sys-primary)] interlude-dot-1 inline-block"></span>
                    <span class="w-2 h-2 rounded-full bg-[var(--md-sys-primary)] interlude-dot-2 inline-block"></span>
                    <span class="w-2 h-2 rounded-full bg-[var(--md-sys-primary)] interlude-dot-3 inline-block"></span>
                  </div>
                {/if}

                <button
                  type="button"
                  data-lyric-index={i}
                  onclick={() => {
                    playerState.seek(line.time - settingsState.lyricOffsetSeconds);
                    resumeSync();
                  }}
                  class="w-full text-left font-display font-bold tracking-tight py-2 px-2 block text-xl sm:text-2xl md:text-[28px] leading-snug cursor-pointer select-none gpu-lyric cascade-lyric {isCur
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
                  <span class="block">{line.text}</span>

                  {#if isRomanizedMode && line.romanized && line.romanized !== line.text}
                    <span
                      class="block text-sm sm:text-base font-sans font-medium tracking-normal mt-1 {isCur
                        ? 'text-[var(--md-sys-primary)]'
                        : 'text-white/45'}"
                    >
                      {line.romanized}
                    </span>
                  {/if}

                  {#if isRomanizedMode && line.translation}
                    <span
                      class="block text-xs sm:text-sm font-sans font-normal tracking-normal mt-0.5 {isCur
                        ? 'text-white/70'
                        : 'text-white/35'}"
                    >
                      {line.translation}
                    </span>
                  {/if}
                </button>
              {/each}
            {:else if playerState.lyrics.plain_text}
              <div class="text-white/80 whitespace-pre-line text-base sm:text-lg font-medium leading-relaxed px-2 py-2">
                {playerState.lyrics.plain_text}
              </div>
            {:else}
              <div class="h-full flex flex-col items-center justify-center text-center text-white/35 py-16 gap-3">
                <span class="material-symbols-rounded text-[44px] opacity-40">lyrics</span>
                <p class="text-sm font-medium text-white/55">No lyrics found for this track</p>
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
    width: min(310px, 38vh);
    height: min(310px, 38vh);
    aspect-ratio: 1 / 1;
    will-change: transform;
  }

  .artwork-shadow-playing {
    box-shadow:
      0 28px 68px -12px color-mix(in srgb, var(--artwork-glow, #d0bcff) 48%, transparent),
      0 18px 40px -10px rgba(0, 0, 0, 0.85);
  }

  .artwork-shadow-paused {
    box-shadow: 0 16px 36px -12px rgba(0, 0, 0, 0.75);
  }

  .lyrics-viewport-mask {
    mask-image: linear-gradient(to bottom, transparent 0%, black 12%, black 86%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 12%, black 86%, transparent 100%);
  }

  .gpu-layer {
    will-change: transform, opacity;
    transform: translate3d(0, 0, 0);
  }
  .gpu-lyric {
    will-change: transform, opacity;
    transform: translate3d(0, 0, 0);
    background: transparent;
  }

  .cascade-lyric {
    transform-origin: left center;
    transition:
      transform 0.34s cubic-bezier(0.22, 1, 0.36, 1),
      opacity 0.34s cubic-bezier(0.22, 1, 0.36, 1),
      color 0.25s ease;
  }

  /* Seamless Borderless Apple Music / ArchiveTune Active Lyric (Zero Box Corners!) */
  .active-lyric {
    opacity: 1;
    transform: translate3d(0, 0, 0) scale(1.03);
    color: #ffffff;
    text-shadow: 0 0 28px color-mix(in srgb, var(--lyric-glow, #d0bcff) 60%, transparent);
    background: transparent;
  }

  .cascade-dist-1 {
    opacity: 0.48;
    transform: translate3d(0, 0, 0) scale(0.98);
    color: rgba(255, 255, 255, 0.9);
  }

  .cascade-dist-2 {
    opacity: 0.30;
    transform: translate3d(0, 0, 0) scale(0.965);
    color: rgba(255, 255, 255, 0.82);
  }

  .cascade-dist-far {
    opacity: 0.18;
    transform: translate3d(0, 0, 0) scale(0.95);
    color: rgba(255, 255, 255, 0.75);
  }

  .cascade-dist-past-1 {
    opacity: 0.22;
    transform: translate3d(0, 0, 0) scale(0.965);
    color: rgba(255, 255, 255, 0.75);
  }

  .cascade-dist-past-far {
    opacity: 0.12;
    transform: translate3d(0, 0, 0) scale(0.95);
    color: rgba(255, 255, 255, 0.65);
  }

  .cascade-lyric:hover {
    opacity: 0.92;
    transform: translate3d(0, 0, 0) scale(1);
    color: #ffffff;
  }

  .sheet-squircle-btn {
    border-radius: 50%;
    transition: border-radius 240ms cubic-bezier(0.2, 0, 0, 1), transform 140ms cubic-bezier(0.2, 0, 0, 1);
    will-change: border-radius, transform;
  }
  .sheet-squircle-btn.is-playing {
    border-radius: 28%;
  }

  /* 100% Hardware GPU Composited Visualizer Bars (Zero 2D Canvas CPU Rasterization) */
  .spectrum-bar {
    flex: 1;
    max-width: 6px;
    min-width: 2px;
    background-color: var(--artwork-glow, var(--md-sys-primary));
    transform-origin: center;
    will-change: transform;
    transform: scaleY(0.2) translate3d(0, 0, 0);
    transition: transform 0.35s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .spectrum-bar.playing.anim-spectrum-1 {
    animation: spectrumBounce1 infinite ease-in-out alternate;
  }
  .spectrum-bar.playing.anim-spectrum-2 {
    animation: spectrumBounce2 infinite ease-in-out alternate;
  }
  .spectrum-bar.playing.anim-spectrum-3 {
    animation: spectrumBounce3 infinite ease-in-out alternate;
  }
  .spectrum-bar.playing.anim-spectrum-4 {
    animation: spectrumBounce4 infinite ease-in-out alternate;
  }
  @keyframes spectrumBounce1 {
    0% { transform: scaleY(0.18) translate3d(0, 0, 0); }
    100% { transform: scaleY(1) translate3d(0, 0, 0); }
  }
  @keyframes spectrumBounce2 {
    0% { transform: scaleY(0.26) translate3d(0, 0, 0); }
    50% { transform: scaleY(0.58) translate3d(0, 0, 0); }
    100% { transform: scaleY(0.92) translate3d(0, 0, 0); }
  }
  @keyframes spectrumBounce3 {
    0% { transform: scaleY(0.15) translate3d(0, 0, 0); }
    40% { transform: scaleY(0.85) translate3d(0, 0, 0); }
    100% { transform: scaleY(0.38) translate3d(0, 0, 0); }
  }
  @keyframes spectrumBounce4 {
    0% { transform: scaleY(0.22) translate3d(0, 0, 0); }
    60% { transform: scaleY(0.96) translate3d(0, 0, 0); }
    100% { transform: scaleY(0.28) translate3d(0, 0, 0); }
  }
</style>
