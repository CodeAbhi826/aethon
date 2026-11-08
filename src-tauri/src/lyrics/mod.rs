use lofty::prelude::{ItemKey, TaggedFileExt};
use lofty::probe::Probe;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricLine {
    pub time: f64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub romanized: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsResult {
    pub synced: bool,
    pub lines: Vec<LyricLine>,
    pub plain_text: Option<String>,
    pub provider: String,
    pub has_romanized: bool,
}

#[derive(Deserialize)]
struct LrcLibResponse {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    duration: Option<f64>,
}

static LYRICS_HTTP: OnceLock<reqwest::Client> = OnceLock::new();
static MXM_TOKEN_CACHE: OnceLock<Mutex<Option<(String, u64)>>> = OnceLock::new();

fn lyrics_client() -> &'static reqwest::Client {
    LYRICS_HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_nodelay(true)
            .timeout(std::time::Duration::from_millis(3600))
            .connect_timeout(std::time::Duration::from_millis(2000))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn mxm_token_store() -> &'static Mutex<Option<(String, u64)>> {
    MXM_TOKEN_CACHE.get_or_init(|| Mutex::new(None))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct LyricsService;

impl LyricsService {
    /// Clean noisy YouTube/streaming suffixes like "(Official Video)", "[Lyric Video]", "ft. X"
    pub fn clean_metadata(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut depth_paren = 0i32;
        let mut depth_brack = 0i32;
        for ch in raw.chars() {
            match ch {
                '(' => depth_paren += 1,
                ')' => {
                    if depth_paren > 0 {
                        depth_paren -= 1;
                    }
                }
                '[' => depth_brack += 1,
                ']' => {
                    if depth_brack > 0 {
                        depth_brack -= 1;
                    }
                }
                _ if depth_paren == 0 && depth_brack == 0 => out.push(ch),
                _ => {}
            }
        }
        let cleaned = out
            .split(" - ")
            .next()
            .unwrap_or(&out)
            .split(" | ")
            .next()
            .unwrap_or(&out)
            .split(" // ")
            .next()
            .unwrap_or(&out)
            .split(" • ")
            .next()
            .unwrap_or(&out)
            .trim();
        if cleaned.is_empty() {
            raw.trim().to_string()
        } else {
            cleaned.to_string()
        }
    }

    /// Multi-Provider Parallel Racing & Priority Fallback Engine:
    /// Main Priority Tier:
    ///   1. Musixmatch (Desktop Subtitles & RichSync LRC + Mirror fallback)
    ///   2. Apple Music (Time-Synced TTML/Syllable & LRC)
    ///   3. Spotify (Time-Synced Color-Lyrics LRC)
    ///   4. YouTube Music (Official InnerTube `WEB_REMIX` `MPLYt_` `timedLyricsData` & plain shelf)
    /// Secondary Fallback & CJK Enrichment Tier:
    ///   5. LRCLIB (`lrclib.net` with duration validation)
    ///   6. NetEase Cloud Music (`music.163.com` synced `lrc` + `romalrc` + `tlyric`)
    ///   7. SimpMusic (`api-lyrics.simpmusic.org`) & KuGou (`lyrics.kugou.com`) & `Lyrics.ovh`
    pub async fn fetch_lyrics(
        title: &str,
        artist: &str,
        duration: Option<f64>,
        file_path: Option<&str>,
        video_id: Option<&str>,
        preferred_provider: Option<&str>,
    ) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
        // Tier 0a: Local .lrc sidecar or embedded audio tags first (0 ms)
        if let Some(fp) = file_path {
            if !fp.starts_with("http://") && !fp.starts_with("https://") {
                if let Some(local_res) = Self::fetch_local_or_embedded(fp) {
                    return Ok(Self::enrich_romanization(local_res));
                }
            }
        }

        let clean_title = Self::clean_metadata(title);
        let clean_artist = Self::clean_metadata(artist)
            .split(&[',', '&', '/'][..])
            .next()
            .unwrap_or(artist)
            .trim()
            .to_string();

        let resolved_vid = video_id
            .map(|s| s.to_string())
            .or_else(|| file_path.and_then(crate::youtube::YouTubeService::extract_video_id));

        let pref = preferred_provider.unwrap_or("auto").trim().to_lowercase();

        // Tier 0b: Check Disk Cache (~/.cache/aethon/lyrics/) if synced
        let c_key = Self::disk_cache_key(&clean_title, &clean_artist, resolved_vid.as_deref());
        if pref == "auto" || pref.is_empty() {
            if let Some(cached) = Self::read_disk_cache(&c_key) {
                if cached.synced && !cached.lines.is_empty() {
                    return Ok(Self::enrich_romanization(cached));
                }
            }
        }

        let vid_ref = resolved_vid.as_deref().unwrap_or("");

        // Launch the Main Priority Providers + Secondary Fallbacks concurrently so total latency is <= 1 round-trip!
        let mxm_fut = Self::fetch_musixmatch(&clean_title, &clean_artist, duration);
        let am_fut = Self::fetch_apple_music(&clean_title, &clean_artist, duration, vid_ref);
        let sp_fut = Self::fetch_spotify(&clean_title, &clean_artist, duration);
        let ytm_fut = Self::fetch_youtube_music(vid_ref, &clean_title, &clean_artist);
        let lrc_fut = Self::fetch_lrclib(&clean_title, &clean_artist, duration);
        let ne_fut = Self::fetch_netease(&clean_title, &clean_artist, duration);
        let simp_fut = Self::fetch_simpmusic(vid_ref);
        let kg_fut = Self::fetch_kugou(&clean_title, &clean_artist, duration);

        let (mxm_res, am_res, sp_res, ytm_res, lrc_res, ne_res, simp_res, kg_res) = tokio::join!(
            mxm_fut, am_fut, sp_fut, ytm_fut, lrc_fut, ne_fut, simp_fut, kg_fut
        );

        // Extract NetEase romalrc / tlyric lines if available so ANY winning provider gets official CJK/Kanji romanization
        let netease_donor_lines: Option<Vec<LyricLine>> = ne_res
            .as_ref()
            .filter(|r| r.has_romanized && !r.lines.is_empty())
            .map(|r| r.lines.clone());

        // Score all candidates automatically by Synced Status, Video-ID Timeline Lock, and Duration Alignment
        // so music-video vs. album-audio timestamp mismatches are automatically rejected!
        let raw_candidates: Vec<(&str, Option<LyricsResult>)> = vec![
            ("simpmusic", simp_res),
            ("youtube_music", ytm_res),
            ("lrclib", lrc_res),
            ("musixmatch", mxm_res),
            ("apple_music", am_res),
            ("spotify", sp_res),
            ("netease", ne_res),
            ("kugou", kg_res),
        ];

        let mut synced_candidates: Vec<(i64, LyricsResult)> = Vec::new();
        let mut plain_fallback: Option<LyricsResult> = None;
        let target_dur = duration.unwrap_or(0.0);
        let has_vid = !vid_ref.is_empty();

        for (name, opt_res) in raw_candidates {
            if let Some(res) = opt_res {
                if res.synced && res.lines.len() >= 3 {
                    let mut score: i64 = match name {
                        // Video-ID indexed providers are locked to the exact YouTube video timeline
                        "simpmusic" if has_vid => 960,
                        "youtube_music" if has_vid => 950,
                        "lrclib" => 930,
                        "musixmatch" => 920,
                        "apple_music" => 910,
                        "spotify" => 900,
                        "netease" => 890,
                        "kugou" => 870,
                        _ => 850,
                    };

                    if let (Some(first_l), Some(last_l)) = (res.lines.first(), res.lines.last()) {
                        let last_t = last_l.time;
                        let span = last_t - first_l.time;
                        if span < 12.0 {
                            score -= 600;
                        }
                        if target_dur > 25.0 {
                            // If last lyric line is > 5.5s past the actual audio track's total duration,
                            // this lyric file belongs to a longer Music Video / Extended Version (out of sync!)
                            if last_t > target_dur + 5.5 {
                                score -= 700;
                            } else if last_t >= target_dur * 0.55 && last_t <= target_dur + 2.5 {
                                // Last line sits cleanly in the final half of the track without overrunning
                                score += 120;
                            } else if last_t < target_dur * 0.35 {
                                score -= 350;
                            }
                        }
                    }

                    if res.has_romanized {
                        score += 25;
                    }

                    synced_candidates.push((score, res));
                } else if plain_fallback.is_none() && res.plain_text.is_some() {
                    plain_fallback = Some(res);
                }
            }
        }

        synced_candidates.sort_by(|a, b| b.0.cmp(&a.0));

        // Pass 1: Return the highest-scoring timeline-aligned SYNCED lyrics
        if let Some((_, mut best_res)) = synced_candidates.into_iter().next() {
            if let Some(ref donor) = netease_donor_lines {
                if !best_res.has_romanized {
                    Self::merge_donor_roma_trans(&mut best_res.lines, donor);
                    best_res.has_romanized = best_res.lines.iter().any(|l| l.romanized.is_some());
                }
            }
            let enriched = Self::enrich_romanization(best_res);
            Self::write_disk_cache(&c_key, &enriched);
            return Ok(enriched);
        }

        // Pass 2: If raw title differed from clean_title, try exact LRCLIB with raw title
        if title != clean_title {
            if let Some(mut res) = Self::fetch_lrclib(title, artist, duration).await {
                if res.synced && !res.lines.is_empty() {
                    let enriched = Self::enrich_romanization(res);
                    Self::write_disk_cache(&c_key, &enriched);
                    return Ok(enriched);
                } else if plain_fallback.is_none() && res.plain_text.is_some() {
                    plain_fallback = Some(res);
                }
            }
        }

        // Pass 3: Plain lyrics fallback (from Musixmatch, YouTube Music, Apple Music, Spotify, LRCLIB, or Lyrics.ovh)
        if let Some(plain) = plain_fallback {
            let enriched = Self::enrich_romanization(plain);
            Self::write_disk_cache(&c_key, &enriched);
            return Ok(enriched);
        }

        if let Some(ovh) = Self::fetch_lyrics_ovh(&clean_title, &clean_artist).await {
            let enriched = Self::enrich_romanization(ovh);
            return Ok(enriched);
        }

        Err("No lyrics found across Musixmatch, Apple Music, Spotify, YouTube Music, LRCLIB, NetEase, SimpMusic, or KuGou".into())
    }

    fn merge_donor_roma_trans(target: &mut [LyricLine], donor: &[LyricLine]) {
        for line in target.iter_mut() {
            if line.romanized.is_none() {
                if let Some(closest) = donor.iter().min_by(|a, b| {
                    (a.time - line.time)
                        .abs()
                        .partial_cmp(&(b.time - line.time).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    if (closest.time - line.time).abs() <= 1.2 {
                        if closest.romanized.is_some() {
                            line.romanized = closest.romanized.clone();
                        }
                        if line.translation.is_none() && closest.translation.is_some() {
                            line.translation = closest.translation.clone();
                        }
                    }
                }
            }
        }
    }

    fn disk_cache_dir() -> std::path::PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("aethon")
            .join("lyrics")
    }

    fn disk_cache_key(title: &str, artist: &str, video_id: Option<&str>) -> String {
        if let Some(vid) = video_id.filter(|v| !v.is_empty()) {
            format!("v41_yt_{}", vid)
        } else {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            title.to_lowercase().hash(&mut hasher);
            artist.to_lowercase().hash(&mut hasher);
            format!("v41_{:016x}", hasher.finish())
        }
    }

    fn read_disk_cache(key: &str) -> Option<LyricsResult> {
        let path = Self::disk_cache_dir().join(format!("{}.json", key));
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(path) {
                if let Ok(res) = serde_json::from_str::<LyricsResult>(&data) {
                    if !res.lines.is_empty() || res.plain_text.is_some() {
                        return Some(res);
                    }
                }
            }
        }
        None
    }

    fn write_disk_cache(key: &str, result: &LyricsResult) {
        let dir = Self::disk_cache_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.json", key));
        if let Ok(data) = serde_json::to_string(result) {
            let _ = std::fs::write(path, data);
        }
    }

    pub fn has_cjk_characters(s: &str) -> bool {
        s.chars().any(|c| {
            let u = c as u32;
            (0x4E00..=0x9FFF).contains(&u) || (0x3400..=0x4DBF).contains(&u)
        })
    }

    fn fetch_local_or_embedded(file_path: &str) -> Option<LyricsResult> {
        let path = Path::new(file_path);
        if !path.exists() {
            return None;
        }

        let lrc_path = path.with_extension("lrc");
        if lrc_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&lrc_path) {
                let lines = Self::parse_lrc(&content);
                if !lines.is_empty() {
                    return Some(LyricsResult {
                        synced: true,
                        lines,
                        plain_text: Some(content),
                        provider: "Local .LRC".to_string(),
                        has_romanized: false,
                    });
                }
            }
        }

        if let Ok(tagged) = Probe::open(path).and_then(|p| p.read()) {
            if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
                if let Some(lyric_str) = tag.get_string(&ItemKey::Lyrics) {
                    let lines = Self::parse_lrc(lyric_str);
                    if !lines.is_empty() {
                        return Some(LyricsResult {
                            synced: true,
                            lines,
                            plain_text: Some(lyric_str.to_string()),
                            provider: "Embedded Tag".to_string(),
                            has_romanized: false,
                        });
                    } else if !lyric_str.trim().is_empty() {
                        return Some(LyricsResult {
                            synced: false,
                            lines: Vec::new(),
                            plain_text: Some(lyric_str.to_string()),
                            provider: "Embedded Tag".to_string(),
                            has_romanized: false,
                        });
                    }
                }
            }
        }

        None
    }

    // =========================================================================
    // MAIN PRIORITY PROVIDER 1: MUSIXMATCH (Desktop API + Mirror Fallback)
    // =========================================================================
    async fn get_musixmatch_token() -> Option<String> {
        let now = now_secs();
        if let Some((tok, exp)) = mxm_token_store().lock().clone() {
            if exp > now && !tok.is_empty() {
                return Some(tok);
            }
        }
        let client = lyrics_client();
        let url = format!(
            "https://apic-desktop.musixmatch.com/ws/1.1/token.get?app_id=web-desktop-app-v1.0&t={}",
            now * 1000
        );
        let resp = client
            .get(&url)
            .header("Cookie", "x-mxm-token-guid=aethon-desktop-guid")
            .send()
            .await
            .ok()?;
        let json: serde_json::Value = resp.json().await.ok()?;
        let token = json
            .pointer("/message/body/user_token")
            .and_then(|v| v.as_str())?
            .trim()
            .to_string();
        if token.is_empty() || token.contains("UpgradeOnly") {
            return None;
        }
        *mxm_token_store().lock() = Some((token.clone(), now + 900));
        Some(token)
    }

    async fn fetch_musixmatch(
        title: &str,
        artist: &str,
        duration: Option<f64>,
    ) -> Option<LyricsResult> {
        let client = lyrics_client();

        // 1A: Direct Musixmatch Desktop macro.subtitles.get
        if let Some(token) = Self::get_musixmatch_token().await {
            let mut url = format!(
                "https://apic-desktop.musixmatch.com/ws/1.1/macro.subtitles.get?format=json&namespace=lyrics_richsynched&subtitle_format=lrc&app_id=web-desktop-app-v1.0&q_track={}&q_artist={}&usertoken={}",
                urlencoding::encode(title),
                urlencoding::encode(artist),
                urlencoding::encode(&token)
            );
            if let Some(d) = duration.filter(|&d| d > 0.0) {
                url.push_str(&format!(
                    "&q_duration={:.0}&f_subtitle_length={:.0}",
                    d, d
                ));
            }
            if let Ok(resp) = client
                .get(&url)
                .header("Cookie", "x-mxm-token-guid=aethon-desktop-guid")
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(sub_str) = json
                            .pointer("/message/body/macro_calls/track.subtitles.get/message/body/subtitle_list/0/subtitle/subtitle_body")
                            .and_then(|v| v.as_str())
                        {
                            let lines = Self::parse_lrc(sub_str);
                            if !lines.is_empty() {
                                return Some(LyricsResult {
                                    synced: true,
                                    lines,
                                    plain_text: Some(sub_str.to_string()),
                                    provider: "Musixmatch".to_string(),
                                    has_romanized: false,
                                });
                            }
                        }
                        if let Some(plain_str) = json
                            .pointer("/message/body/macro_calls/track.lyrics.get/message/body/lyrics/lyrics_body")
                            .and_then(|v| v.as_str())
                        {
                            let clean_plain = plain_str
                                .lines()
                                .filter(|l| !l.contains("******* This Lyrics is NOT for Commercial use"))
                                .collect::<Vec<_>>()
                                .join("\n")
                                .trim()
                                .to_string();
                            if !clean_plain.is_empty() {
                                return Some(LyricsResult {
                                    synced: false,
                                    lines: Vec::new(),
                                    plain_text: Some(clean_plain),
                                    provider: "Musixmatch".to_string(),
                                    has_romanized: false,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 1B: Musixmatch Paxsenix Mirror Fallback
        let mirror_url = format!(
            "https://lyrics.paxsenix.org/musixmatch/lyrics?title={}&artist={}",
            urlencoding::encode(title),
            urlencoding::encode(artist)
        );
        if let Ok(resp) = client.get(&mirror_url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Some(res) = Self::parse_generic_lyrics_payload(&text, "Musixmatch") {
                        return Some(res);
                    }
                }
            }
        }

        None
    }

    // =========================================================================
    // MAIN PRIORITY PROVIDER 2: APPLE MUSIC (Syllable/Line Synced TTML & LRC)
    // =========================================================================
    async fn fetch_apple_music(
        title: &str,
        artist: &str,
        _duration: Option<f64>,
        video_id: &str,
    ) -> Option<LyricsResult> {
        let client = lyrics_client();
        let query = format!("{} {}", title, artist);
        let search_url = format!(
            "https://lyrics.paxsenix.org/apple-music/search?q={}",
            urlencoding::encode(&query)
        );

        if let Ok(s_resp) = client.get(&search_url).send().await {
            if s_resp.status().is_success() {
                if let Ok(s_json) = s_resp.json::<serde_json::Value>().await {
                    let song_id = s_json
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|item| item.get("id"))
                        .and_then(|id| {
                            id.as_str()
                                .map(String::from)
                                .or_else(|| id.as_i64().map(|n| n.to_string()))
                        });
                    if let Some(id) = song_id {
                        let lyr_url = format!(
                            "https://lyrics.paxsenix.org/apple-music/lyrics?id={}",
                            urlencoding::encode(&id)
                        );
                        if let Ok(l_resp) = client.get(&lyr_url).send().await {
                            if l_resp.status().is_success() {
                                if let Ok(body) = l_resp.text().await {
                                    if let Some(res) =
                                        Self::parse_generic_lyrics_payload(&body, "Apple Music")
                                    {
                                        return Some(res);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Apple Music RichSync / TTML via SimpMusic if video_id is known
        if !video_id.is_empty() {
            let url = format!(
                "https://api-lyrics.simpmusic.org/v1/{}",
                urlencoding::encode(video_id)
            );
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(first) = json.get("data").and_then(|d| d.as_array()).and_then(|a| a.first()) {
                            if let Some(rich) = first
                                .get("richSyncLyrics")
                                .or_else(|| first.get("syncedLyrics"))
                                .and_then(|s| s.as_str())
                            {
                                let lines = Self::parse_lrc(rich);
                                if !lines.is_empty() {
                                    return Some(LyricsResult {
                                        synced: true,
                                        lines,
                                        plain_text: first
                                            .get("plainLyrics")
                                            .and_then(|s| s.as_str())
                                            .map(String::from),
                                        provider: "Apple Music".to_string(),
                                        has_romanized: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    // =========================================================================
    // MAIN PRIORITY PROVIDER 3: SPOTIFY (Color-Lyrics Time-Synced Lines)
    // =========================================================================
    async fn fetch_spotify(
        title: &str,
        artist: &str,
        _duration: Option<f64>,
    ) -> Option<LyricsResult> {
        let client = lyrics_client();
        let url = format!(
            "https://lyrics.paxsenix.org/spotify/lyrics?title={}&artist={}",
            urlencoding::encode(title),
            urlencoding::encode(artist)
        );
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.text().await {
                    if let Some(res) = Self::parse_generic_lyrics_payload(&body, "Spotify") {
                        return Some(res);
                    }
                }
            }
        }
        None
    }

    fn parse_generic_lyrics_payload(raw: &str, provider_name: &str) -> Option<LyricsResult> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        // Check if raw response is direct LRC text
        if trimmed.starts_with('[') {
            let lines = Self::parse_lrc(trimmed);
            if !lines.is_empty() {
                return Some(LyricsResult {
                    synced: true,
                    lines,
                    plain_text: Some(trimmed.to_string()),
                    provider: provider_name.to_string(),
                    has_romanized: false,
                });
            }
        }

        let json: serde_json::Value = serde_json::from_str(trimmed).ok()?;

        // Case 1: LRC string field (`lrc`, `syncedLyrics`, `lyrics`, `subtitles`)
        for key in ["syncedLyrics", "lrc", "subtitles", "lyrics"] {
            if let Some(s) = json.get(key).and_then(|v| v.as_str()) {
                let lines = Self::parse_lrc(s);
                if !lines.is_empty() {
                    return Some(LyricsResult {
                        synced: true,
                        lines,
                        plain_text: Some(s.to_string()),
                        provider: provider_name.to_string(),
                        has_romanized: false,
                    });
                }
            }
        }

        // Case 2: Array of timed objects (`content`, `lines`, or top-level array)
        let arr_opt = json
            .as_array()
            .or_else(|| json.get("content").and_then(|v| v.as_array()))
            .or_else(|| json.get("lines").and_then(|v| v.as_array()))
            .or_else(|| json.pointer("/lyrics/lines").and_then(|v| v.as_array()));

        if let Some(arr) = arr_opt {
            let mut lines = Vec::new();
            for item in arr {
                let text = item
                    .get("words")
                    .or_else(|| item.get("text"))
                    .and_then(|v| {
                        if let Some(s) = v.as_str() {
                            Some(s.trim().to_string())
                        } else if let Some(syllables) = v.as_array() {
                            let joined: String = syllables
                                .iter()
                                .filter_map(|sy| sy.get("text").and_then(|t| t.as_str()))
                                .collect::<Vec<_>>()
                                .join(" ");
                            Some(joined.trim().to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                let time_ms = item
                    .get("startTimeMs")
                    .or_else(|| item.get("timestamp"))
                    .or_else(|| item.get("time"))
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_i64().map(|n| n as f64))
                            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                    })
                    .unwrap_or(-1.0);

                if !text.is_empty() && text != "♪" && time_ms >= 0.0 {
                    // Convert ms to seconds if > 500 or if startTimeMs was used
                    let secs = if item.get("startTimeMs").is_some()
                        || item.get("timestamp").is_some()
                        || time_ms > 500.0
                    {
                        time_ms / 1000.0
                    } else {
                        time_ms
                    };
                    lines.push(LyricLine {
                        time: secs,
                        text,
                        romanized: None,
                        translation: None,
                    });
                }
            }
            if !lines.is_empty() {
                return Some(LyricsResult {
                    synced: true,
                    lines,
                    plain_text: None,
                    provider: provider_name.to_string(),
                    has_romanized: false,
                });
            }
        }

        // Case 3: Plain text fallback inside JSON
        if let Some(plain) = json
            .get("plainLyrics")
            .or_else(|| json.get("plain"))
            .or_else(|| json.get("lyrics"))
            .and_then(|v| v.as_str())
        {
            if !plain.trim().is_empty() {
                return Some(LyricsResult {
                    synced: false,
                    lines: Vec::new(),
                    plain_text: Some(plain.trim().to_string()),
                    provider: provider_name.to_string(),
                    has_romanized: false,
                });
            }
        }

        None
    }

    // =========================================================================
    // MAIN PRIORITY PROVIDER 4: YOUTUBE MUSIC (Recursive InnerTube WEB_REMIX)
    // =========================================================================
    async fn fetch_youtube_music(
        video_id: &str,
        title: &str,
        artist: &str,
    ) -> Option<LyricsResult> {
        let mut vid = video_id.trim().to_string();
        if vid.is_empty() && !title.is_empty() {
            let q = format!("{} {}", title, artist);
            if let Some(tracks) =
                crate::youtube::YouTubeService::search_innertube(&q, 1, "IN").await
            {
                if let Some(first) = tracks.first() {
                    vid = first.id.clone();
                }
            }
        }
        if vid.is_empty() {
            return None;
        }

        let client = lyrics_client();
        let next_payload = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "hl": "en",
                    "gl": "IN"
                }
            },
            "videoId": vid
        });

        let resp = client
            .post("https://music.youtube.com/youtubei/v1/next?prettyPrint=false")
            .header("Referer", "https://music.youtube.com/")
            .header("Origin", "https://music.youtube.com")
            .json(&next_payload)
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let next_json: serde_json::Value = resp.json().await.ok()?;
        let browse_id = Self::find_lyrics_browse_id_recursive(&next_json)?;

        let browse_payload = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "hl": "en",
                    "gl": "IN"
                }
            },
            "browseId": browse_id
        });

        let b_resp = client
            .post("https://music.youtube.com/youtubei/v1/browse?prettyPrint=false")
            .header("Referer", "https://music.youtube.com/")
            .header("Origin", "https://music.youtube.com")
            .json(&browse_payload)
            .send()
            .await
            .ok()?;
        if !b_resp.status().is_success() {
            return None;
        }
        let b_json: serde_json::Value = b_resp.json().await.ok()?;

        // 1. Recursive search for `timedLyricsData` anywhere in the browse tree
        let mut timed_lines = Vec::new();
        Self::extract_ytm_timed_lyrics_recursive(&b_json, &mut timed_lines);
        if !timed_lines.is_empty() {
            return Some(LyricsResult {
                synced: true,
                lines: timed_lines,
                plain_text: None,
                provider: "YouTube Music".to_string(),
                has_romanized: false,
            });
        }

        // 2. Recursive search for plain `musicDescriptionShelfRenderer` lyrics
        if let Some(plain_text) = Self::extract_ytm_plain_lyrics_recursive(&b_json) {
            if !plain_text.trim().is_empty() {
                let maybe_lrc = Self::parse_lrc(&plain_text);
                if !maybe_lrc.is_empty() {
                    return Some(LyricsResult {
                        synced: true,
                        lines: maybe_lrc,
                        plain_text: Some(plain_text),
                        provider: "YouTube Music".to_string(),
                        has_romanized: false,
                    });
                }
                return Some(LyricsResult {
                    synced: false,
                    lines: Vec::new(),
                    plain_text: Some(plain_text),
                    provider: "YouTube Music".to_string(),
                    has_romanized: false,
                });
            }
        }

        None
    }

    fn find_lyrics_browse_id_recursive(val: &serde_json::Value) -> Option<String> {
        match val {
            serde_json::Value::Object(map) => {
                if let Some(b_id) = map.get("browseId").and_then(|v| v.as_str()) {
                    if b_id.starts_with("MPLYt") {
                        return Some(b_id.to_string());
                    }
                }
                for v in map.values() {
                    if let Some(found) = Self::find_lyrics_browse_id_recursive(v) {
                        return Some(found);
                    }
                }
                None
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(found) = Self::find_lyrics_browse_id_recursive(item) {
                        return Some(found);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn extract_ytm_timed_lyrics_recursive(val: &serde_json::Value, out: &mut Vec<LyricLine>) {
        match val {
            serde_json::Value::Object(map) => {
                if let Some(arr) = map.get("timedLyricsData").and_then(|v| v.as_array()) {
                    for item in arr {
                        let text = item
                            .get("lyricLine")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        let start_ms = item
                            .pointer("/cueRange/startTimeMilliseconds")
                            .and_then(|v| {
                                v.as_str()
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .or_else(|| v.as_f64())
                            })
                            .unwrap_or(0.0);
                        if !text.is_empty() && text != "♪" {
                            out.push(LyricLine {
                                time: start_ms / 1000.0,
                                text,
                                romanized: None,
                                translation: None,
                            });
                        }
                    }
                    return;
                }
                for v in map.values() {
                    Self::extract_ytm_timed_lyrics_recursive(v, out);
                    if !out.is_empty() {
                        return;
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    Self::extract_ytm_timed_lyrics_recursive(item, out);
                    if !out.is_empty() {
                        return;
                    }
                }
            }
            _ => {}
        }
    }

    fn extract_ytm_plain_lyrics_recursive(val: &serde_json::Value) -> Option<String> {
        match val {
            serde_json::Value::Object(map) => {
                if let Some(shelf) = map.get("musicDescriptionShelfRenderer") {
                    if let Some(runs) = shelf
                        .pointer("/description/runs")
                        .and_then(|r| r.as_array())
                    {
                        let mut text = String::new();
                        for r in runs {
                            if let Some(t) = r.get("text").and_then(|s| s.as_str()) {
                                text.push_str(t);
                            }
                        }
                        if !text.trim().is_empty() {
                            return Some(text);
                        }
                    }
                }
                for v in map.values() {
                    if let Some(found) = Self::extract_ytm_plain_lyrics_recursive(v) {
                        return Some(found);
                    }
                }
                None
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(found) = Self::extract_ytm_plain_lyrics_recursive(item) {
                        return Some(found);
                    }
                }
                None
            }
            _ => None,
        }
    }

    // =========================================================================
    // SECONDARY PROVIDERS: LRCLIB, NETEASE, KUGOU, SIMPMUSIC, LYRICS.OVH
    // =========================================================================
    async fn fetch_lrclib(
        title: &str,
        artist: &str,
        duration: Option<f64>,
    ) -> Option<LyricsResult> {
        if title.is_empty() {
            return None;
        }
        let client = lyrics_client();
        let mut url = format!(
            "https://lrclib.net/api/get?track_name={}&artist_name={}",
            urlencoding::encode(title),
            urlencoding::encode(artist)
        );
        if let Some(d) = duration {
            if d > 0.0 {
                url.push_str(&format!("&duration={:.0}", d));
            }
        }

        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<LrcLibResponse>().await {
                    let parsed = Self::parse_lrclib_response(data);
                    if parsed.synced || parsed.plain_text.is_some() {
                        return Some(parsed);
                    }
                }
            }
        }

        let search_url = format!(
            "https://lrclib.net/api/search?q={}%20{}",
            urlencoding::encode(title),
            urlencoding::encode(artist)
        );
        let resp = client.get(&search_url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let mut items: Vec<LrcLibResponse> = resp.json().await.ok()?;
        if items.is_empty() {
            return None;
        }

        if let Some(target_d) = duration.filter(|&d| d > 15.0) {
            let strict_matches: Vec<_> = items
                .iter()
                .filter(|item| {
                    item.duration
                        .map(|d| (d - target_d).abs() <= 4.5)
                        .unwrap_or(false)
                })
                .count()
                .gt(&0)
                .then(|| ())
                .into_iter()
                .collect();
            let max_delta = if !strict_matches.is_empty() { 4.5 } else { 8.0 };
            items.retain(|item| {
                item.duration
                    .map(|d| (d - target_d).abs() <= max_delta)
                    .unwrap_or(true)
            });
            if items.is_empty() {
                return None;
            }
        }

        items.sort_by(|a, b| {
            let a_synced = a
                .synced_lyrics
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            let b_synced = b
                .synced_lyrics
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            if a_synced != b_synced {
                return b_synced.cmp(&a_synced);
            }
            if let Some(target_d) = duration.filter(|&d| d > 0.0) {
                let da = a.duration.map(|d| (d - target_d).abs()).unwrap_or(999.0);
                let db = b.duration.map(|d| (d - target_d).abs()).unwrap_or(999.0);
                return da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal);
            }
            std::cmp::Ordering::Equal
        });

        items.into_iter().next().map(Self::parse_lrclib_response)
    }

    fn parse_lrclib_response(data: LrcLibResponse) -> LyricsResult {
        if let Some(synced) = data.synced_lyrics {
            let lines = Self::parse_lrc(&synced);
            if !lines.is_empty() {
                return LyricsResult {
                    synced: true,
                    lines,
                    plain_text: data.plain_lyrics,
                    provider: "LRCLIB".to_string(),
                    has_romanized: false,
                };
            }
        }
        LyricsResult {
            synced: false,
            lines: Vec::new(),
            plain_text: data.plain_lyrics,
            provider: "LRCLIB".to_string(),
            has_romanized: false,
        }
    }

    async fn fetch_netease_raw(
        title: &str,
        artist: &str,
        duration: Option<f64>,
    ) -> Option<(String, Option<String>, Option<String>)> {
        let client = lyrics_client();
        let query = format!("{} {}", title, artist);
        let search_url = format!(
            "https://music.163.com/api/search/get/web?s={}&type=1&offset=0&total=true&limit=5",
            urlencoding::encode(&query)
        );

        let resp = client
            .post(&search_url)
            .header("Referer", "https://music.163.com/")
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let search_json: serde_json::Value = resp.json().await.ok()?;
        let songs = search_json.pointer("/result/songs")?.as_array()?;
        if songs.is_empty() {
            return None;
        }

        let target_ms = duration.unwrap_or(0.0) * 1000.0;
        let mut best_song_id = None;
        let mut best_delta = f64::MAX;

        for s in songs {
            if let Some(id) = s.get("id").and_then(|v| v.as_i64()) {
                let d_ms = s.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let delta = if target_ms > 0.0 && d_ms > 0.0 {
                    (d_ms - target_ms).abs()
                } else {
                    0.0
                };
                if target_ms > 15_000.0 && d_ms > 0.0 && delta > 15_000.0 {
                    continue;
                }
                if best_song_id.is_none() || delta < best_delta {
                    best_delta = delta;
                    best_song_id = Some(id);
                }
            }
        }

        let song_id = best_song_id?;
        let lyric_url = format!(
            "https://music.163.com/api/song/lyric?id={}&lv=-1&kv=-1&tv=-1&rv=-1",
            song_id
        );
        let lrc_resp = client
            .get(&lyric_url)
            .header("Referer", "https://music.163.com/")
            .send()
            .await
            .ok()?;
        let lrc_json: serde_json::Value = lrc_resp.json().await.ok()?;

        let raw_lrc = lrc_json
            .pointer("/lrc/lyric")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        let roma_lrc = lrc_json
            .pointer("/romalrc/lyric")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        let t_lyric = lrc_json
            .pointer("/tlyric/lyric")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        Some((raw_lrc, roma_lrc, t_lyric))
    }

    async fn fetch_netease(
        title: &str,
        artist: &str,
        duration: Option<f64>,
    ) -> Option<LyricsResult> {
        let (raw_lrc, roma_opt, trans_opt) =
            Self::fetch_netease_raw(title, artist, duration).await?;
        if raw_lrc.trim().is_empty() {
            return None;
        }

        let mut lines = Self::parse_lrc(&raw_lrc);
        if lines.is_empty() {
            return Some(LyricsResult {
                synced: false,
                lines: Vec::new(),
                plain_text: Some(raw_lrc),
                provider: "NetEase".to_string(),
                has_romanized: false,
            });
        }

        if let Some(roma_str) = roma_opt {
            let roma_lines = Self::parse_lrc(&roma_str);
            if !roma_lines.is_empty() {
                for line in &mut lines {
                    if let Some(closest) = roma_lines.iter().min_by(|a, b| {
                        (a.time - line.time)
                            .abs()
                            .partial_cmp(&(b.time - line.time).abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }) {
                        if (closest.time - line.time).abs() <= 1.0 && !closest.text.is_empty() {
                            line.romanized = Some(closest.text.clone());
                        }
                    }
                }
            }
        }

        if let Some(trans_str) = trans_opt {
            let trans_lines = Self::parse_lrc(&trans_str);
            if !trans_lines.is_empty() {
                for line in &mut lines {
                    if let Some(closest) = trans_lines.iter().min_by(|a, b| {
                        (a.time - line.time)
                            .abs()
                            .partial_cmp(&(b.time - line.time).abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }) {
                        if (closest.time - line.time).abs() <= 1.0 && !closest.text.is_empty() {
                            line.translation = Some(closest.text.clone());
                        }
                    }
                }
            }
        }

        let has_romanized = lines.iter().any(|l| l.romanized.is_some());
        Some(LyricsResult {
            synced: true,
            lines,
            plain_text: Some(raw_lrc),
            provider: "NetEase".to_string(),
            has_romanized,
        })
    }

    async fn fetch_kugou(
        title: &str,
        artist: &str,
        duration: Option<f64>,
    ) -> Option<LyricsResult> {
        let client = lyrics_client();
        let keyword = format!("{} - {}", artist, title);
        let dur_ms = duration.map(|d| (d * 1000.0) as i64).unwrap_or(0);

        let search_url = format!(
            "https://lyrics.kugou.com/search?ver=1&man=yes&client=pc&keyword={}&duration={}&hash=",
            urlencoding::encode(&keyword),
            if dur_ms > 0 { dur_ms } else { 200_000 }
        );

        let resp = client.get(&search_url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let json: serde_json::Value = resp.json().await.ok()?;
        let candidates = json.get("candidates")?.as_array()?;
        let first = candidates.first()?;
        let id = first.get("id")?.as_str()?;
        let accesskey = first.get("accesskey")?.as_str()?;

        let dl_url = format!(
            "https://lyrics.kugou.com/download?ver=1&client=pc&id={}&accesskey={}&fmt=lrc&charset=utf8",
            urlencoding::encode(id),
            urlencoding::encode(accesskey)
        );
        let dl_resp = client.get(&dl_url).send().await.ok()?;
        let dl_json: serde_json::Value = dl_resp.json().await.ok()?;
        let b64_content = dl_json.get("content")?.as_str()?;
        let decoded_bytes = Self::decode_base64(b64_content)?;
        let lrc_text = String::from_utf8(decoded_bytes).ok()?;

        let lines = Self::parse_lrc(&lrc_text);
        if lines.is_empty() {
            return None;
        }

        Some(LyricsResult {
            synced: true,
            lines,
            plain_text: Some(lrc_text),
            provider: "KuGou".to_string(),
            has_romanized: false,
        })
    }

    async fn fetch_simpmusic(video_id: &str) -> Option<LyricsResult> {
        if video_id.is_empty() {
            return None;
        }
        let client = lyrics_client();
        let url = format!(
            "https://api-lyrics.simpmusic.org/v1/{}",
            urlencoding::encode(video_id)
        );
        let resp = client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let json: serde_json::Value = resp.json().await.ok()?;
        let data = json.get("data").and_then(|d| d.as_array())?.first()?;

        let synced_str = data
            .get("syncedLyrics")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        let plain_str = data
            .get("plainLyrics")
            .and_then(|s| s.as_str())
            .map(String::from);

        if !synced_str.is_empty() {
            let lines = Self::parse_lrc(synced_str);
            if !lines.is_empty() {
                return Some(LyricsResult {
                    synced: true,
                    lines,
                    plain_text: plain_str,
                    provider: "SimpMusic".to_string(),
                    has_romanized: false,
                });
            }
        }

        if plain_str.is_some() {
            return Some(LyricsResult {
                synced: false,
                lines: Vec::new(),
                plain_text: plain_str,
                provider: "SimpMusic".to_string(),
                has_romanized: false,
            });
        }

        None
    }

    async fn fetch_lyrics_ovh(title: &str, artist: &str) -> Option<LyricsResult> {
        let client = lyrics_client();
        let url = format!(
            "https://api.lyrics.ovh/v1/{}/{}",
            urlencoding::encode(artist),
            urlencoding::encode(title)
        );
        let resp = client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let json: serde_json::Value = resp.json().await.ok()?;
        let lyrics = json.get("lyrics")?.as_str()?.trim();
        if lyrics.is_empty() {
            return None;
        }
        Some(LyricsResult {
            synced: false,
            lines: Vec::new(),
            plain_text: Some(lyrics.to_string()),
            provider: "Lyrics.ovh".to_string(),
            has_romanized: false,
        })
    }

    /// Enrich every `LyricLine` with automatic phonetic romanization if it contains non-Latin script
    pub fn enrich_romanization(mut result: LyricsResult) -> LyricsResult {
        let mut any_romanized = result.has_romanized;
        for line in &mut result.lines {
            if line.romanized.is_none() {
                if let Some(rom) = Romanizer::romanize_line(&line.text) {
                    line.romanized = Some(rom);
                    any_romanized = true;
                }
            } else {
                any_romanized = true;
            }
        }
        result.has_romanized = any_romanized;
        result
    }

    pub fn parse_lrc(lrc_content: &str) -> Vec<LyricLine> {
        let mut time_map: BTreeMap<i64, String> = BTreeMap::new();

        for raw_line in lrc_content.lines() {
            let mut rest = raw_line.trim();
            if !rest.starts_with('[') {
                continue;
            }
            let mut timestamps = Vec::new();
            while rest.starts_with('[') {
                if let Some(close_idx) = rest.find(']') {
                    let tag = &rest[1..close_idx];
                    if let Some(secs) = Self::parse_lrc_timestamp(tag) {
                        timestamps.push(secs);
                        rest = &rest[close_idx + 1..];
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            let clean_text = Self::strip_inline_time_tags(rest.trim());
            for t in timestamps {
                let key_ms = (t * 1000.0).round() as i64;
                if !clean_text.is_empty() || !time_map.contains_key(&key_ms) {
                    time_map.insert(key_ms, clean_text.clone());
                }
            }
        }

        time_map
            .into_iter()
            .filter(|(_, text)| !text.is_empty())
            .map(|(ms, text)| LyricLine {
                time: (ms as f64) / 1000.0,
                text,
                romanized: None,
                translation: None,
            })
            .collect()
    }

    fn strip_inline_time_tags(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut in_angle = false;
        for ch in s.chars() {
            match ch {
                '<' => in_angle = true,
                '>' if in_angle => in_angle = false,
                _ if !in_angle => out.push(ch),
                _ => {}
            }
        }
        out.trim().to_string()
    }

    fn parse_lrc_timestamp(time_part: &str) -> Option<f64> {
        let parts: Vec<&str> = time_part.split(':').collect();
        match parts.len() {
            2 => {
                let minutes = parts[0].trim().parse::<f64>().ok()?;
                let seconds = parts[1].trim().replace(',', ".").parse::<f64>().ok()?;
                Some(minutes * 60.0 + seconds)
            }
            3 => {
                let first = parts[0].trim().parse::<f64>().ok()?;
                let second = parts[1].trim().parse::<f64>().ok()?;
                let third = parts[2].trim().parse::<f64>().ok()?;
                Some(first * 60.0 + second + (third / 100.0))
            }
            _ => None,
        }
    }

    fn decode_base64(input: &str) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(input.len() * 3 / 4);
        let mut buf = 0u32;
        let mut bits = 0u8;
        for b in input.bytes() {
            let val = match b {
                b'A'..=b'Z' => b - b'A',
                b'a'..=b'z' => b - b'a' + 26,
                b'0'..=b'9' => b - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => break,
                b' ' | b'\n' | b'\r' | b'\t' => continue,
                _ => return None,
            };
            buf = (buf << 6) | (val as u32);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push(((buf >> bits) & 0xFF) as u8);
            }
        }
        Some(out)
    }
}

/// Built-in 12-Script Phonetic Romanizer (ArchiveTune / Apple Music style)
pub struct Romanizer;

impl Romanizer {
    pub fn has_non_latin_phonetic_script(text: &str) -> bool {
        text.chars().any(|c| {
            let u = c as u32;
            (0xAC00..=0xD7A3).contains(&u) // Hangul syllables
                || (0x3040..=0x30FF).contains(&u) // Hiragana + Katakana
                || (0x0900..=0x0D7F).contains(&u) // All 9 Brahmic/Indic scripts (Devanagari..Malayalam)
                || (0x0400..=0x04FF).contains(&u) // Cyrillic
        })
    }

    pub fn romanize_line(text: &str) -> Option<String> {
        if !Self::has_non_latin_phonetic_script(text) {
            return None;
        }

        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len() * 2);
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            let u = c as u32;

            // 1. Korean Hangul Syllable Decomposition (Revised Romanization)
            if (0xAC00..=0xD7A3).contains(&u) {
                let s_idx = u - 0xAC00;
                let l_idx = (s_idx / 588) as usize;
                let v_idx = ((s_idx % 588) / 28) as usize;
                let t_idx = (s_idx % 28) as usize;

                const CHOSEONG: [&str; 19] = [
                    "g", "kk", "n", "d", "tt", "r", "m", "b", "pp", "s", "ss", "", "j", "jj",
                    "ch", "k", "t", "p", "h",
                ];
                const JUNGSEONG: [&str; 21] = [
                    "a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo",
                    "u", "wo", "we", "wi", "yu", "eu", "ui", "i",
                ];
                const JONGSEONG: [&str; 28] = [
                    "", "k", "k", "k", "n", "n", "n", "t", "l", "k", "m", "p", "l", "l", "p", "l",
                    "m", "p", "p", "t", "t", "ng", "t", "t", "k", "t", "p", "t",
                ];

                out.push_str(CHOSEONG[l_idx]);
                out.push_str(JUNGSEONG[v_idx]);
                out.push_str(JONGSEONG[t_idx]);
                i += 1;
                continue;
            }

            // 2. Japanese Hiragana & Katakana (Hepburn Romanization)
            if (0x3040..=0x30FF).contains(&u) {
                let h_char = if (0x30A1..=0x30F6).contains(&u) {
                    char::from_u32(u - 0x60).unwrap_or(c)
                } else {
                    c
                };
                let next_h = chars.get(i + 1).map(|&nc| {
                    let nu = nc as u32;
                    if (0x30A1..=0x30F6).contains(&nu) {
                        char::from_u32(nu - 0x60).unwrap_or(nc)
                    } else {
                        nc
                    }
                });

                if h_char == 'っ' {
                    if let Some(nh) = next_h {
                        if let Some(rom) = Self::kana_base(nh) {
                            if let Some(first_c) = rom.chars().next() {
                                if first_c.is_ascii_alphabetic() {
                                    out.push(first_c);
                                }
                            }
                        }
                    }
                    i += 1;
                    continue;
                }

                if let Some(nh) = next_h {
                    if matches!(nh, 'ゃ' | 'ゅ' | 'ょ') {
                        if let Some(base) = Self::kana_base(h_char) {
                            if base.ends_with('i') && base.len() >= 2 {
                                let stem = &base[..base.len() - 1];
                                let glide = match nh {
                                    'ゃ' => "ya",
                                    'ゅ' => "yu",
                                    'ょ' => "yo",
                                    _ => "",
                                };
                                if stem == "sh" || stem == "ch" || stem == "j" {
                                    out.push_str(stem);
                                    out.push_str(&glide[1..]);
                                } else {
                                    out.push_str(stem);
                                    out.push_str(glide);
                                }
                                i += 2;
                                continue;
                            }
                        }
                    }
                }

                if let Some(rom) = Self::kana_base(h_char) {
                    out.push_str(rom);
                } else if c == 'ー' {
                    if let Some(last) = out.chars().last() {
                        if matches!(last, 'a' | 'i' | 'u' | 'e' | 'o') {
                            out.push(last);
                        }
                    }
                } else if !LyricsService::has_cjk_characters(&c.to_string()) {
                    out.push(c);
                }
                i += 1;
                continue;
            }

            // 3. Unified ISCII / Brahmic 9-Script Indic Transliteration (Devanagari, Bengali, Gurmukhi, Gujarati, Odia, Tamil, Telugu, Kannada, Malayalam)
            if (0x0900..=0x0D7F).contains(&u) {
                let is_dravidian = (0x0B80..=0x0D7F).contains(&u);
                let is_malayalam = (0x0D00..=0x0D7F).contains(&u);
                let is_tamil = (0x0B80..=0x0BFF).contains(&u);
                let offset_in_block = (u & 0x7F) as u8;

                if is_malayalam && (0x0D7A..=0x0D7F).contains(&u) {
                    let chillu = match u {
                        0x0D7A => "nn",
                        0x0D7B => "n",
                        0x0D7C => "r",
                        0x0D7D => "l",
                        0x0D7E => "ll",
                        0x0D7F => "k",
                        _ => "",
                    };
                    out.push_str(chillu);
                    i += 1;
                    continue;
                }

                let next_u = chars.get(i + 1).map(|&nc| nc as u32);
                let has_nukta = next_u
                    .map(|nu| (nu & 0x7F) == 0x3C && (0x0900..=0x0D7F).contains(&nu))
                    .unwrap_or(false);

                if let Some(cons) =
                    Self::indic_consonant(offset_in_block, has_nukta, is_malayalam, is_tamil)
                {
                    let step = if has_nukta { 2 } else { 1 };
                    let after_u = chars.get(i + step).map(|&nc| nc as u32);
                    let after_offset = after_u.and_then(|au| {
                        if (0x0900..=0x0D7F).contains(&au) {
                            Some((au & 0x7F) as u8)
                        } else {
                            None
                        }
                    });

                    if is_malayalam && after_offset == Some(0x4D) {
                        if let Some(&third_c) = chars.get(i + step + 1) {
                            let third_u = third_c as u32;
                            if u == 0x0D31 && third_u == 0x0D31 {
                                out.push_str("tt");
                                i += step + 1;
                                let fourth_u = chars.get(i + 1).map(|&nc| nc as u32);
                                let fourth_off = fourth_u.and_then(|fu| {
                                    if (0x0D00..=0x0D7F).contains(&fu) {
                                        Some((fu & 0x7F) as u8)
                                    } else {
                                        None
                                    }
                                });
                                if let Some(fo) = fourth_off {
                                    if fo == 0x4D {
                                        i += 2;
                                        continue;
                                    } else if let Some(matra) = Self::indic_matra(fo) {
                                        out.push_str(matra);
                                        i += 2;
                                        continue;
                                    }
                                }
                                out.push('a');
                                i += 1;
                                continue;
                            } else if u == 0x0D28 && third_u == 0x0D31 {
                                out.push_str("nd");
                                i += step + 1;
                                let fourth_u = chars.get(i + 1).map(|&nc| nc as u32);
                                let fourth_off = fourth_u.and_then(|fu| {
                                    if (0x0D00..=0x0D7F).contains(&fu) {
                                        Some((fu & 0x7F) as u8)
                                    } else {
                                        None
                                    }
                                });
                                if let Some(fo) = fourth_off {
                                    if fo == 0x4D {
                                        i += 2;
                                        continue;
                                    } else if let Some(matra) = Self::indic_matra(fo) {
                                        out.push_str(matra);
                                        i += 2;
                                        continue;
                                    }
                                }
                                out.push('a');
                                i += 1;
                                continue;
                            }
                        }
                    }

                    out.push_str(cons);

                    match after_offset {
                        Some(0x4D) => {
                            i += step + 1;
                            continue;
                        }
                        Some(mo) if Self::indic_matra(mo).is_some() => {
                            out.push_str(Self::indic_matra(mo).unwrap());
                            i += step + 1;
                            continue;
                        }
                        _ => {
                            let next_is_boundary = chars
                                .get(i + step)
                                .map(|nc| nc.is_whitespace() || nc.is_ascii_punctuation())
                                .unwrap_or(true);
                            if is_dravidian || !next_is_boundary {
                                out.push('a');
                            }
                            i += step;
                            continue;
                        }
                    }
                } else if let Some(vow) = Self::indic_vowel_or_sign(offset_in_block, is_malayalam) {
                    out.push_str(vow);
                    i += 1;
                    continue;
                }
            }

            // 4. Cyrillic Transliteration
            if (0x0400..=0x04FF).contains(&u) {
                if let Some(cyr) = Self::cyrillic_char(c) {
                    out.push_str(cyr);
                    i += 1;
                    continue;
                }
            }

            if !LyricsService::has_cjk_characters(&c.to_string()) {
                out.push(c);
            }
            i += 1;
        }

        let trimmed = out.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.is_empty() || trimmed == text.trim() {
            None
        } else {
            Some(trimmed)
        }
    }

    fn kana_base(c: char) -> Option<&'static str> {
        Some(match c {
            'あ' => "a", 'い' => "i", 'う' => "u", 'え' => "e", 'お' => "o",
            'か' => "ka", 'き' => "ki", 'く' => "ku", 'け' => "ke", 'こ' => "ko",
            'さ' => "sa", 'し' => "shi", 'す' => "su", 'せ' => "se", 'そ' => "so",
            'た' => "ta", 'ち' => "chi", 'つ' => "tsu", 'て' => "te", 'と' => "to",
            'な' => "na", 'に' => "ni", 'ぬ' => "nu", 'ね' => "ne", 'の' => "no",
            'は' => "ha", 'ひ' => "hi", 'ふ' => "fu", 'へ' => "he", 'ほ' => "ho",
            'ま' => "ma", 'み' => "mi", 'む' => "mu", 'め' => "me", 'も' => "mo",
            'や' => "ya", 'ゆ' => "yu", 'よ' => "yo",
            'ら' => "ra", 'り' => "ri", 'る' => "ru", 'れ' => "re", 'ろ' => "ro",
            'わ' => "wa", 'を' => "o", 'ん' => "n",
            'が' => "ga", 'ぎ' => "gi", 'ぐ' => "gu", 'げ' => "ge", 'ご' => "go",
            'ざ' => "za", 'じ' => "ji", 'ず' => "zu", 'ぜ' => "ze", 'ぞ' => "zo",
            'だ' => "da", 'ぢ' => "ji", 'づ' => "zu", 'で' => "de", 'ど' => "do",
            'ば' => "ba", 'び' => "bi", 'ぶ' => "bu", 'べ' => "be", 'ぼ' => "bo",
            'ぱ' => "pa", 'ぴ' => "pi", 'ぷ' => "pu", 'ぺ' => "pe", 'ぽ' => "po",
            'ぁ' => "a", 'ぃ' => "i", 'ぅ' => "u", 'ぇ' => "e", 'ぉ' => "o",
            _ => return None,
        })
    }

    fn indic_consonant(
        offset: u8,
        has_nukta: bool,
        is_malayalam: bool,
        is_tamil: bool,
    ) -> Option<&'static str> {
        if has_nukta {
            return Some(match offset {
                0x15 => "q",
                0x16 => "kh",
                0x17 => "gh",
                0x1C => "z",
                0x21 => "r",
                0x22 => "rh",
                0x2B => "f",
                _ => return Self::indic_consonant(offset, false, is_malayalam, is_tamil),
            });
        }
        Some(match offset {
            0x15 => "k",
            0x16 => "kh",
            0x17 => "g",
            0x18 => "gh",
            0x19 => "ng",
            0x1A => "ch",
            0x1B => "chh",
            0x1C => "j",
            0x1D => "jh",
            0x1E => "nj",
            0x1F => "t",
            0x20 => "th",
            0x21 => "d",
            0x22 => "dh",
            0x23 => "n",
            0x24 => "th",
            0x25 => "thh",
            0x26 => "d",
            0x27 => "dh",
            0x28 => "n",
            0x29 => "n",
            0x2A => "p",
            0x2B => "ph",
            0x2C => "b",
            0x2D => "bh",
            0x2E => "m",
            0x2F => "y",
            0x30 => "r",
            0x31 => "r",
            0x32 => "l",
            0x33 => "l",
            0x34 => {
                if is_malayalam || is_tamil {
                    "zh"
                } else {
                    "l"
                }
            }
            0x35 => "v",
            0x36 => "sh",
            0x37 => "sh",
            0x38 => "s",
            0x39 => "h",
            0x3A => "tt",
            0x58 => "q",
            0x59 => "kh",
            0x5A => "gh",
            0x5B => "z",
            0x5C => "r",
            0x5D => "rh",
            0x5E => "f",
            0x5F => "y",
            _ => return None,
        })
    }

    fn indic_matra(offset: u8) -> Option<&'static str> {
        Some(match offset {
            0x3E => "aa",
            0x3F => "i",
            0x40 => "ee",
            0x41 => "u",
            0x42 => "oo",
            0x43 | 0x44 => "ri",
            0x45 | 0x46 | 0x47 => "e",
            0x48 => "ai",
            0x49 | 0x4A | 0x4B => "o",
            0x4C | 0x57 => "au",
            _ => return None,
        })
    }

    fn indic_vowel_or_sign(offset: u8, is_malayalam: bool) -> Option<&'static str> {
        Some(match offset {
            0x01 | 0x02 => {
                if is_malayalam {
                    "m"
                } else {
                    "n"
                }
            }
            0x03 => "h",
            0x05 => "a",
            0x06 => "aa",
            0x07 => "i",
            0x08 => "ee",
            0x09 => "u",
            0x0A => "oo",
            0x0B | 0x60 => "ri",
            0x0D | 0x0E => "e",
            0x0F => "ee",
            0x10 => "ai",
            0x11 | 0x12 => "o",
            0x13 => "oo",
            0x14 => "au",
            0x64 | 0x65 => ".",
            _ => return None,
        })
    }

    fn cyrillic_char(c: char) -> Option<&'static str> {
        Some(match c {
            'а' | 'А' => "a", 'б' | 'Б' => "b", 'в' | 'В' => "v", 'г' | 'Г' => "g",
            'д' | 'Д' => "d", 'е' | 'Е' => "e", 'ё' | 'Ё' => "yo", 'ж' | 'Ж' => "zh",
            'з' | 'З' => "z", 'и' | 'И' => "i", 'й' | 'Й' => "y", 'к' | 'К' => "k",
            'л' | 'Л' => "l", 'м' | 'М' => "m", 'н' | 'Н' => "n", 'о' | 'О' => "o",
            'п' | 'П' => "p", 'р' | 'Р' => "r", 'с' | 'С' => "s", 'т' | 'Т' => "t",
            'у' | 'У' => "u", 'ф' | 'Ф' => "f", 'х' | 'Х' => "kh", 'ц' | 'Ц' => "ts",
            'ч' | 'Ч' => "ch", 'ш' | 'Ш' => "sh", 'щ' | 'Щ' => "shch", 'ы' | 'Ы' => "y",
            'э' | 'Э' => "e", 'ю' | 'Ю' => "yu", 'я' | 'Я' => "ya", 'ъ' | 'ь' => "",
            _ => return None,
        })
    }
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut result = String::new();
        for b in s.bytes() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(b as char);
                }
                _ => {
                    result.push_str(&format!("%{:02X}", b));
                }
            }
        }
        result
    }
}
