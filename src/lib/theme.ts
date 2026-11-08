import {
  argbFromHex,
  hexFromArgb,
  themeFromSourceColor,
} from '@material/material-color-utilities';
import { convertFileSrc } from '@tauri-apps/api/core';

export interface ThemeColors {
  primary: string;
  onPrimary: string;
  primaryContainer: string;
  onPrimaryContainer: string;
  secondary: string;
  onSecondary: string;
  secondaryContainer: string;
  onSecondaryContainer: string;
  tertiary: string;
  surface: string;
  surfaceDim: string;
  surfaceContainerLow: string;
  surfaceContainer: string;
  surfaceContainerHigh: string;
  onSurface: string;
  onSurfaceVariant: string;
  outline: string;
  outlineVariant: string;
  glow: string;
  mesh1: string;
  mesh2: string;
  mesh3: string;
  acrylic: string;
  acrylicCard: string;
  acrylicDock: string;
}

export interface ThemeOptions {
  pureOled?: boolean;
  dynamicBg?: boolean;
  accentIntensity?: 'subtle' | 'balanced' | 'expressive';
}

export const THEME_PRESETS: Record<string, { name: string; hex: string; desc: string }> = {
  dynamic: { name: 'Dynamic Monet', hex: '#d0bcff', desc: 'Adapts in real-time to current album art' },
  violet: { name: 'Expressive Violet', hex: '#d0bcff', desc: 'Signature expressive Material 3 purple' },
  emerald: { name: 'Bit-Perfect Emerald', hex: '#80e2a7', desc: 'Hi-Res audio green aesthetic' },
  sapphire: { name: 'Sapphire Ocean', hex: '#8ec5fc', desc: 'Vibrant cool blue tones' },
  crimson: { name: 'Crimson Velvet', hex: '#ff8ba7', desc: 'Warm passionate rose velvet' },
  amber: { name: 'Sunset Amber', hex: '#ffc078', desc: 'Cozy golden acoustic warmth' },
  oled: { name: 'Pure Obsidian OLED', hex: '#e2e2e6', desc: 'Absolute #000000 true black contrast' },
  cupertino: { name: 'Cupertino Graphite', hex: '#0a84ff', desc: 'Modern minimal system accent & deep graphite' },
  tokyonight: { name: 'Tokyo Night Neon', hex: '#7aa2f7', desc: 'Cyberpunk neon indigo aesthetic' },
};

