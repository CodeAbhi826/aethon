use lofty::prelude::{Accessor, ItemKey, TaggedFileExt};
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
            .user_agent("Aethon/0.0.35 (https://github.com/codeabhi826/aethon)")
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

    /// ArchiveTune-style 5-provider fallback pipeline:
    /// 1. Local `.lrc` sidecar file & embedded ID3/Vorbis `Lyrics` tag
    /// 2. LRCLIB Exact + Duration-Scored Fuzzy Search (`lrclib.net`)
    /// 3. KuGou Synced LRC (`mobileservice.kugou.com` -> `lyrics.kugou.com`, Base64 LRC)
    /// 4. NetEase Cloud Music (`music.163.com` synced `lrc` + `romalrc` + `tlyric`)
    /// 5. SimpMusic / YouLyPlus VideoID API (`api-lyrics.simpmusic.org`)
    pub async fn fetch_lyrics(
        title: &str,
        artist: &str,
        duration: Option<f64>,
        file_path: Option<&str>,
        video_id: Option<&str>,
    ) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
        // 1. Check local .lrc sidecar or embedded audio tags first
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

        let mut plain_fallback: Option<LyricsResult> = None;

        // 2. Provider: LRCLIB (Exact + Search with duration scoring)
        for (q_title, q_artist) in [
            (title, artist),
            (clean_title.as_str(), clean_artist.as_str()),
        ] {
            if q_title.is_empty() {
                continue;
            }
            if let Some(res) = Self::fetch_lrclib(q_title, q_artist, duration).await {
                if res.synced && !res.lines.is_empty() {
                    return Ok(Self::enrich_romanization(res));
                } else if plain_fallback.is_none() && res.plain_text.is_some() {
                    plain_fallback = Some(res);
                }
            }
        }

        // 3. Provider: NetEase Cloud Music (Includes official `romalrc` romanized timestamps + `lrc`)
        if let Some(res) =
            Self::fetch_netease(&clean_title, &clean_artist, duration).await
        {
            if res.synced && !res.lines.is_empty() {
                return Ok(Self::enrich_romanization(res));
            } else if plain_fallback.is_none() && res.plain_text.is_some() {
                plain_fallback = Some(res);
            }
        }

        // 4. Provider: KuGou Synced LRC (ArchiveTune KuGou provider)
        if let Some(res) =
            Self::fetch_kugou(&clean_title, &clean_artist, duration).await
        {
            if res.synced && !res.lines.is_empty() {
                return Ok(Self::enrich_romanization(res));
            } else if plain_fallback.is_none() && res.plain_text.is_some() {
                plain_fallback = Some(res);
            }
        }

        // 5. Provider: SimpMusic VideoID API (if YouTube video_id or watch URL is known)
        let resolved_vid = video_id
            .map(|s| s.to_string())
            .or_else(|| file_path.and_then(crate::youtube::YouTubeService::extract_video_id));
        if let Some(ref vid) = resolved_vid {
            if !vid.is_empty() {
                if let Some(res) = Self::fetch_simpmusic(vid).await {
                    if res.synced && !res.lines.is_empty() {
                        return Ok(Self::enrich_romanization(res));
                    } else if plain_fallback.is_none() && res.plain_text.is_some() {
                        plain_fallback = Some(res);
                    }
                }
            }
        }

        if let Some(plain) = plain_fallback {
            return Ok(Self::enrich_romanization(plain));
        }

        Err("No lyrics found across Local, LRCLIB, NetEase, KuGou, or SimpMusic".into())
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

    async fn fetch_netease(
        title: &str,
        artist: &str,
        duration: Option<f64>,
    ) -> Option<LyricsResult> {
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
            .unwrap_or("");
        if raw_lrc.trim().is_empty() {
            return None;
        }

        let mut lines = Self::parse_lrc(raw_lrc);
        if lines.is_empty() {
            return Some(LyricsResult {
                synced: false,
                lines: Vec::new(),
                plain_text: Some(raw_lrc.to_string()),
                provider: "NetEase".to_string(),
                has_romanized: false,
            });
        }

        // Align official NetEase `romalrc` (romanized lyrics) if present
        if let Some(roma_str) = lrc_json.pointer("/romalrc/lyric").and_then(|s| s.as_str()) {
            let roma_lines = Self::parse_lrc(roma_str);
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
                        if (closest.time - line.time).abs() <= 0.8 && !closest.text.is_empty() {
                            line.romanized = Some(closest.text.clone());
                        }
                    }
                }
            }
        }

        // Align NetEase `tlyric` (translated lyrics) if present
        if let Some(trans_str) = lrc_json.pointer("/tlyric/lyric").and_then(|s| s.as_str()) {
            let trans_lines = Self::parse_lrc(trans_str);
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
                        if (closest.time - line.time).abs() <= 0.8 && !closest.text.is_empty() {
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
            plain_text: Some(raw_lrc.to_string()),
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
                || (0x0900..=0x097F).contains(&u) // Devanagari (Hindi/Marathi/Sanskrit)
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
                // Normalize Katakana to Hiragana offset where possible
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

                // Digraphs (きゃ, しゃ, ちゃ, にゃ, ひゃ, みゃ, りゃ, ぎゃ, じゃ, びゃ, ぴゃ)
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
                    if h_char == 'ん' || !out.ends_with(' ') {
                        // Add subtle word spacing before particles
                    }
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

            // 3. Devanagari / Hindi Phonetic Transliteration
            if (0x0900..=0x097F).contains(&u) {
                let has_nukta = chars.get(i + 1).copied() == Some('\u{093c}');
                let cons_opt = if has_nukta {
                    match c {
                        'क' => Some("q"),
                        'ख' => Some("kh"),
                        'ग' => Some("gh"),
                        'ज' => Some("z"),
                        'ड' => Some("r"),
                        'ढ' => Some("rh"),
                        'फ' => Some("f"),
                        _ => Self::devanagari_consonant(c),
                    }
                } else {
                    Self::devanagari_consonant(c)
                };

                if let Some(cons) = cons_opt {
                    let offset = if has_nukta { 2 } else { 1 };
                    out.push_str(cons);
                    let next_c = chars.get(i + offset).copied();
                    match next_c {
                        Some('्') => {
                            // Virama / Halant suppresses inherent 'a'
                            i += offset + 1;
                            continue;
                        }
                        Some(nc) if Self::devanagari_matra(nc).is_some() => {
                            out.push_str(Self::devanagari_matra(nc).unwrap());
                            i += offset + 1;
                            continue;
                        }
                        Some(nc) if nc.is_whitespace() || nc.is_ascii_punctuation() => {
                            // Word-final schwa deletion in modern Hindi
                            i += offset;
                            continue;
                        }
                        None => {
                            i += offset;
                            continue;
                        }
                        _ => {
                            out.push('a');
                            i += offset;
                            continue;
                        }
                    }
                } else if let Some(vow) = Self::devanagari_vowel_or_sign(c) {
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

    fn devanagari_consonant(c: char) -> Option<&'static str> {
        Some(match c {
            'क' => "k", 'ख' => "kh", 'ग' => "g", 'घ' => "gh", 'ङ' => "ng",
            'च' => "ch", 'छ' => "chh", 'ज' => "j", 'झ' => "jh", 'ञ' => "ny",
            'ट' => "t", 'ठ' => "th", 'ड' => "d", 'ढ' => "dh", 'ण' => "n",
            'त' => "t", 'थ' => "th", 'द' => "d", 'ध' => "dh", 'न' => "n",
            'प' => "p", 'फ' => "ph", 'ब' => "b", 'भ' => "bh", 'म' => "m",
            'य' => "y", 'र' => "r", 'ल' => "l", 'व' => "v",
            'श' => "sh", 'ष' => "sh", 'स' => "s", 'ह' => "h",
            'ळ' => "l",
            '\u{0958}' => "q",
            '\u{0959}' => "kh",
            '\u{095a}' => "gh",
            '\u{095b}' => "z",
            '\u{095c}' => "r",
            '\u{095d}' => "rh",
            '\u{095e}' => "f",
            _ => return None,
        })
    }

    fn devanagari_matra(c: char) -> Option<&'static str> {
        Some(match c {
            'ा' => "aa", 'ि' => "i", 'ी' => "ee", 'ु' => "u", 'ू' => "oo",
            'ृ' => "ri", 'े' => "e", 'ै' => "ai", 'ो' => "o", 'ौ' => "au",
            'ॉ' => "o", 'ॅ' => "e",
            _ => return None,
        })
    }

    fn devanagari_vowel_or_sign(c: char) -> Option<&'static str> {
        Some(match c {
            'अ' => "a", 'आ' => "aa", 'इ' => "i", 'ई' => "ee", 'उ' => "u", 'ऊ' => "oo",
            'ऋ' => "ri", 'ए' => "e", 'ऐ' => "ai", 'ओ' => "o", 'औ' => "au",
            'ं' | 'ँ' => "n", 'ः' => "h", '।' => ".",
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

