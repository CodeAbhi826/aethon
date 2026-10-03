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
}

export interface LyricsResult {
  synced: boolean;
  lines: LyricLine[];
  plain_text?: string;
}

export interface YouTubeTrack {
  id: string;
  title: string;
  artist: string;
  duration: number;
  thumbnail: string;
  stream_url: string;
}

export type TabType = 'home' | 'search' | 'library' | 'stats' | 'settings';
