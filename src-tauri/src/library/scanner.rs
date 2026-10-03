use super::db::{Database, Track};
use lofty::file::AudioFile;
use lofty::prelude::*;
use lofty::probe::Probe;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct Scanner;

impl Scanner {
    pub fn scan_directory(dir_path: Option<PathBuf>, db: &Database) -> Vec<Track> {
        let music_dir = dir_path.unwrap_or_else(|| {
            dirs::audio_dir().unwrap_or_else(|| {
                dirs::home_dir().map(|h| h.join("Music")).unwrap_or_else(|| PathBuf::from("."))
            })
        });

        if !music_dir.exists() {
            let _ = std::fs::create_dir_all(&music_dir);
        }

        let mut scanned_tracks = Vec::new();

        for entry in WalkDir::new(&music_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = match path.extension().and_then(|s| s.to_str()) {
                Some(e) => e.to_lowercase(),
                None => continue,
            };

            if !matches!(ext.as_str(), "flac" | "mp3" | "m4a" | "alac" | "opus" | "ogg" | "wav" | "aac") {
                continue;
            }

            if let Some(track) = Self::read_metadata(path, &ext) {
                let _ = db.upsert_track(&track);
                scanned_tracks.push(track);
            }
        }

        scanned_tracks
    }

    fn read_metadata(path: &Path, ext: &str) -> Option<Track> {
        let path_str = path.to_string_lossy().to_string();
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown Track");

        let mut hasher = DefaultHasher::new();
        path_str.hash(&mut hasher);
        let id = format!("{:x}", hasher.finish());

        let mut title = file_stem.to_string();
        let mut artist = "Unknown Artist".to_string();
        let mut album = "Unknown Album".to_string();
        let mut duration = 0.0;
        let mut bitrate = None;
        let mut sample_rate = None;
        let mut cover_art = None;

        if let Ok(probe) = Probe::open(path) {
            if let Ok(tagged_file) = probe.read() {
                let properties = tagged_file.properties();
                duration = properties.duration().as_secs_f64();
                bitrate = properties.audio_bitrate();
                sample_rate = properties.sample_rate();

                if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
                    if let Some(t) = tag.title() {
                        if !t.trim().is_empty() {
                            title = t.trim().to_string();
                        }
                    }
                    if let Some(a) = tag.artist() {
                        if !a.trim().is_empty() {
                            artist = a.trim().to_string();
                        }
                    }
                    if let Some(al) = tag.album() {
                        if !al.trim().is_empty() {
                            album = al.trim().to_string();
                        }
                    }

                    if let Some(pic) = tag.pictures().first() {
                        let mime = pic.mime_type().map(|m| m.as_str()).unwrap_or("image/jpeg");
                        let b64 = base64_encode(pic.data());
                        cover_art = Some(format!("data:{};base64,{}", mime, b64));
                    }
                }
            }
        }

        let format = ext.to_uppercase();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Some(Track {
            id,
            title,
            artist,
            album,
            duration,
            file_path: path_str,
            cover_art,
            format,
            bitrate,
            sample_rate,
            play_count: 0,
            favorite: false,
            date_added: now,
        })
    }
}

fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = match chunk.len() {
            3 => (chunk[0] as usize) << 16 | (chunk[1] as usize) << 8 | (chunk[2] as usize),
            2 => (chunk[0] as usize) << 16 | (chunk[1] as usize) << 8,
            1 => (chunk[0] as usize) << 16,
            _ => unreachable!(),
        };

        result.push(CHARSET[(b >> 18) & 63] as char);
        result.push(CHARSET[(b >> 12) & 63] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[(b >> 6) & 63] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[b & 63] as char);
        } else {
            result.push('=');
        }
    }
    result
}