function hexToRgba(hex: string, alpha: number): string {
  const clean = hex.replace('#', '');
  const num = parseInt(clean, 16);
  const r = (num >> 16) & 255;
  const g = (num >> 8) & 255;
  const b = num & 255;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/**
 * Generate full Material 3 Dark theme tokens from a seed ARGB / hex color
 */
export function generateThemeFromHex(seedHex: string, options?: ThemeOptions): ThemeColors {
  const seedArgb = argbFromHex(seedHex);
  const theme = themeFromSourceColor(seedArgb);
  const dark = theme.schemes.dark;

  const primary = hexFromArgb(dark.primary);
  const onPrimary = hexFromArgb(dark.onPrimary);
  const primaryContainer = hexFromArgb(dark.primaryContainer);
  const onPrimaryContainer = hexFromArgb(dark.onPrimaryContainer);
  const secondary = hexFromArgb(dark.secondary);
  const onSecondary = hexFromArgb(dark.onSecondary);
  const secondaryContainer = hexFromArgb(dark.secondaryContainer);
  const onSecondaryContainer = hexFromArgb(dark.onSecondaryContainer);
  const tertiary = hexFromArgb(dark.tertiary);

  let surface = hexFromArgb(dark.surface);
  let surfaceDim = hexFromArgb(theme.palettes.neutral.tone(6));
  let surfaceContainerLow = hexFromArgb(theme.palettes.neutral.tone(10));
  let surfaceContainer = hexFromArgb(theme.palettes.neutral.tone(14));
  let surfaceContainerHigh = hexFromArgb(theme.palettes.neutral.tone(18));

  if (options?.pureOled) {
    surface = '#000000';
    surfaceDim = '#000000';
    surfaceContainerLow = '#08080a';
    surfaceContainer = '#111114';
    surfaceContainerHigh = '#1a1a1f';
  } else if (options?.dynamicBg === false) {
    surface = '#100d14';
    surfaceDim = '#0b090e';
    surfaceContainerLow = '#19151f';
    surfaceContainer = '#201a27';
    surfaceContainerHigh = '#2b2434';
  }

  const onSurface = hexFromArgb(dark.onSurface);
  const onSurfaceVariant = hexFromArgb(dark.onSurfaceVariant);
  const outline = hexFromArgb(dark.outline);
  const outlineVariant = hexFromArgb(dark.outlineVariant);

  const intensity = options?.accentIntensity || 'expressive';
  const glowAlpha = intensity === 'subtle' ? 0.22 : intensity === 'balanced' ? 0.35 : 0.48;
  const m1Alpha = intensity === 'subtle' ? 0.18 : intensity === 'balanced' ? 0.28 : 0.38;
  const m2Alpha = intensity === 'subtle' ? 0.14 : intensity === 'balanced' ? 0.22 : 0.30;
  const m3Alpha = intensity === 'subtle' ? 0.10 : intensity === 'balanced' ? 0.16 : 0.24;

  const glow = hexToRgba(primary, glowAlpha);
  const mesh1 = hexToRgba(primary, m1Alpha);
  const mesh2 = hexToRgba(secondary, m2Alpha);
  const mesh3 = hexToRgba(tertiary, m3Alpha);

  const acrylic = hexToRgba(surfaceContainer, 0.95);
  const acrylicCard = hexToRgba(surfaceContainerHigh, 0.90);
  const acrylicDock = hexToRgba(surface, 0.97);

  return {
    primary,
    onPrimary,
    primaryContainer,
    onPrimaryContainer,
    secondary,
    onSecondary,
    secondaryContainer,
    onSecondaryContainer,
    tertiary,
    surface,
    surfaceDim,
    surfaceContainerLow,
    surfaceContainer,
    surfaceContainerHigh,
    onSurface,
    onSurfaceVariant,
    outline,
    outlineVariant,
    glow,
    mesh1,
    mesh2,
    mesh3,
    acrylic,
    acrylicCard,
    acrylicDock,
  };
}

/**
 * Apply the generated palette directly to CSS root variables
 */
export function applyThemeTokens(colors: ThemeColors) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;

  root.style.setProperty('--md-sys-primary', colors.primary);
  root.style.setProperty('--md-sys-on-primary', colors.onPrimary);
  root.style.setProperty('--md-sys-primary-container', colors.primaryContainer);
  root.style.setProperty('--md-sys-on-primary-container', colors.onPrimaryContainer);
  root.style.setProperty('--md-sys-secondary', colors.secondary);
  root.style.setProperty('--md-sys-on-secondary', colors.onSecondary);
  root.style.setProperty('--md-sys-secondary-container', colors.secondaryContainer);
  root.style.setProperty('--md-sys-on-secondary-container', colors.onSecondaryContainer);
  root.style.setProperty('--md-sys-tertiary', colors.tertiary);
  root.style.setProperty('--md-sys-surface', colors.surface);
  root.style.setProperty('--md-sys-surface-dim', colors.surfaceDim);
  root.style.setProperty('--md-sys-surface-container-low', colors.surfaceContainerLow);
  root.style.setProperty('--md-sys-surface-container', colors.surfaceContainer);
  root.style.setProperty('--md-sys-surface-container-high', colors.surfaceContainerHigh);
  root.style.setProperty('--md-sys-on-surface', colors.onSurface);
  root.style.setProperty('--md-sys-on-surface-variant', colors.onSurfaceVariant);
  root.style.setProperty('--md-sys-outline', colors.outline);
  root.style.setProperty('--md-sys-outline-variant', colors.outlineVariant);
  root.style.setProperty('--md-sys-glow', colors.glow);
  root.style.setProperty('--md-sys-mesh-1', colors.mesh1);
  root.style.setProperty('--md-sys-mesh-2', colors.mesh2);
  root.style.setProperty('--md-sys-mesh-3', colors.mesh3);
  root.style.setProperty('--md-sys-acrylic', colors.acrylic);
  root.style.setProperty('--md-sys-acrylic-card', colors.acrylicCard);
  root.style.setProperty('--md-sys-acrylic-dock', colors.acrylicDock);

  root.style.setProperty('--at-accent', colors.primary);
  root.style.setProperty('--at-accent-container', colors.primaryContainer);
  root.style.setProperty('--at-on-accent-container', colors.onPrimaryContainer);
}

