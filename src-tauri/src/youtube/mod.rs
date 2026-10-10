use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration: f64,
    pub thumbnail: String,
    pub stream_url: String,
}

pub struct YouTubeService;

impl YouTubeService {
    pub async fn search(query: &str, limit: usize) -> Vec<YouTubeTrack> {
        let max_results = limit.clamp(1, 50);

        // 1. Direct YouTube Music InnerTube API (matching ArchiveTune with India geo "IN")
        if let Some(tracks) = Self::search_innertube(query, max_results, "IN").await {
            if !tracks.is_empty() {
                crate::aethon_log!("InnerTube API returned {} tracks for query '{}'", tracks.len(), query);
                return tracks;
            }
        }

        // 2. Fallback: yt-dlp CLI
        crate::aethon_log!("InnerTube returned 0 tracks, falling back to yt-dlp for query '{}'", query);
        Self::search_ytdlp(query, max_results)
    }

    pub async fn search_innertube(query: &str, limit: usize, gl: &str) -> Option<Vec<YouTubeTrack>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .ok()?;

        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "gl": gl,
                    "hl": "en-GB"
                }
            },
            "query": query
        });

        let resp = client
            .post("https://music.youtube.com/youtubei/v1/search")
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0")
            .json(&body)
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let json_data: serde_json::Value = resp.json().await.ok()?;
        let mut results = Vec::new();
        Self::extract_innertube_tracks(&json_data, &mut results, limit);

        Some(results)
    }

    fn is_metadata_noise_run(text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed == "•" || trimmed == "·" || trimmed == "|" || trimmed == "-" {
            return true;
        }
        let lower = trimmed.to_lowercase();
        if lower == "song"
            || lower == "video"
            || lower == "artist"
            || lower == "album"
            || lower == "single"
            || lower == "ep"
            || lower == "station"
            || lower == "playlist"
            || lower == "chart"
            || lower == "episode"
            || lower == "podcast"
        {
            return true;
        }
        // Duration pattern: e.g. "3:45", "1:02:15"
        if trimmed.contains(':') && trimmed.chars().all(|c| c.is_ascii_digit() || c == ':') {
            return true;
        }
        // 4-digit release year: e.g. "2018", "1999"
        if trimmed.len() == 4 && trimmed.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(yr) = trimmed.parse::<u32>() {
                if (1900..=2099).contains(&yr) {
                    return true;
                }
            }
        }
        // Solitary metric or metadata words
        if lower == "views"
            || lower == "view"
            || lower == "subscribers"
            || lower == "subscriber"
            || lower == "plays"
            || lower == "play"
            || lower == "audience"
            || lower == "songs"
            || lower == "tracks"
        {
            return true;
        }
        // View count, subscriber count, or play count: e.g. "3.8M views", "319M subscribers", "12M plays"
        let is_metric_suffix = lower.ends_with("views")
            || lower.ends_with("view")
            || lower.ends_with("subscribers")
            || lower.ends_with("subscriber")
            || lower.ends_with("plays")
            || lower.ends_with("play")
            || lower.ends_with("audience")
            || lower.ends_with("monthly audience")
            || lower.ends_with("songs")
            || lower.ends_with("tracks");
        if is_metric_suffix && trimmed.chars().any(|c| c.is_ascii_digit()) {
            return true;
        }
        false
    }

    fn strip_artist_metadata_suffixes(raw: &str) -> String {
        let mut cleaned = raw.trim();
        if cleaned.to_lowercase().starts_with("similar to ") {
            cleaned = cleaned[11..].trim();
        }

        let suffixes = [
            " views",
            " view",
            " subscribers",
            " subscriber",
            " plays",
            " play",
            " monthly audience",
            " audience",
            " songs",
            " tracks",
        ];

        let mut changed = true;
        while changed {
            changed = false;
            let lower = cleaned.to_lowercase();
            for suffix in &suffixes {
                if lower.ends_with(suffix) {
                    let prefix_len = lower.len() - suffix.len();
                    let prefix = cleaned[..prefix_len].trim_end();
                    if let Some(last_space) = prefix.rfind(' ') {
                        let count_part = &prefix[last_space + 1..];
                        if count_part.chars().any(|c| c.is_ascii_digit()) {
                            cleaned = prefix[..last_space].trim_end();
                            changed = true;
                            break;
                        } else {
                            cleaned = prefix;
                            changed = true;
                            break;
                        }
                    } else {
                        cleaned = prefix;
                        changed = true;
                        break;
                    }
                }
            }
            cleaned = cleaned.trim_end_matches(['•', '·', '|', '-', ',', ' ']).trim();
        }
        cleaned.to_string()
    }

    fn extract_innertube_tracks(
        value: &serde_json::Value,
        results: &mut Vec<YouTubeTrack>,
        limit: usize,
    ) {
        if results.len() >= limit {
            return;
        }

        if let Some(map) = value.as_object() {
            // 1. Top Hit Hero Card (musicCardShelfRenderer)
            if let Some(card) = map.get("musicCardShelfRenderer") {
                let card_title = card
                    .pointer("/title/runs")
                    .and_then(|r| r.as_array())
                    .and_then(|a| a.first())
                    .and_then(|o| o.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                let card_vid = card
                    .pointer("/title/runs")
                    .and_then(|r| r.as_array())
                    .and_then(|a| a.first())
                    .and_then(|o| o.pointer("/navigationEndpoint/watchEndpoint/videoId"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let mut card_artist = String::new();
                if let Some(sub_runs) = card.pointer("/subtitle/runs").and_then(|r| r.as_array()) {
                    let mut parts = Vec::new();
                    for r in sub_runs {
                        if let Some(t) = r.get("text").and_then(|s| s.as_str()) {
                            if !Self::is_metadata_noise_run(t) {
                                parts.push(t.trim());
                            }
                        }
                    }
                    card_artist = Self::strip_artist_metadata_suffixes(&parts.join(" "));
                }

                if !card_vid.is_empty() && !card_title.is_empty() && !results.iter().any(|r| r.id == card_vid) {
                    let thumb = card
                        .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
                        .and_then(|th| th.as_array())
                        .and_then(|arr| arr.last())
                        .and_then(|t| t.get("url"))
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", card_vid));

                    results.push(YouTubeTrack {
                        id: card_vid.to_string(),
                        title: card_title.to_string(),
                        artist: card_artist,
                        duration: 0.0,
                        thumbnail: thumb,
                        stream_url: format!("https://www.youtube.com/watch?v={}", card_vid),
                    });
                }
            }

            // 2. Standard Search List Item (musicResponsiveListItemRenderer)
            if let Some(item) = map.get("musicResponsiveListItemRenderer") {
                let cols = item.get("flexColumns").and_then(|c| c.as_array());

                let mut title = String::new();
                let mut artist = String::new();
                let mut video_id = item
                    .get("playlistItemData")
                    .and_then(|p| p.get("videoId"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(col_arr) = cols {
                    if let Some(first_col) = col_arr.first() {
                        if let Some(runs) = first_col
                            .pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs")
                            .and_then(|r| r.as_array())
                        {
                            if let Some(run0) = runs.first() {
                                if let Some(t) = run0.get("text").and_then(|s| s.as_str()) {
                                    title = t.to_string();
                                }
                                if video_id.is_empty() {
                                    if let Some(vid) = run0
                                        .pointer("/navigationEndpoint/watchEndpoint/videoId")
                                        .and_then(|s| s.as_str())
                                    {
                                        video_id = vid.to_string();
                                    }
                                }
                            }
                        }
                    }

                    if col_arr.len() > 1 {
                        if let Some(runs) = col_arr[1]
                            .pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs")
                            .and_then(|r| r.as_array())
                        {
                            let mut artist_parts = Vec::new();
                            for r in runs {
                                if let Some(t) = r.get("text").and_then(|s| s.as_str()) {
                                    if !Self::is_metadata_noise_run(t) {
                                        artist_parts.push(t.trim());
                                    }
                                }
                            }
                            artist = Self::strip_artist_metadata_suffixes(&artist_parts.join(" "))
                                .replace(" ,", ",");
                        }
                    }
                }

                if !video_id.is_empty() && !title.is_empty() {
                    let thumbnail = item
                        .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
                        .and_then(|th| th.as_array())
                        .and_then(|arr| arr.last())
                        .and_then(|t| t.get("url"))
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id));

                    let stream_url = format!("https://www.youtube.com/watch?v={}", video_id);

                    if !results.iter().any(|r| r.id == video_id) {
                        results.push(YouTubeTrack {
                            id: video_id,
                            title,
                            artist,
                            duration: 0.0,
                            thumbnail,
                            stream_url,
                        });
                    }
                }
            }

            for v in map.values() {
                Self::extract_innertube_tracks(v, results, limit);
            }
        } else if let Some(arr) = value.as_array() {
            for v in arr {
                Self::extract_innertube_tracks(v, results, limit);
            }
        }
    }

    pub fn search_ytdlp(query: &str, limit: usize) -> Vec<YouTubeTrack> {
        let max_results = limit.clamp(1, 50);
        let search_query = format!("ytsearch{}:{}", max_results, query);

        // Run yt-dlp to quickly extract search entries
        let mut cmd = Command::new("yt-dlp");
        cmd.arg("--flat-playlist")
            .arg("--print")
            .arg("%(id)s||%(title)s||%(uploader)s||%(duration)s")
            .arg(&search_query);

        // Sanitize environment when executed from AppImage
        if let Ok(ld) = std::env::var("LD_LIBRARY_PATH") {
            let filtered: Vec<&str> = ld
                .split(':')
                .filter(|p| !p.contains(".mount_") && !p.is_empty())
                .collect();
            if filtered.is_empty() {
                cmd.env_remove("LD_LIBRARY_PATH");
            } else {
                cmd.env("LD_LIBRARY_PATH", filtered.join(":"));
            }
        } else {
            cmd.env_remove("LD_LIBRARY_PATH");
        }
        cmd.env_remove("PYTHONHOME");
        cmd.env_remove("PYTHONPATH");

        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[YouTubeService] yt-dlp error: {:?}", e);
                return Vec::new();
            }
        };

        if !output.status.success() {
            return Vec::new();
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split("||").collect();
            if parts.len() >= 3 {
                let id = parts[0].trim().to_string();
                let title = parts[1].trim().to_string();
                let artist = parts[2].trim().to_string();
                let duration = parts.get(3)
                    .and_then(|d| d.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);

                let thumbnail = format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", id);
                let stream_url = format!("https://www.youtube.com/watch?v={}", id);

                if !id.is_empty() && id != "NA" {
                    results.push(YouTubeTrack {
                        id,
                        title,
                        artist,
                        duration,
                        thumbnail,
                        stream_url,
                    });
                }
            }
        }

        results
    }
}
