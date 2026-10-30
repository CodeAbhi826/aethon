use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricLine {
    pub time: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsResult {
    pub synced: bool,
    pub lines: Vec<LyricLine>,
    pub plain_text: Option<String>,
}

#[derive(Deserialize)]
struct LrcLibResponse {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

pub struct LyricsService;

impl LyricsService {
    pub async fn fetch_lyrics(
        title: &str,
        artist: &str,
        duration: Option<f64>,
    ) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

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

        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            // Try search fallback
            let search_url = format!(
                "https://lrclib.net/api/search?q={}+{}",
                urlencoding::encode(title),
                urlencoding::encode(artist)
            );
            let search_resp = client.get(&search_url).send().await?;
            if search_resp.status().is_success() {
                let items: Vec<LrcLibResponse> = search_resp.json().await.unwrap_or_default();
                if let Some(first) = items.into_iter().next() {
                    return Ok(Self::parse_response(first));
                }
            }
            return Err("Lyrics not found".into());
        }

        let lrc_data: LrcLibResponse = resp.json().await?;
        Ok(Self::parse_response(lrc_data))
    }

    fn parse_response(data: LrcLibResponse) -> LyricsResult {
        if let Some(synced) = data.synced_lyrics {
            let lines = Self::parse_lrc(&synced);
            if !lines.is_empty() {
                return LyricsResult {
                    synced: true,
                    lines,
                    plain_text: data.plain_lyrics,
                };
            }
        }

        LyricsResult {
            synced: false,
            lines: Vec::new(),
            plain_text: data.plain_lyrics,
        }
    }

    fn parse_lrc(lrc_content: &str) -> Vec<LyricLine> {
        let mut lines = Vec::new();
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
            let text_part = rest.trim().to_string();
            for t in timestamps {
                lines.push(LyricLine {
                    time: t,
                    text: text_part.clone(),
                });
            }
        }
        lines.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        lines
    }

    fn parse_lrc_timestamp(time_part: &str) -> Option<f64> {
        let parts: Vec<&str> = time_part.split(':').collect();
        match parts.len() {
            2 => {
                let minutes = parts[0].parse::<f64>().ok()?;
                let seconds = parts[1].replace(',', ".").parse::<f64>().ok()?;
                Some(minutes * 60.0 + seconds)
            }
            3 => {
                let first = parts[0].parse::<f64>().ok()?;
                let second = parts[1].parse::<f64>().ok()?;
                let third = parts[2].parse::<f64>().ok()?;
                // Handle mm:ss:xx (centiseconds)
                Some(first * 60.0 + second + (third / 100.0))
            }
            _ => None,
        }
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
