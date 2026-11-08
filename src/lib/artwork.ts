// High-resolution artwork resolver and iTunes/Apple Music 1400x1400 cover art cache

const itunesArtCache = new Map<string, string | null>();

/**
 * Transforms low-res YouTube Music or ytimg thumbnail URLs to ArchiveTune native edge-cached standard (544x544 / maxresdefault).
 */
export function getHighResArtwork(url?: string | null, videoId?: string | null, size = 544): string {
  if (!url && videoId) {
    return `https://i.ytimg.com/vi/${videoId}/maxresdefault.jpg`;
  }
  if (!url) return '';

  // Google User Content & GGPHT (YouTube Music artwork & channel avatars)
  if (url.includes('googleusercontent.com') || url.includes('ggpht.com')) {
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
  const cleanTitle = title.replace(/\s*[\(\[].*?[\)\]]/g, '').trim();
  const cleanArtist = artist.split(/[,&]/)[0].trim();
  if (!cleanTitle) return null;

  const cacheKey = `${cleanArtist.toLowerCase()}::${cleanTitle.toLowerCase()}`;
  if (itunesArtCache.has(cacheKey)) {
    return itunesArtCache.get(cacheKey) || null;
  }

  try {
    const query = encodeURIComponent(`${cleanArtist} ${cleanTitle}`);
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