const extractedHexCache = new Map<string, string>();

const FALLBACK_VIBRANT_SEEDS = [
  '#ff7eb6',
  '#78a9ff',
  '#42be65',
  '#ffab40',
  '#be95ff',
  '#3ddbd9',
  '#ff8389',
  '#82cfff',
  '#f1c21b',
  '#d4bbff',
];

function deterministicSeedFromUrl(url: string): string {
  let h = 2166136261;
  for (let i = 0; i < url.length; i++) {
    h ^= url.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return FALLBACK_VIBRANT_SEEDS[Math.abs(h) % FALLBACK_VIBRANT_SEEDS.length];
}

/**
 * Fast Downsampled 24x24 Canvas (< 0.3 ms) + CORS-Safe Monet Artwork Extraction
 */
export async function extractAndApplyThemeFromImage(
  imageUrl: string,
  options?: ThemeOptions
): Promise<string> {
  if (!imageUrl) return '#d0bcff';

  const cached = extractedHexCache.get(imageUrl);
  if (cached) {
    const tokens = generateThemeFromHex(cached, options);
    applyThemeTokens(tokens);
    return cached;
  }

  return new Promise((resolve) => {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    const isRemote = imageUrl.startsWith('http://') || imageUrl.startsWith('https://');
    const isDataOrBlob = imageUrl.startsWith('data:') || imageUrl.startsWith('blob:');
    // Add a CORS-specific query key on remote URLs so WebKitGTK doesn't reuse a non-CORS cached response from <img> tags
    const corsSafeUrl = isRemote
      ? `${imageUrl}${imageUrl.includes('?') ? '&' : '?'}cors=1`
      : isDataOrBlob
        ? imageUrl
        : convertFileSrc(imageUrl);

    img.src = corsSafeUrl;

    const finishWithSeed = (seedHex: string) => {
      const tokens = generateThemeFromHex(seedHex, options);
      extractedHexCache.set(imageUrl, tokens.primary);
      applyThemeTokens(tokens);
      resolve(tokens.primary);
    };

    img.onload = () => {
      try {
        // Ignore YouTube's 120x90 gray placeholder when extracting color
        if (img.naturalWidth === 120 && img.naturalHeight === 90) {
          return finishWithSeed(deterministicSeedFromUrl(imageUrl));
        }
        const canvas = document.createElement('canvas');
        canvas.width = 24;
        canvas.height = 24;
        const ctx = canvas.getContext('2d', { willReadFrequently: true });
        if (!ctx) {
          return finishWithSeed(deterministicSeedFromUrl(imageUrl));
        }

        ctx.drawImage(img, 0, 0, 24, 24);
        const data = ctx.getImageData(0, 0, 24, 24).data;

        let bestHex = '';
        let maxScore = -1;
        let avgR = 0, avgG = 0, avgB = 0, totalValid = 0;

        for (let i = 0; i < data.length; i += 4) {
          const a = data[i + 3];
          if (a < 128) continue;

          const r = data[i];
          const g = data[i + 1];
          const b = data[i + 2];

          avgR += r;
          avgG += g;
          avgB += b;
          totalValid++;

          const max = Math.max(r, g, b);
          const min = Math.min(r, g, b);
          const chroma = max - min;
          const lightness = (max + min) / 2;

          if (lightness < 22 || lightness > 232 || chroma < 18) continue;

          const score = chroma * 1.6 + (1 - Math.abs(lightness - 130) / 130) * 45;
          if (score > maxScore) {
            maxScore = score;
            bestHex = `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
          }
        }

        if (!bestHex && totalValid > 0) {
          const r = Math.round(avgR / totalValid);
          const g = Math.round(avgG / totalValid);
          const b = Math.round(avgB / totalValid);
          bestHex = `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
        }

        finishWithSeed(bestHex || deterministicSeedFromUrl(imageUrl));
      } catch {
        finishWithSeed(deterministicSeedFromUrl(imageUrl));
      }
    };

    img.onerror = () => {
      finishWithSeed(deterministicSeedFromUrl(imageUrl));
    };
  });
}
