// High-resolution artwork resolver and iTunes/Apple Music 1400x1400 cover art cache

const itunesArtCache = new Map<string, string | null>();

/**
 * Transforms low-res YouTube Music or ytimg thumbnail URLs to ArchiveTune native edge-cached standard (544x544 / maxresdefault).
 */
export function getHighResArtwork(url?: string | null, videoId?: string | null, size = 544): string {
  if (videoId && (!url || url.includes('yt3.googleusercontent.com') || url.includes('default.jpg'))) {
    return `https://i.ytimg.com/vi/${videoId}/maxresdefault.jpg`;
  }
  if (!url) return videoId ? `https://i.ytimg.com/vi/${videoId}/maxresdefault.jpg` : '';

  // Google User Content & GGPHT (YouTube Music album covers)
  if (url.includes('googleusercontent.com') || url.includes('ggpht.com')) {
    // Avoid YouTube channel profile avatars when videoId is known
    if (url.includes('yt3.googleusercontent.com') && videoId) {
      return `https://i.ytimg.com/vi/${videoId}/maxresdefault.jpg`;
    }
    if (url.includes('=')) {
      return url.replace(/=.*$/i, `=w${size}-h${size}-l90-rj`);
    }
    if (/\/(?:[ws]\d+.*)$/i.test(url)) {
      return url.replace(/\/(?:[ws]\d+.*)$/i, `=w${size}-h${size}-l90-rj`);
    }
    return `${url}=w${size}-h${size}-l90-rj`;
  }

  // YouTube video thumbnails (ytimg)
  if (url.includes('ytimg.com')) {
    return url.replace(/(?:hqdefault|mqdefault|default|sddefault)\.jpg/i, 'maxresdefault.jpg');
  }

  return url;
}

/**
 * Attempts to fetch crisp 1400x1400 studio album cover art from iTunes / Apple Music search API.
 */
export async function fetchCrispAlbumArt(title: string, artist: string): Promise<string | null> {
  let cleanTitle = title.replace(/\s*[\(\[].*?[\)\]]/g, '').trim();
  let cleanArtist = artist.split(/[,&]/)[0].trim();
  if (/^youtube(?:\s+music)?$/i.test(cleanArtist)) {
    cleanArtist = '';
  }
  if (!cleanArtist && cleanTitle.includes(' - ')) {
    const parts = cleanTitle.split(' - ');
    cleanArtist = parts[0].trim();
    cleanTitle = parts[1].trim();
  }
  if (!cleanTitle && !cleanArtist) return null;

  const searchQuery = cleanArtist ? `${cleanArtist} ${cleanTitle}` : cleanTitle;
  const cacheKey = searchQuery.toLowerCase();
  if (itunesArtCache.has(cacheKey)) {
    return itunesArtCache.get(cacheKey) || null;
  }

  try {
    const query = encodeURIComponent(searchQuery);
    const res = await fetch(`https://itunes.apple.com/search?term=${query}&entity=song&limit=1`, {
      headers: { Accept: 'application/json' },
    });
    if (!res.ok) {
      itunesArtCache.set(cacheKey, null);
      return null;
    }
    const data = await res.json();
    if (data.results && data.results.length > 0 && data.results[0].artworkUrl100) {
      const artUrl = data.results[0].artworkUrl100.replace('100x100bb.jpg', '1400x1400bb.jpg');
      itunesArtCache.set(cacheKey, artUrl);
      return artUrl;
    }
  } catch {
    // Graceful fallback on network error
  }

  itunesArtCache.set(cacheKey, null);
  return null;
}
