export interface Track {
  id: string;
  title: string;
  artist: string;
  album: string;
  duration: number;
  file_path: string;
  cover_art?: string;
  format: string;
  bitrate?: number;
  sample_rate?: number;
  play_count: number;
  favorite: boolean;
  date_added: number;
}

export interface PlaybackState {
  current_uri: string | null;
  media_title?: string | null;
  is_playing: boolean;
  is_buffering?: boolean;
  position: number;
  duration: number;
  volume: number;
  codec?: string;
  bitrate?: number;
  sample_rate?: number;
  channels?: number;
}

export interface LyricLine {
  time: number;
  text: string;
  romanized?: string | null;
  translation?: string | null;
}

export interface LyricsResult {
  synced: boolean;
  lines: LyricLine[];
  plain_text?: string;
  provider?: string;
  has_romanized?: boolean;
}

export interface YouTubeTrack {
  id: string;
  title: string;
  artist: string;
  duration: number;
  thumbnail: string;
  stream_url: string;
}

export interface HomeShelf {
  title: string;
  tracks: YouTubeTrack[];
}

export interface PlaybackTrackingSession {
  video_id: string;
  cpn: string;
  playback_url: string;
  watchtime_url: string;
  last_reported_pos: number;
}

export interface YtmSyncSummary {
  liked_songs_count: number;
  playlists_count: number;
  history_count: number;
}

export type TabType = 'home' | 'search' | 'library' | 'stats' | 'settings' | 'artist' | 'album';

export interface ArtistDetails {
  name: string;
  avatar?: string;
  monthlyListeners?: string;
  topTracks: Track[];
  albums: { title: string; year?: string; cover?: string }[];
}

export interface AlbumDetails {
  title: string;
  artist: string;
  cover?: string;
  year?: string;
  tracks: Track[];
}

export interface Playlist {
  id: string;
  name: string;
  description?: string | null;
  created_at: number;
  track_count: number;
}

export function sanitizeArtist(raw: string): string {
  if (!raw) return '';
  const trimmed = raw.trim();
  if (/^youtube(?:\s+music)?$/i.test(trimmed)) {
    return '';
  }
  return trimmed
    .replace(/^similar\s+to\s+/i, '')
    .replace(/\s*[•·|\-]\s*\d+(\.\d+)?[kmbKMB]?\s*(views?|subscribers?|plays?|watching|listeners?).*$/gi, '')
    .replace(/\s+\d+(\.\d+)?[kmbKMB]?\s*(views?|subscribers?|plays?|watching|listeners?).*$/gi, '')
    .replace(/\s*-\s*topic$/i, '')
    .replace(/\s*[•·]\s*.*$/gi, '')
    .replace(/\s*,\s*/g, ', ')
    .replace(/(?:,\s*){2,}/g, ', ')
    .replace(/\s*&\s*/g, ' & ')
    .replace(/\s{2,}/g, ' ')
    .replace(/^[\s,•·|\-\/]+|[\s,•·|\-\/]+$/g, '')
    .trim();
}

export function splitArtists(raw: string): string[] {
  if (!raw) return [];
  const cleaned = sanitizeArtist(raw);
  if (!cleaned) return [];

  const knownBands = new Set([
    'ac/dc',
    'simon & garfunkel',
    'hall & oates',
    'earth, wind & fire',
    'crosby, stills, nash & young',
    'kool & the gang',
    'broods',
    'chvrches',
  ]);
  if (knownBands.has(cleaned.toLowerCase())) {
    return [cleaned];
  }

  let text = cleaned.replace(/[\(\[\{]\s*(?:feat\.?|ft\.?|featuring|with)\s+([^\)\]\}]+)[\)\]\}]/gi, ' feat. $1 ');
  const delimiterRegex = /\s*(?:,\s*|\s+&\s+|\s+(?:feat\.?|ft\.?|featuring)\s+|\s+[xX×]\s+|\s+with\s+|\s+\/\s+)\s*/i;

  const parts = text.split(delimiterRegex);
  const seen = new Set<string>();
  const result: string[] = [];

  for (let p of parts) {
    p = p.trim().replace(/^[\(\[\{,]/, '').replace(/[\)\]\},]$/, '').trim();
    if (p && !seen.has(p.toLowerCase())) {
      seen.add(p.toLowerCase());
      result.push(p);
    }
  }

  return result.length > 0 ? result : [cleaned];
}

const FALLBACK_ARTWORK_SVG =
  'data:image/svg+xml;utf8,' +
  encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 240" width="240" height="240">
      <defs>
        <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#2b223c"/>
          <stop offset="50%" stop-color="#1d1829"/>
          <stop offset="100%" stop-color="#120f1a"/>
        </linearGradient>
      </defs>
      <rect width="240" height="240" fill="url(#g)"/>
      <circle cx="120" cy="120" r="62" fill="none" stroke="rgba(255,255,255,0.08)" stroke-width="2"/>
      <path d="M108 148V92l44-10v54" fill="none" stroke="rgba(208,188,255,0.65)" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/>
      <circle cx="98" cy="148" r="11" fill="rgba(208,188,255,0.65)"/>
      <circle cx="142" cy="136" r="11" fill="rgba(208,188,255,0.65)"/>
    </svg>`
  );

function extractVidFromHint(src: string, hint?: string): string | null {
  for (const s of [hint || '', src || '']) {
    if (!s) continue;
    if (s.startsWith('yt_') && s.length === 14) return s.slice(3);
    const m = s.match(/(?:\/vi\/|\/vi_webp\/|v=|youtu\.be\/|\/stream\/)([0-9A-Za-z_-]{11})/);
    if (m) return m[1];
    if (/^[0-9A-Za-z_-]{11}$/.test(s)) return s;
  }
  return null;
}

export function handleThumbnailError(event: Event, hint?: string) {
  const img = event.currentTarget as HTMLImageElement | null;
  if (!img) return;
  const step = Number(img.dataset.fbStep || '0');
  const vid = extractVidFromHint(img.src, hint);

  if (vid && step === 0) {
    img.dataset.fbStep = '1';
    img.src = `https://i.ytimg.com/vi/${vid}/mqdefault.jpg`;
  } else if (vid && step === 1) {
    img.dataset.fbStep = '2';
    img.src = `https://i.ytimg.com/vi/${vid}/0.jpg`;
  } else if (step < 3) {
    img.dataset.fbStep = '3';
    img.src = FALLBACK_ARTWORK_SVG;
  }
}

export function handleThumbnailLoad(event: Event, hint?: string) {
  const img = event.currentTarget as HTMLImageElement | null;
  if (!img) return;
  if (img.naturalWidth === 120 && img.naturalHeight === 90 && !img.src.startsWith('data:')) {
    handleThumbnailError(event, hint);
  }
}
