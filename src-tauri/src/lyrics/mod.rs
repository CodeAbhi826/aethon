use lofty::prelude::{ItemKey, TaggedFileExt};
use lofty::probe::Probe;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

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

fn lyrics_client() -> &'static reqwest::Client {
    LYRICS_HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_nodelay(true)
            .timeout(std::time::Duration::from_secs(5))
            .user_agent("Aethon/0.0.38 (https://github.com/codeabhi826/aethon)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
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
            .trim();
        if cleaned.is_empty() {
            raw.trim().to_string()
        } else {
            cleaned.to_string()
        }
    }

    /// ArchiveTune 7-Provider Lyrics Fallback Pipeline:
    /// Tier 0: Local .lrc Sidecar + Embedded Tags + Disk Cache (~/.cache/aethon/lyrics/)
    /// Tier 1: LRCLIB Exact + Search with ±12s duration validation
    /// Tier 2: NetEase Cloud Music (synced lrc + romalrc + tlyric)
    /// Tier 3: SimpMusic / YouLyPlus VideoID API
    /// Tier 4: KuGou Music Synced LRC
    /// Tier 5: YouTube Music Native InnerTube (timedLyricsData & musicDescriptionShelfRenderer)
    /// Tier 6: Lyrics.ovh Plain Text Fallback
    pub async fn fetch_lyrics(
        title: &str,
        artist: &str,
        duration: Option<f64>,
        file_path: Option<&str>,
        video_id: Option<&str>,
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
            .split(',')
            .next()
            .unwrap_or(artist)
            .trim()
            .to_string();

        let resolved_vid = video_id
            .map(|s| s.to_string())
            .or_else(|| file_path.and_then(crate::youtube::YouTubeService::extract_video_id));

        // Tier 0b: Check Disk Cache (~/.cache/aethon/lyrics/)
        let c_key = Self::disk_cache_key(&clean_title, &clean_artist, resolved_vid.as_deref());
        if let Some(cached) = Self::read_disk_cache(&c_key) {
            return Ok(Self::enrich_romanization(cached));
        }

        let mut plain_fallback: Option<LyricsResult> = None;

        // Concurrent Race of Primary Providers (Musixmatch, Apple Music, Spotify, YouTube Music)
        let mm_fut = Self::fetch_musixmatch(&clean_title, &clean_artist, duration);
        let am_fut = Self::fetch_apple_music(&clean_title, &clean_artist, duration);
        let sp_fut = Self::fetch_spotify(&clean_title, &clean_artist, duration);
        let ytm_fut = async {
            if let Some(ref vid) = resolved_vid {
                if !vid.is_empty() {
                    return Self::fetch_innertube_lyrics(vid).await;
                }
            }
            None
        };

        let (mm_res, am_res, sp_res, ytm_res) = tokio::join!(mm_fut, am_fut, sp_fut, ytm_fut);

        for candidate in [mm_res, am_res, sp_res, ytm_res] {
            if let Some(res) = candidate {
                if res.synced && !res.lines.is_empty() {
                    let enriched = Self::enrich_romanization(res);
                    Self::write_disk_cache(&c_key, &enriched);
                    return Ok(enriched);
                } else if plain_fallback.is_none() && res.plain_text.is_some() {
                    plain_fallback = Some(res);
                }
            }
        }

        // Concurrent Race of Secondary Providers (LRCLIB, NetEase, SimpMusic, KuGou, Lyrics.ovh)
        let lrclib_fut = Self::fetch_lrclib(&clean_title, &clean_artist, duration);
        let netease_fut = Self::fetch_netease(&clean_title, &clean_artist, duration);
        let simp_fut = async {
            if let Some(ref vid) = resolved_vid {
                if !vid.is_empty() {
                    return Self::fetch_simpmusic(vid).await;
                }
            }
            None
        };
        let kugou_fut = Self::fetch_kugou(&clean_title, &clean_artist, duration);
        let ovh_fut = Self::fetch_lyrics_ovh(&clean_title, &clean_artist);

        let (lrclib_res, netease_res, simp_res, kugou_res, ovh_res) =
            tokio::join!(lrclib_fut, netease_fut, simp_fut, kugou_fut, ovh_fut);

        if let Some(mut res) = lrclib_res {
            if res.synced && !res.lines.is_empty() {
                if res.lines.iter().any(|l| Self::has_cjk_characters(&l.text)) && !res.has_romanized {
                    Self::merge_netease_roma_trans_onto(&mut res.lines, &clean_title, &clean_artist, duration).await;
                    res.has_romanized = res.lines.iter().any(|l| l.romanized.is_some());
                }
                let enriched = Self::enrich_romanization(res);
                Self::write_disk_cache(&c_key, &enriched);
                return Ok(enriched);
            } else if plain_fallback.is_none() && res.plain_text.is_some() {
                plain_fallback = Some(res);
            }
        }

        for candidate in [netease_res, simp_res, kugou_res] {
            if let Some(res) = candidate {
                if res.synced && !res.lines.is_empty() {
                    let enriched = Self::enrich_romanization(res);
                    Self::write_disk_cache(&c_key, &enriched);
                    return Ok(enriched);
                } else if plain_fallback.is_none() && res.plain_text.is_some() {
                    plain_fallback = Some(res);
                }
            }
        }

        if let Some(res) = ovh_res {
            if plain_fallback.is_none() && res.plain_text.is_some() {
                plain_fallback = Some(res);
            }
        }

        if let Some(plain) = plain_fallback {
            let enriched = Self::enrich_romanization(plain);
            Self::write_disk_cache(&c_key, &enriched);
            return Ok(enriched);
        }

        Err("No lyrics found across concurrent multi-provider engine".into())
    }

    fn disk_cache_dir() -> std::path::PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("aethon")
            .join("lyrics")
    }

    fn disk_cache_key(title: &str, artist: &str, video_id: Option<&str>) -> String {
        if let Some(vid) = video_id.filter(|v| !v.is_empty()) {
            format!("v40_yt_{}", vid)
        } else {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            title.to_lowercase().hash(&mut hasher);
            artist.to_lowercase().hash(&mut hasher);
            format!("v40_{:016x}", hasher.finish())
        }
    }

    fn read_disk_cache(key: &str) -> Option<LyricsResult> {
        let path = Self::disk_cache_dir().join(format!("{}.json", key));
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(path) {
                if let Ok(mut res) = serde_json::from_str::<LyricsResult>(&data) {
                    res.provider = format!("Cached ({})", res.provider);
                    return Some(res);
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

        // Check sidecar .lrc file first
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

        // Check embedded audio tag (ID3 USLT/SYLT, Vorbis LYRICS)
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

    async fn fetch_lrclib(
        title: &str,
        artist: &str,
        duration: Option<f64>,
    ) -> Option<LyricsResult> {
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

        if let Some(target_d) = duration.filter(|&d| d > 0.0) {
            items.retain(|item| {
                item.duration
                    .map(|d| (d - target_d).abs() <= 12.0)
                    .unwrap_or(true)
            });
            if items.is_empty() {
                return None;
            }
        }

        // Sort candidates: prefer synced lyrics with closest duration match
        items.sort_by(|a, b| {
            let a_synced = a.synced_lyrics.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
            let b_synced = b.synced_lyrics.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
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

    async fn merge_netease_roma_trans_onto(
        lines: &mut [LyricLine],
        title: &str,
        artist: &str,
        duration: Option<f64>,
    ) {
        if let Some((_, Some(roma_str), trans_opt)) =
            Self::fetch_netease_raw(title, artist, duration).await
        {
            let roma_lines = Self::parse_lrc(&roma_str);
            if !roma_lines.is_empty() {
                for line in lines.iter_mut() {
                    if line.romanized.is_none() {
                        if let Some(closest) = roma_lines
                            .iter()
                            .min_by(|a, b| {
                                (a.time - line.time)
                                    .abs()
                                    .partial_cmp(&(b.time - line.time).abs())
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                        {
                            if (closest.time - line.time).abs() <= 1.2 && !closest.text.is_empty() {
                                line.romanized = Some(closest.text.clone());
                            }
                        }
                    }
                }
            }

            if let Some(t_str) = trans_opt {
                let trans_lines = Self::parse_lrc(&t_str);
                if !trans_lines.is_empty() {
                    for line in lines.iter_mut() {
                        if line.translation.is_none() {
                            if let Some(closest) = trans_lines
                                .iter()
                                .min_by(|a, b| {
                                    (a.time - line.time)
                                        .abs()
                                        .partial_cmp(&(b.time - line.time).abs())
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                })
                            {
                                if (closest.time - line.time).abs() <= 1.2 && !closest.text.is_empty() {
                                    line.translation = Some(closest.text.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
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
                    if let Some(closest) = roma_lines
                        .iter()
                        .min_by(|a, b| {
                            (a.time - line.time)
                                .abs()
                                .partial_cmp(&(b.time - line.time).abs())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                    {
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
                    if let Some(closest) = trans_lines
                        .iter()
                        .min_by(|a, b| {
                            (a.time - line.time)
                                .abs()
                                .partial_cmp(&(b.time - line.time).abs())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                    {
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
        let client = lyrics_client();
        let url = format!("https://api-lyrics.simpmusic.org/v1/{}", urlencoding::encode(video_id));
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

    async fn fetch_innertube_lyrics(video_id: &str) -> Option<LyricsResult> {
        let client = lyrics_client();
        let next_url = "https://music.youtube.com/youtubei/v1/next";
        let next_payload = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en"
                }
            },
            "videoId": video_id
        });

        let resp = client.post(next_url).json(&next_payload).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let next_json: serde_json::Value = resp.json().await.ok()?;

        let mut browse_id: Option<String> = None;
        if let Some(tabs) = next_json
            .pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer/tabs")
            .and_then(|t| t.as_array())
        {
            for tab in tabs {
                if let Some(b_id) = tab
                    .pointer("/tabRenderer/endpoint/browseEndpoint/browseId")
                    .and_then(|b| b.as_str())
                {
                    if b_id.starts_with("MPLYt_") {
                        browse_id = Some(b_id.to_string());
                        break;
                    }
                }
            }
        }

        let b_id = browse_id?;
        let browse_url = "https://music.youtube.com/youtubei/v1/browse";
        let browse_payload = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en"
                }
            },
            "browseId": b_id
        });

        let b_resp = client.post(browse_url).json(&browse_payload).send().await.ok()?;
        if !b_resp.status().is_success() {
            return None;
        }
        let b_json: serde_json::Value = b_resp.json().await.ok()?;

        let timed_arr_opt = b_json
            .pointer("/contents/sectionListRenderer/contents/0/musicDescriptionShelfRenderer/timedLyricsData")
            .or_else(|| b_json.pointer("/contents/elementRenderer/timedLyricsData"))
            .or_else(|| b_json.pointer("/contents/elementRenderer/newElement/timedLyricsModel/lyricsData/timedLyricsData"))
            .and_then(|a| a.as_array())
            .cloned()
            .or_else(|| {
                // Robust recursive search for "timedLyricsData" or "timedLyricsModel" in any sub-node
                Self::find_timed_lyrics_in_json(&b_json)
            });

        if let Some(timed_arr) = timed_arr_opt {
            let mut lines = Vec::new();
            for item in timed_arr {
                let text = item
                    .get("lyricLine")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let start_ms = item
                    .pointer("/cueRange/startTimeMilliseconds")
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .or_else(|| item.pointer("/cueRange/startTimeMilliseconds").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if !text.is_empty() {
                    lines.push(LyricLine {
                        time: start_ms / 1000.0,
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
                    provider: "YouTube Music".to_string(),
                    has_romanized: false,
                });
            }
        }

        if let Some(runs) = b_json
            .pointer("/contents/sectionListRenderer/contents/0/musicDescriptionShelfRenderer/description/runs")
            .and_then(|r| r.as_array())
        {
            let mut text = String::new();
            for run in runs {
                if let Some(t) = run.get("text").and_then(|s| s.as_str()) {
                    text.push_str(t);
                }
            }
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(LyricsResult {
                    synced: false,
                    lines: Vec::new(),
                    plain_text: Some(trimmed.to_string()),
                    provider: "YouTube Music".to_string(),
                    has_romanized: false,
                });
            }
        }

        None
    }

    fn find_timed_lyrics_in_json(val: &serde_json::Value) -> Option<Vec<serde_json::Value>> {
        match val {
            serde_json::Value::Object(map) => {
                if let Some(arr) = map.get("timedLyricsData").and_then(|v| v.as_array()) {
                    return Some(arr.clone());
                }
                for v in map.values() {
                    if let Some(res) = Self::find_timed_lyrics_in_json(v) {
                        return Some(res);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    if let Some(res) = Self::find_timed_lyrics_in_json(v) {
                        return Some(res);
                    }
                }
            }
            _ => {}
        }
        None
    }

    async fn fetch_musixmatch(title: &str, artist: &str, duration: Option<f64>) -> Option<LyricsResult> {
        let client = lyrics_client();
        let query = format!("{} {}", title, artist);
        // Musixmatch search + synchronized lyrics via public mobile token gateway
        let search_url = format!(
            "https://apic-desktop.musixmatch.com/ws/1.1/macro.subtitles.get?format=json&q_track={}&q_artist={}&user_language=en&app_id=web-desktop-app-v1.0",
            urlencoding::encode(title),
            urlencoding::encode(artist)
        );
        let resp = client
            .get(&search_url)
            .header("Cookie", "AWSELBCORS=0; AWSELB=0")
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let json: serde_json::Value = resp.json().await.ok()?;
        let body = json.pointer("/message/body/macro_result_list")?;
        
        // Check for subtitles
        if let Some(sub_body) = body.pointer("/subtitles_list/0/subtitle/subtitle_body").and_then(|s| s.as_str()) {
            if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(sub_body) {
                let mut lines = Vec::new();
                for item in items {
                    let text = item.get("text").and_then(|s| s.as_str()).unwrap_or("").trim().to_string();
                    let total_sec = item.pointer("/time/total").and_then(|t| t.as_f64()).unwrap_or(0.0);
                    if !text.is_empty() {
                        lines.push(LyricLine {
                            time: total_sec,
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
                        provider: "Musixmatch".to_string(),
                        has_romanized: false,
                    });
                }
            }
        }

        // Fallback to plain lyrics from Musixmatch
        if let Some(lyrics_body) = body.pointer("/lyrics_list/0/lyrics/lyrics_body").and_then(|s| s.as_str()) {
            let clean = lyrics_body.split("******* This Lyrics is NOT for Commercial use *******").next().unwrap_or(lyrics_body).trim();
            if !clean.is_empty() {
                return Some(LyricsResult {
                    synced: false,
                    lines: Vec::new(),
                    plain_text: Some(clean.to_string()),
                    provider: "Musixmatch".to_string(),
                    has_romanized: false,
                });
            }
        }

        None
    }

    async fn fetch_apple_music(title: &str, artist: &str, duration: Option<f64>) -> Option<LyricsResult> {
        let client = lyrics_client();
        // Apple Music synchronized TTML lyrics endpoint via public WebPlayer API
        let term = format!("{} {}", artist, title);
        let search_url = format!(
            "https://music.apple.com/api/v1/catalog/us/search?term={}&types=songs&limit=3",
            urlencoding::encode(&term)
        );
        let resp = client
            .get(&search_url)
            .header("Origin", "https://music.apple.com")
            .header("Referer", "https://music.apple.com/")
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let json: serde_json::Value = resp.json().await.ok()?;
        let songs = json.pointer("/results/songs/data")?.as_array()?;
        let first_song = songs.first()?;
        let song_id = first_song.get("id")?.as_str()?;

        // Query TTML/LRC syllables
        let lyric_url = format!("https://amp-api.music.apple.com/v1/catalog/us/songs/{}/lyrics", song_id);
        let l_resp = client
            .get(&lyric_url)
            .header("Origin", "https://music.apple.com")
            .send()
            .await
            .ok()?;
        if !l_resp.status().is_success() {
            return None;
        }
        let l_json: serde_json::Value = l_resp.json().await.ok()?;
        if let Some(ttml) = l_json.pointer("/data/0/attributes/ttml").and_then(|s| s.as_str()) {
            let lines = Self::parse_ttml_to_lines(ttml);
            if !lines.is_empty() {
                return Some(LyricsResult {
                    synced: true,
                    lines,
                    plain_text: None,
                    provider: "Apple Music".to_string(),
                    has_romanized: false,
                });
            }
        }

        None
    }

    fn parse_ttml_to_lines(ttml: &str) -> Vec<LyricLine> {
        let mut lines = Vec::new();
        for chunk in ttml.split("<p ") {
            if let Some(begin_idx) = chunk.find("begin=\"") {
                let rest = &chunk[begin_idx + 7..];
                if let Some(end_quote) = rest.find('"') {
                    let time_str = &rest[..end_quote];
                    let seconds = Self::parse_ttml_timestamp(time_str);
                    if let Some(text_start) = chunk.find('>') {
                        let text_part = &chunk[text_start + 1..];
                        if let Some(text_end) = text_part.find("</p>") {
                            let raw_text = &text_part[..text_end];
                            // Strip inner spans
                            let mut clean_text = String::new();
                            let mut in_tag = false;
                            for c in raw_text.chars() {
                                if c == '<' { in_tag = true; }
                                else if c == '>' { in_tag = false; }
                                else if !in_tag { clean_text.push(c); }
                            }
                            let trimmed = clean_text.trim();
                            if !trimmed.is_empty() {
                                lines.push(LyricLine {
                                    time: seconds,
                                    text: trimmed.to_string(),
                                    romanized: None,
                                    translation: None,
                                });
                            }
                        }
                    }
                }
            }
        }
        lines
    }

    fn parse_ttml_timestamp(ts: &str) -> f64 {
        let parts: Vec<&str> = ts.split(':').collect();
        if parts.len() == 2 {
            let m: f64 = parts[0].parse().unwrap_or(0.0);
            let s: f64 = parts[1].parse().unwrap_or(0.0);
            m * 60.0 + s
        } else if parts.len() == 3 {
            let h: f64 = parts[0].parse().unwrap_or(0.0);
            let m: f64 = parts[1].parse().unwrap_or(0.0);
            let s: f64 = parts[2].parse().unwrap_or(0.0);
            h * 3600.0 + m * 60.0 + s
        } else {
            ts.trim_end_matches('s').parse().unwrap_or(0.0)
        }
    }

    async fn fetch_spotify(title: &str, artist: &str, duration: Option<f64>) -> Option<LyricsResult> {
        let client = lyrics_client();
        // Spotify Musixmatch partner sync proxy endpoint
        let q = format!("{} {}", title, artist);
        let url = format!(
            "https://spotify-lyric-api.herokuapp.com/?track_name={}&artist_name={}",
            urlencoding::encode(title),
            urlencoding::encode(artist)
        );
        let resp = client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let json: serde_json::Value = resp.json().await.ok()?;
        if let Some(lines_arr) = json.get("lines").and_then(|l| l.as_array()) {
            let mut lines = Vec::new();
            for item in lines_arr {
                let words = item.get("words").and_then(|w| w.as_str()).unwrap_or("").trim().to_string();
                let start_ms = item.get("startTimeMs")
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .or_else(|| item.get("startTimeMs").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if !words.is_empty() && words != "♪" {
                    lines.push(LyricLine {
                        time: start_ms / 1000.0,
                        text: words,
                        romanized: None,
                        translation: None,
                    });
                }
            }
            if !lines.is_empty() {
                let sync_type = json.get("syncType").and_then(|s| s.as_str()).unwrap_or("LINE_SYNCED");
                return Some(LyricsResult {
                    synced: sync_type == "LINE_SYNCED" || sync_type == "SYLLABLE_SYNCED",
                    lines,
                    plain_text: None,
                    provider: "Spotify".to_string(),
                    has_romanized: false,
                });
            }
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
        let text = json.get("lyrics")?.as_str()?.trim();
        if text.is_empty() {
            return None;
        }
        Some(LyricsResult {
            synced: false,
            lines: Vec::new(),
            plain_text: Some(text.to_string()),
            provider: "Lyrics.ovh".to_string(),
            has_romanized: false,
        })
    }

    /// Enrich every `LyricLine` with automatic phonetic romanization if it contains non-Latin script
    /// (Hangul, Hiragana, Katakana, Devanagari/Hindi, Cyrillic) and does not already have a romanized line.
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
            // Strip inline word-level `<00:12.34>` enhanced LRC tags if present
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

/// Built-in Multi-Script Phonetic Romanizer (ArchiveTune / Apple Music style)
/// Supports Korean Hangul, Japanese Hiragana & Katakana, Hindi/Devanagari, and Cyrillic.
pub struct Romanizer;

impl Romanizer {
    pub fn has_non_latin_phonetic_script(text: &str) -> bool {
        text.chars().any(|c| {
            let u = c as u32;
            (0xAC00..=0xD7A3).contains(&u) // Hangul syllables
                || (0x3040..=0x30FF).contains(&u) // Hiragana + Katakana
                || (0x0900..=0x0D7F).contains(&u) // Indic scripts (Devanagari, Bengali, Gurmukhi, Gujarati, Odia, Tamil, Telugu, Kannada, Malayalam)
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

                // Sokuon (っ) doubles the next consonant
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

                // Digraphs (きゃ, しゃ, ちゃ, etc.)
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
                } else {
                    out.push(c);
                }
                i += 1;
                continue;
            }

            // 3. 10-Script Indic Phonetic Transliteration (Devanagari, Malayalam, Tamil, Telugu, Kannada, Bengali, Gujarati, Gurmukhi, Odia)
            if (0x0900..=0x0D7F).contains(&u) {
                // Malayalam Chillu Letters (0x0D7A..=0x0D7F)
                if (0x0D7A..=0x0D7F).contains(&u) {
                    let chillu = match u {
                        0x0D7A => "nn", // ൺ
                        0x0D7B => "n",  // ൻ
                        0x0D7C => "r",  // ർ
                        0x0D7D => "l",  // ൽ
                        0x0D7E => "ll", // ൾ
                        0x0D7F => "k",  // ൿ
                        _ => "",
                    };
                    out.push_str(chillu);
                    i += 1;
                    continue;
                }

                // Malayalam Special Clusters: റ്റ (tta) & ന്റ (nda)
                let cluster_cons = if c == 'റ'
                    && chars.get(i + 1) == Some(&'്')
                    && chars.get(i + 2) == Some(&'റ')
                {
                    Some("tt")
                } else if c == 'ന'
                    && chars.get(i + 1) == Some(&'്')
                    && chars.get(i + 2) == Some(&'റ')
                {
                    Some("nd")
                } else {
                    None
                };

                if let Some(cons) = cluster_cons {
                    out.push_str(cons);
                    let next_c = chars.get(i + 3).copied();
                    if let Some(nc) = next_c {
                        let nu = nc as u32;
                        let n_offset = (nu & 0x7F) as u8;
                        let n_block = nu & 0xFF80;
                        if n_offset == 0x4D {
                            // Chandrakkala / Halant suppresses inherent 'a'
                            i += 4;
                            continue;
                        } else if let Some(matra) = Self::indic_matra(n_offset, n_block) {
                            out.push_str(matra);
                            i += 4;
                            continue;
                        } else if nc.is_whitespace() || nc.is_ascii_punctuation() {
                            // Word-final schwa deletion
                            i += 3;
                            continue;
                        }
                    } else {
                        // End of string: schwa deletion
                        i += 3;
                        continue;
                    }
                    // Inherent vowel 'a'
                    out.push('a');
                    i += 3;
                    continue;
                }

                let block = u & 0xFF80;
                let offset = (u & 0x7F) as u8;

                // Devanagari Nukta handling
                let has_nukta = block == 0x0900 && chars.get(i + 1).copied() == Some('\u{093c}');
                let cons_opt = if has_nukta {
                    match c {
                        'क' => Some("q"),
                        'ख' => Some("kh"),
                        'ग' => Some("gh"),
                        'ज' => Some("z"),
                        'ड' => Some("r"),
                        'ढ' => Some("rh"),
                        'फ' => Some("f"),
                        _ => Self::indic_consonant(offset, block),
                    }
                } else {
                    Self::indic_consonant(offset, block)
                };

                if let Some(cons) = cons_opt {
                    let step = if has_nukta { 2 } else { 1 };
                    out.push_str(cons);
                    let next_c = chars.get(i + step).copied();
                    if let Some(nc) = next_c {
                        let nu = nc as u32;
                        let n_offset = (nu & 0x7F) as u8;
                        if n_offset == 0x4D {
                            // Virama / Halant / Chandrakkala: suppresses inherent 'a'
                            i += step + 1;
                            continue;
                        } else if let Some(matra) = Self::indic_matra(n_offset, block) {
                            out.push_str(matra);
                            i += step + 1;
                            continue;
                        } else if nc.is_whitespace() || nc.is_ascii_punctuation() {
                            // Word-final schwa deletion in modern Indic languages
                            i += step;
                            continue;
                        }
                    } else {
                        // End of string: schwa deletion
                        i += step;
                        continue;
                    }

                    // Inherent vowel 'a'
                    out.push('a');
                    i += step;
                    continue;
                } else if let Some(vow) = Self::indic_vowel(offset, block) {
                    out.push_str(vow);
                    i += 1;
                    continue;
                }
            }

            // 4. Cyrillic Transliteration (0x0400..=0x04FF)
            if (0x0400..=0x04FF).contains(&u) {
                if let Some(cyr) = Self::cyrillic_char(c) {
                    out.push_str(cyr);
                    i += 1;
                    continue;
                }
            }

            out.push(c);
            i += 1;
        }

        let trimmed = out.trim().to_string();
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

    fn indic_consonant(offset: u8, block: u32) -> Option<&'static str> {
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
            0x1E => "ny",
            0x1F => "t",
            0x20 => "th",
            0x21 => "d",
            0x22 => "dh",
            0x23 => if block == 0x0B80 || block == 0x0D00 { "nn" } else { "n" },
            0x24 => "t",
            0x25 => "th",
            0x26 => "d",
            0x27 => "dh",
            0x28 => "n",
            0x29 => if block == 0x0B80 { "n" } else { "nn" }, // Tamil ன (alveolar nasal)
            0x2A => "p",
            0x2B => "ph",
            0x2C => "b",
            0x2D => "bh",
            0x2E => "m",
            0x2F => "y",
            0x30 => "r",
            0x31 => "rr", // Tamil/Malayalam ற/റ
            0x32 => "l",
            0x33 => "ll", // Tamil/Malayalam/Kannada/Telugu ள/ള/ಳ/ళ
            0x34 => "zh", // Tamil/Malayalam ழ/ഴ (retroflex approximant)
            0x35 => "v",
            0x36 => "sh",
            0x37 => "sh",
            0x38 => "s",
            0x39 => "h",
            0x58 => "q",
            0x59 => "kh",
            0x5A => "gh",
            0x5B => "z",
            0x5C => "r",
            0x5D => "rh",
            0x5E => "f",
            _ => return None,
        })
    }

    fn indic_matra(offset: u8, _block: u32) -> Option<&'static str> {
        Some(match offset {
            0x3E => "aa",
            0x3F => "i",
            0x40 => "ee",
            0x41 => "u",
            0x42 => "oo",
            0x43 => "ri",
            0x46 => "e",  // Short e (Tamil/Telugu/Kannada/Malayalam)
            0x47 => "ee", // Long e
            0x48 => "ai",
            0x4A => "o",  // Short o (Tamil/Telugu/Kannada/Malayalam)
            0x4B => "oo", // Long o
            0x4C => "au",
            0x4F => "au",
            _ => return None,
        })
    }

    fn indic_vowel(offset: u8, _block: u32) -> Option<&'static str> {
        Some(match offset {
            0x05 => "a",
            0x06 => "aa",
            0x07 => "i",
            0x08 => "ee",
            0x09 => "u",
            0x0A => "oo",
            0x0B => "ri",
            0x0E => "e",
            0x0F => "ee",
            0x10 => "ai",
            0x12 => "o",
            0x13 => "oo",
            0x14 => "au",
            0x01 => "n", // Chandrabindu (Nasalization)
            0x02 => "m", // Anusvara (Am / M)
            0x03 => "h", // Visarga
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

