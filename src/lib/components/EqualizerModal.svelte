<script lang="ts">
  import { playerState } from '../state.svelte';

  const frequencies = ['31Hz', '62Hz', '125Hz', '250Hz', '500Hz', '1kHz', '2kHz', '4kHz', '8kHz', '16kHz'];

  const presets: { name: string; bands: number[] }[] = [
    { name: 'Flat', bands: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
    { name: 'Bass Boost', bands: [6, 5, 4, 2, 0, 0, 0, 0, 0, 0] },
    { name: 'Vocal', bands: [-2, -1, 0, 2, 4, 4, 3, 1, 0, -1] },
    { name: 'Electronic', bands: [5, 4, 2, 0, -2, 2, 1, 3, 4, 5] },
    { name: 'Rock', bands: [4, 3, 1, 0, -1, 0, 2, 3, 4, 4] },
    { name: 'Acoustic', bands: [3, 2, 1, 1, 2, 2, 3, 3, 2, 1] },
    { name: 'Hip-Hop', bands: [5, 4, 1, 2, -1, -1, 1, 2, 3, 4] },
    { name: 'Treble Boost', bands: [-2, -2, -1, 0, 0, 1, 3, 5, 6, 7] },
  ];

  let canvasEl = $state<HTMLCanvasElement>();
  let selectedPreset = $state('Flat');
  let draggingBand = $state<number | null>(null);

  // Check if current bands match any preset
  function checkMatchingPreset(bands: number[]) {
    for (const p of presets) {
      if (p.bands.every((val, i) => Math.abs(val - (bands[i] || 0)) < 0.1)) {
        return p.name;
      }
    }
    return 'Custom';
  }

  function setBand(idx: number, gain: number) {
    const clamped = Math.round(Math.max(-12, Math.min(12, gain)) * 10) / 10;
    const newBands = [...playerState.equalizerBands];
    newBands[idx] = clamped;
    playerState.setEqualizer(newBands);
    selectedPreset = checkMatchingPreset(newBands);
  }

  function applyPreset(p: { name: string; bands: number[] }) {
    selectedPreset = p.name;
    playerState.setEqualizer([...p.bands]);
  }

  function handleSliderPointerDown(e: PointerEvent, idx: number) {
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    draggingBand = idx;
    updateGainFromPointer(e, target, idx);
  }

  function handleSliderPointerMove(e: PointerEvent, idx: number) {
    if (draggingBand !== idx) return;
    const target = e.currentTarget as HTMLElement;
    updateGainFromPointer(e, target, idx);
  }

  function handleSliderPointerUp(e: PointerEvent, idx: number) {
    if (draggingBand === idx) {
      const target = e.currentTarget as HTMLElement;
      try {
        target.releasePointerCapture(e.pointerId);
      } catch {}
      draggingBand = null;
    }
  }

  function updateGainFromPointer(e: PointerEvent, trackEl: HTMLElement, idx: number) {
    const rect = trackEl.getBoundingClientRect();
    const trackHeight = rect.height;
    // Y is relative to top of track
    const relY = Math.max(0, Math.min(trackHeight, e.clientY - rect.top));
    // 0 at top = +12dB, height at bottom = -12dB
    const norm = 1 - relY / trackHeight; // 1 at top, 0 at bottom
    const gain = Math.round((norm * 24 - 12) * 2) / 2; // snap to 0.5dB
    setBand(idx, gain);
  }

  function handleKeyDown(e: KeyboardEvent, idx: number) {
    const current = playerState.equalizerBands[idx] || 0;
    if (e.key === 'ArrowUp' || e.key === 'ArrowRight') {
      e.preventDefault();
      setBand(idx, current + 0.5);
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowLeft') {
      e.preventDefault();
      setBand(idx, current - 0.5);
    } else if (e.key === 'Home') {
      e.preventDefault();
      setBand(idx, 12);
    } else if (e.key === 'End') {
      e.preventDefault();
      setBand(idx, -12);
    }
  }

  function drawCurve() {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext('2d');
    if (!ctx) return;

    const rect = canvasEl.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    const w = rect.width || 560;
    const h = rect.height || 100;

    canvasEl.width = w * dpr;
    canvasEl.height = h * dpr;
    ctx.scale(dpr, dpr);

    ctx.clearRect(0, 0, w, h);

    // Grid lines for +12, +6, 0, -6, -12 dB
    const gridLevels = [
      { db: 12, y: 10, label: '+12' },
      { db: 6, y: h * 0.25 + 5, label: '+6' },
      { db: 0, y: h * 0.5, label: '0' },
      { db: -6, y: h * 0.75 - 5, label: '-6' },
      { db: -12, y: h - 10, label: '-12' },
    ];

    ctx.font = '9px monospace';
    ctx.textBaseline = 'middle';

    for (const grid of gridLevels) {
      ctx.strokeStyle = grid.db === 0 ? 'rgba(255, 255, 255, 0.15)' : 'rgba(255, 255, 255, 0.04)';
      ctx.lineWidth = 1;
      if (grid.db !== 0) {
        ctx.setLineDash([3, 4]);
      } else {
        ctx.setLineDash([]);
      }
      ctx.beginPath();
      ctx.moveTo(30, grid.y);
      ctx.lineTo(w - 10, grid.y);
      ctx.stroke();

      ctx.fillStyle = grid.db === 0 ? 'rgba(255, 255, 255, 0.4)' : 'rgba(255, 255, 255, 0.2)';
      ctx.fillText(grid.label, 8, grid.y);
    }
    ctx.setLineDash([]);

    // Curve calculation
    const bands = playerState.equalizerBands;
    const startX = 35;
    const endX = w - 15;
    const step = (endX - startX) / (bands.length - 1);

    const points: { x: number; y: number; gain: number }[] = bands.map((g, i) => {
      const x = startX + i * step;
      // -12 to +12 maps to (h - 10) to 10
      const norm = (g + 12) / 24;
      const y = (h - 10) - norm * (h - 20);
      return { x, y, gain: g };
    });

    const accent = playerState.accentColor || '#38bdf8';

    // Spline curve
    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    for (let i = 0; i < points.length - 1; i++) {
      const xc = (points[i].x + points[i + 1].x) / 2;
      const yc = (points[i].y + points[i + 1].y) / 2;
      ctx.quadraticCurveTo(points[i].x, points[i].y, xc, yc);
    }
    ctx.lineTo(points[points.length - 1].x, points[points.length - 1].y);

    // Save path for stroke
    ctx.strokeStyle = accent;
    ctx.lineWidth = 2.5;
    ctx.shadowColor = accent;
    ctx.shadowBlur = 10;
    ctx.stroke();
    ctx.shadowBlur = 0;

    // Gradient fill under curve to 0dB line
    ctx.lineTo(points[points.length - 1].x, h * 0.5);
    ctx.lineTo(points[0].x, h * 0.5);
    ctx.closePath();

    const grad = ctx.createLinearGradient(0, 0, 0, h);
    grad.addColorStop(0, `${accent}33`);
    grad.addColorStop(0.5, `${accent}11`);
    grad.addColorStop(1, `${accent}00`);
    ctx.fillStyle = grad;
    ctx.fill();

    // Draw dots at each band
    for (const pt of points) {
      ctx.beginPath();
      ctx.arc(pt.x, pt.y, Math.abs(pt.gain) > 0.1 ? 3.5 : 2, 0, Math.PI * 2);
      ctx.fillStyle = Math.abs(pt.gain) > 0.1 ? '#ffffff' : accent;
      ctx.fill();
      if (Math.abs(pt.gain) > 0.1) {
        ctx.strokeStyle = accent;
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }
    }
  }

  // Reactive redraw on state change
  $effect(() => {
    if (playerState.isEqualizerOpen && canvasEl) {
      // Small timeout to allow modal container to lay out bounding box
      const raf = requestAnimationFrame(() => {
        drawCurve();
      });
      return () => cancelAnimationFrame(raf);
    }
  });

  $effect(() => {
    const _ = playerState.equalizerBands;
    if (canvasEl && playerState.isEqualizerOpen) {
      drawCurve();
    }
  });
</script>

{#if playerState.isEqualizerOpen}
  <div class="fixed inset-0 z-50 bg-black/60 backdrop-blur-md flex items-center justify-center p-4">
    <div class="w-full max-w-2xl bg-[#14151a] border border-white/[0.08] rounded-3xl p-6 shadow-2xl animate-scale-up select-none">
      
      <!-- Modal Header -->
      <div class="flex items-center justify-between pb-4 border-b border-white/[0.06]">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-xl bg-[var(--md-sys-primary)]/15 flex items-center justify-center text-[var(--md-sys-primary)]">
            <span class="material-symbols-rounded text-[20px]">tune</span>
          </div>
          <div>
            <h3 class="font-heading font-bold text-base text-white">10-Band Graphic Equalizer</h3>
            <p class="text-[11px] text-[#9ea2b0]">Zero-latency PipeWire / MPV DSP Audio Filters</p>
          </div>
        </div>
        <button
          onclick={() => playerState.isEqualizerOpen = false}
          class="w-8 h-8 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white transition-colors cursor-pointer"
        >
          <span class="material-symbols-rounded text-[18px]">close</span>
        </button>
      </div>

      <!-- Presets Capsules -->
      <div class="flex items-center gap-2 py-4 overflow-x-auto scrollbar-none">
        {#each presets as p}
          {@const isActive = selectedPreset === p.name}
          <button
            onclick={() => applyPreset(p)}
            class="px-3 py-1.5 rounded-full text-xs font-medium transition-all shrink-0 cursor-pointer {isActive ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] font-bold shadow-md shadow-[var(--md-sys-primary)]/20 scale-105' : 'bg-white/[0.05] text-[#9ea2b0] hover:text-white hover:bg-white/[0.08]'}"
          >
            {p.name}
          </button>
        {/each}
        {#if selectedPreset === 'Custom'}
          <span class="px-3 py-1.5 rounded-full text-xs font-semibold bg-white/10 text-white/90 border border-white/10 shrink-0">
            Custom
          </span>
        {/if}
      </div>

      <!-- Response Curve Canvas -->
      <div class="w-full bg-[#0d0e12] rounded-2xl p-2 border border-white/[0.06] mb-5">
        <canvas bind:this={canvasEl} class="w-full h-24 rounded-lg block"></canvas>
      </div>

      <!-- 10 Vertical Sliders -->
      <div class="grid grid-cols-10 gap-1.5 items-center text-center">
        {#each frequencies as freq, idx}
          {@const gain = playerState.equalizerBands[idx] || 0}
          {@const isNonZero = Math.abs(gain) > 0.05}
          {@const norm = (gain + 12) / 24} <!-- 0 at -12, 0.5 at 0, 1 at +12 -->
          <div class="flex flex-col items-center">
            <!-- dB Readout (clickable to reset band) -->
            <button
              onclick={() => setBand(idx, 0)}
              title="Click to reset to 0dB"
              class="text-[10px] font-mono px-1 py-0.5 rounded transition-colors cursor-pointer mb-2 {isNonZero ? 'text-[var(--md-sys-primary)] font-bold bg-[var(--md-sys-primary)]/10' : 'text-[#6b6e7c] hover:text-white'}"
            >
              {gain > 0 ? `+${gain.toFixed(1)}` : gain.toFixed(1)}
            </button>

            <!-- Custom Vertical Slider Track -->
            <div
              tabindex="0"
              role="slider"
              aria-label="{freq} Gain"
              aria-valuemin="-12"
              aria-valuemax="12"
              aria-valuenow={gain}
              onpointerdown={(e) => handleSliderPointerDown(e, idx)}
              onpointermove={(e) => handleSliderPointerMove(e, idx)}
              onpointerup={(e) => handleSliderPointerUp(e, idx)}
              onkeydown={(e) => handleKeyDown(e, idx)}
              class="relative w-7 h-36 flex items-center justify-center cursor-pointer outline-none group touch-none"
            >
              <!-- Center 0dB indicator tick -->
              <div class="absolute left-1 right-1 top-1/2 -translate-y-1/2 h-[1px] bg-white/20 pointer-events-none z-0"></div>

              <!-- Base track -->
              <div class="w-1.5 h-full rounded-full bg-white/[0.08] group-hover:bg-white/[0.14] transition-colors relative overflow-hidden">
                <!-- Active filled region from 0dB center -->
                {#if gain > 0}
                  <div
                    class="absolute left-0 right-0 bg-[var(--md-sys-primary)] rounded-full transition-all"
                    style="top: {(1 - norm) * 100}%; bottom: 50%;"
                  ></div>
                {:else if gain < 0}
                  <div
                    class="absolute left-0 right-0 bg-[var(--md-sys-primary)]/70 rounded-full transition-all"
                    style="top: 50%; bottom: {norm * 100}%;"
                  ></div>
                {/if}
              </div>

              <!-- Custom Thumb Knob -->
              <div
                class="absolute w-4 h-4 rounded-full border-2 border-white shadow-lg pointer-events-none transition-transform duration-75 {isNonZero ? 'bg-[var(--md-sys-primary)] ring-2 ring-[var(--md-sys-primary)]/40' : 'bg-[#e2e4ea]'} group-hover:scale-125"
                style="top: {(1 - norm) * 100}%; transform: translateY(-50%);"
              ></div>
            </div>

            <!-- Frequency Label -->
            <span class="text-[10px] font-medium text-[#8c909e] truncate w-full mt-2 font-display">{freq}</span>
          </div>
        {/each}
      </div>

      <!-- Footer: Reset button -->
      <div class="flex items-center justify-between pt-4 mt-5 border-t border-white/[0.06]">
        <span class="text-[11px] text-[#6b6e7c]">Double click any dB label to reset to 0dB</span>
        <button
          onclick={() => applyPreset(presets[0])}
          class="px-4 py-1.5 rounded-xl text-xs font-semibold text-[#9ea2b0] hover:text-white hover:bg-white/[0.08] border border-white/[0.06] transition-all cursor-pointer"
        >
          Reset to Flat
        </button>
      </div>

    </div>
  </div>
{/if}

<style>
  .animate-scale-up {
    animation: scaleUp 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes scaleUp {
    from { opacity: 0; transform: scale(0.96); }
    to { opacity: 1; transform: scale(1); }
  }
</style>
