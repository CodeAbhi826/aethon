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
    .replace(/[\s\-\•\·\|\/,]+$/, '')
    .trim();
}

export function splitArtists(raw: string): string[] {
  if (!raw) return [];
  const cleaned = sanitizeArtist(raw);
  if (!cleaned) return [];

  // Protect established bands/duos whose proper names contain punctuation
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

  // Pre-normalize (feat. X) or [ft. X] inside brackets so nested artists get split cleanly
  let text = cleaned.replace(/[\(\[\{]\s*(?:feat\.?|ft\.?|featuring|with)\s+([^\)\]\}]+)[\)\]\}]/gi, ' feat. $1 ');

  // Match common collaboration separators:
  // - comma: ,
  // - ampersand: & (with whitespace around it)
  // - feat / ft / featuring: (?:feat\.?|ft\.?|featuring)
  // - x / × (as standalone word or surrounded by spaces): \s+[xX×]\s+
  // - with: \s+with\s+
  // - slash: \s+\/\s+ (require spaces around slash to avoid breaking band acronyms)
  const delimiterRegex = /\s*(?:,\s*|\s+&\s+|\s+(?:feat\.?|ft\.?|featuring)\s+|\s+[xX×]\s+|\s+with\s+|\s+\/\s+)\s*/i;

  const parts = text.split(delimiterRegex);
  const seen = new Set<string>();
  const result: string[] = [];

  for (let p of parts) {
    p = p.trim().replace(/^[\(\[\{]/, '').replace(/[\)\]\}]$/, '').trim();
    if (p && !seen.has(p.toLowerCase())) {
      seen.add(p.toLowerCase());
      result.push(p);
    }
  }

  return result.length > 0 ? result : [cleaned];
}


