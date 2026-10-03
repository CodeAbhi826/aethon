<script lang="ts">
  import { onMount } from 'svelte';
  import { playerState } from '../state.svelte';

  const frequencies = ['31Hz', '62Hz', '125Hz', '250Hz', '500Hz', '1kHz', '2kHz', '4kHz', '8kHz', '16kHz'];

  const presets: { name: string; bands: number[] }[] = [
    { name: 'Flat', bands: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
    { name: 'Bass Boost', bands: [6, 5, 4, 2, 0, 0, 0, 0, 0, 0] },
    { name: 'Vocal', bands: [-2, -1, 0, 2, 4, 4, 3, 1, 0, -1] },
    { name: 'Electronic', bands: [5, 4, 2, 0, -2, 2, 1, 3, 4, 5] },
    { name: 'Rock', bands: [4, 3, 1, 0, -1, 0, 2, 3, 4, 4] },
    { name: 'Acoustic', bands: [3, 2, 1, 1, 2, 2, 3, 3, 2, 1] },
  ];

  let canvasEl = $state<HTMLCanvasElement>();

  function setBand(idx: number, gain: number) {
    const newBands = [...playerState.equalizerBands];
    newBands[idx] = gain;
    playerState.setEqualizer(newBands);
    selectedPreset = 'Custom';
    drawCurve();
  }

  function applyPreset(p: { name: string; bands: number[] }) {
    selectedPreset = p.name;
    playerState.setEqualizer([...p.bands]);
    drawCurve();
  }

  function drawCurve() {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext('2d');
    if (!ctx) return;

    const w = canvasEl.width;
    const h = canvasEl.height;
    ctx.clearRect(0, 0, w, h);

    // Center 0dB line
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();
    ctx.setLineDash([]);

    // Curve line
    const bands = playerState.equalizerBands;
    const step = w / (bands.length - 1);

    ctx.strokeStyle = playerState.accentColor || '#79dc9c';
    ctx.lineWidth = 3;
    ctx.beginPath();

    const points: { x: number; y: number }[] = bands.map((g, i) => {
      const x = i * step;
      // -12dB to +12dB mapped to h to 0
      const norm = (g + 12) / 24;
      const y = h - norm * (h - 16) - 8;
      return { x, y };
    });

    // Spline curve
    ctx.moveTo(points[0].x, points[0].y);
    for (let i = 0; i < points.length - 1; i++) {
      const xc = (points[i].x + points[i + 1].x) / 2;
      const yc = (points[i].y + points[i + 1].y) / 2;
      ctx.quadraticCurveTo(points[i].x, points[i].y, xc, yc);
    }
    ctx.lineTo(points[points.length - 1].x, points[points.length - 1].y);
    ctx.stroke();

    // Area fill gradient
    ctx.lineTo(w, h);
    ctx.lineTo(0, h);
    ctx.closePath();
    const grad = ctx.createLinearGradient(0, 0, 0, h);
    grad.addColorStop(0, 'rgba(121, 220, 156, 0.15)');
    grad.addColorStop(1, 'rgba(121, 220, 156, 0.0)');
    ctx.fillStyle = grad;
    ctx.fill();
  }

  onMount(() => {
    drawCurve();
  });
</script>

{#if playerState.isEqualizerOpen}
  <div class="fixed inset-0 z-50 bg-black/60 backdrop-blur-md flex items-center justify-center p-4">
    <div class="w-full max-w-2xl bg-[#1a1c22] border border-white/[0.08] rounded-3xl p-6 shadow-2xl animate-scale-up select-none">
      
      <!-- Modal Header -->
      <div class="flex items-center justify-between pb-4 border-b border-white/[0.06]">
        <div class="flex items-center gap-2.5">
          <span class="material-symbols-rounded text-emerald-400 text-[24px]">tune</span>
          <h3 class="font-heading font-bold text-lg text-white">10-Band Equalizer</h3>
          <span class="text-xs px-2 py-0.5 rounded-full bg-white/[0.06] text-[#9ea2b0] font-mono">MPV Filter</span>
        </div>
        <button
          onclick={() => playerState.isEqualizerOpen = false}
          class="w-8 h-8 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white transition-colors"
        >
          <span class="material-symbols-rounded text-[18px]">close</span>
        </button>
      </div>

      <!-- Presets Capsules -->
      <div class="flex items-center gap-2 py-4 overflow-x-auto scrollbar-none">
        {#each presets as p}
          <button
            onclick={() => applyPreset(p)}
            class="px-3.5 py-1.5 rounded-full text-xs font-medium transition-all {selectedPreset === p.name ? 'bg-white text-[#111215] font-semibold shadow-sm' : 'bg-white/[0.05] text-[#9ea2b0] hover:text-white hover:bg-white/[0.08]'}"
          >
            {p.name}
          </button>
        {/each}
      </div>

      <!-- Response Curve Canvas -->
      <div class="w-full bg-[#121316] rounded-2xl p-3 border border-white/[0.04] mb-6">
        <canvas bind:this={canvasEl} width="560" height="90" class="w-full h-20 rounded-lg"></canvas>
      </div>

      <!-- 10 Vertical Sliders -->
      <div class="grid grid-cols-10 gap-2 items-center text-center">
        {#each frequencies as freq, idx}
          {@const gain = playerState.equalizerBands[idx] || 0}
          <div class="flex flex-col items-center gap-2">
            <span class="text-[10px] font-mono text-[#9ea2b0]">{gain > 0 ? `+${gain}` : gain}dB</span>
            <div class="h-40 flex items-center justify-center">
              <input
                type="range"
                min="-12"
                max="12"
                step="1"
                value={gain}
                oninput={(e) => setBand(idx, Number((e.target as HTMLInputElement).value))}
                class="slider-vertical h-36 w-1.5 appearance-none bg-white/[0.1] rounded-lg accent-emerald-400 cursor-pointer"
                style="writing-mode: vertical-lr; direction: rtl;"
              />
            </div>
            <span class="text-[10px] font-medium text-[#6b6e7c] truncate w-full">{freq}</span>
          </div>
        {/each}
      </div>

      <!-- Reset button -->
      <div class="flex justify-end pt-4 mt-4 border-t border-white/[0.06]">
        <button
          onclick={() => applyPreset(presets[0])}
          class="px-4 py-2 rounded-xl text-xs font-medium text-[#9ea2b0] hover:text-white hover:bg-white/[0.05] transition-colors"
        >
          Reset to Flat
        </button>
      </div>

    </div>
  </div>
{/if}

<style>
  .animate-scale-up {
    animation: scaleUp 0.2s cubic-bezier(0.2, 0, 0, 1);
  }
  @keyframes scaleUp {
    from { opacity: 0; transform: scale(0.95); }
    to { opacity: 1; transform: scale(1); }
  }
</style>
