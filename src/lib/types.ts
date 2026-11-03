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
  return raw
    .replace(/^similar\s+to\s+/i, '')
    .replace(/\s*[•·|\-]\s*\d+(\.\d+)?[kmbKMB]?\s*(views?|subscribers?|plays?|watching|listeners?).*$/gi, '')
    .replace(/\s+\d+(\.\d+)?[kmbKMB]?\s*(views?|subscribers?|plays?|watching|listeners?).*$/gi, '')
    .replace(/\s*-\s*topic$/i, '')
    .replace(/\s*[•·]\s*.*$/gi, '')
    .replace(/[\s\-\•\·\|\/,]+$/, '')
    .trim();
}

