// High-resolution artwork resolver preserving exact YouTube Music square album covers across cards & Split/NowPlaying view

/**
 * Preserves the exact YouTube Music cover art from selection cards and upgrades googleusercontent/ggpht size to 544x544.
 * Never replaces square album covers with 16:9 video frames or broken 404 maxresdefault.jpg URLs.
 */
export function getHighResArtwork(url?: string | null, videoId?: string | null, size = 544): string {
  const trimmed = (url || '').trim();
  const cleanVid = videoId ? videoId.replace(/^yt_/, '').trim() : '';

  if (!trimmed || trimmed.endsWith('/default.jpg')) {
    return cleanVid ? `https://i.ytimg.com/vi/${cleanVid}/mqdefault.jpg` : '';
  }

  // Upgrade YouTube Music square album covers (lh3.googleusercontent.com / yt3.googleusercontent.com / yt3.ggpht.com)
  if (trimmed.includes('googleusercontent.com') || trimmed.includes('ggpht.com')) {
    if (trimmed.includes('=')) {
      return trimmed.replace(/=w\d+-h\d+[^&?]*/i, `=w${size}-h${size}-l90-rj`).replace(/=s\d+[^&?]*/i, `=w${size}-h${size}-l90-rj`);
    }
    return `${trimmed}=w${size}-h${size}-l90-rj`;
  }

  // Keep signed ytimg URLs intact; if a stale maxresdefault.jpg URL was cached, use mqdefault.jpg which never 404s
  if (trimmed.includes('i.ytimg.com') && trimmed.includes('maxresdefault.jpg')) {
    return trimmed.replace('maxresdefault.jpg', 'mqdefault.jpg');
  }

  return trimmed;
}

/**
 * Disabled third-party iTunes cover replacement so Now Playing / Split Mode always shows the exact same artwork as the selected track card.
 */
export async function fetchCrispAlbumArt(_title: string, _artist: string): Promise<string | null> {
  return null;
}
