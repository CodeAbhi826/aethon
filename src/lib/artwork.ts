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

  // YouTube video thumbnails (ytimg)
  if (trimmed.includes('i.ytimg.com')) {
    // If it's hqdefault.jpg, YouTube embeds black letterbox bars! Replace with mqdefault.jpg to eliminate black bars
    if (trimmed.includes('hqdefault.jpg')) {
      const vid = cleanVid || trimmed.match(/\/vi\/([^\/]+)/)?.[1];
      if (vid) {
        return `https://i.ytimg.com/vi/${vid}/mqdefault.jpg`;
      }
      return trimmed.replace('hqdefault.jpg', 'mqdefault.jpg').split('?')[0];
    }
    if (trimmed.includes('maxresdefault.jpg')) {
      return trimmed.replace('maxresdefault.jpg', 'mqdefault.jpg');
    }
  }

  return trimmed;
}

/**
 * Disabled third-party iTunes cover replacement so Now Playing / Split Mode always shows the exact same artwork as the selected track card.
 */
export async function fetchCrispAlbumArt(_title: string, _artist: string): Promise<string | null> {
  return null;
}
