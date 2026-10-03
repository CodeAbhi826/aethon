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
        for line in lrc_content.lines() {
            let line = line.trim();
            if !line.starts_with('[') {
                continue;
            }

            if let Some(close_bracket) = line.find(']') {
                let time_part = &line[1..close_bracket];
                let text_part = line[close_bracket + 1..].trim();

                // Format: mm:ss.xx or mm:ss:xx
                let parts: Vec<&str> = time_part.split(':').collect();
                if parts.len() == 2 {
                    if let (Ok(minutes), Ok(seconds)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        let total_secs = minutes * 60.0 + seconds;
                        lines.push(LyricLine {
                            time: total_secs,
                            text: text_part.to_string(),
                        });
                    }
                }
            }
        }
        lines.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        lines
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
