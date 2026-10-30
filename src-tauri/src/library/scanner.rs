use super::db::{Database, Track};
use lofty::file::AudioFile;
use lofty::prelude::*;
use lofty::probe::Probe;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn deterministic_track_id(path_str: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in path_str.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("trk_{:016x}", hash)
}

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
                scanned_tracks.push(track);
            }
        }

        let _ = db.upsert_tracks_batch(&scanned_tracks);
        scanned_tracks
    }

    fn read_metadata(path: &Path, ext: &str) -> Option<Track> {
        let path_str = path.to_string_lossy().to_string();
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown Track");
        let id = deterministic_track_id(&path_str);

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
                        let data = pic.data();
                        if !data.is_empty() {
                            let mut art_hasher = 0xcbf29ce484222325u64;
                            for b in data.iter().take(8192) {
                                art_hasher ^= *b as u64;
                                art_hasher = art_hasher.wrapping_mul(0x100000001b3);
                            }
                            let ext_pic = if pic.mime_type().map(|m| m.as_str()).unwrap_or("").contains("png") { "png" } else { "jpg" };
                            let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("aethon").join("covers");
                            let _ = std::fs::create_dir_all(&cache_dir);
                            let cover_file = cache_dir.join(format!("{:016x}.{}", art_hasher, ext_pic));
                            if !cover_file.exists() {
                                let _ = std::fs::write(&cover_file, data);
                            }
                            cover_art = Some(cover_file.to_string_lossy().to_string());
                        }
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
