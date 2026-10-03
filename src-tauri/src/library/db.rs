use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub file_path: String,
    pub cover_art: Option<String>,
    pub format: String,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub play_count: u32,
    pub favorite: bool,
    pub date_added: i64,
}

pub struct Database {
    conn: parking_lot::Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self> {
        let mut db_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        db_dir.push("aethon");
        std::fs::create_dir_all(&db_dir).ok();
        let db_path = db_dir.join("aethon.db");

        let conn = Connection::open(db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tracks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                album TEXT NOT NULL,
                duration REAL NOT NULL,
                file_path TEXT NOT NULL UNIQUE,
                cover_art TEXT,
                format TEXT NOT NULL,
                bitrate INTEGER,
                sample_rate INTEGER,
                play_count INTEGER NOT NULL DEFAULT 0,
                favorite INTEGER NOT NULL DEFAULT 0,
                date_added INTEGER NOT NULL
            );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS playlists (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL
            );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS playlist_tracks (
                playlist_id TEXT NOT NULL,
                track_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                PRIMARY KEY (playlist_id, track_id)
            );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS play_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id TEXT NOT NULL,
                played_at INTEGER NOT NULL
            );",
            [],
        )?;

        Ok(Self {
            conn: parking_lot::Mutex::new(conn),
        })
    }

    pub fn upsert_track(&self, track: &Track) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO tracks (id, title, artist, album, duration, file_path, cover_art, format, bitrate, sample_rate, play_count, favorite, date_added)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(file_path) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                duration = excluded.duration,
                cover_art = coalesce(excluded.cover_art, tracks.cover_art),
                format = excluded.format,
                bitrate = excluded.bitrate,
                sample_rate = excluded.sample_rate",
            params![
                track.id,
                track.title,
                track.artist,
                track.album,
                track.duration,
                track.file_path,
                track.cover_art,
                track.format,
                track.bitrate,
                track.sample_rate,
                track.play_count,
                if track.favorite { 1 } else { 0 },
                track.date_added
            ],
        )?;
        Ok(())
    }

    pub fn get_all_tracks(&self) -> Result<Vec<Track>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, album, duration, file_path, cover_art, format, bitrate, sample_rate, play_count, favorite, date_added
             FROM tracks ORDER BY title COLLATE NOCASE ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Track {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                duration: row.get(4)?,
                file_path: row.get(5)?,
                cover_art: row.get(6)?,
                format: row.get(7)?,
                bitrate: row.get(8)?,
                sample_rate: row.get(9)?,
                play_count: row.get(10)?,
                favorite: row.get::<_, i64>(11)? == 1,
                date_added: row.get(12)?,
            })
        })?;

        let mut tracks = Vec::new();
        for r in rows {
            if let Ok(track) = r {
                tracks.push(track);
            }
        }
        Ok(tracks)
    }

    pub fn toggle_favorite(&self, track_id: &str) -> Result<bool> {
        let conn = self.conn.lock();
        let current: i64 = conn.query_row(
            "SELECT favorite FROM tracks WHERE id = ?1",
            params![track_id],
            |row| row.get(0),
        ).unwrap_or(0);

        let new_val = if current == 1 { 0 } else { 1 };
        conn.execute(
            "UPDATE tracks SET favorite = ?1 WHERE id = ?2",
            params![new_val, track_id],
        )?;

        Ok(new_val == 1)
    }

    pub fn record_play(&self, track_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now = chrono_now();
        conn.execute(
            "UPDATE tracks SET play_count = play_count + 1 WHERE id = ?1",
            params![track_id],
        )?;
        conn.execute(
            "INSERT INTO play_history (track_id, played_at) VALUES (?1, ?2)",
            params![track_id, now],
        )?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut map = HashMap::new();
        for r in rows {
            if let Ok((k, v)) = r {
                map.insert(k, v);
            }
        }
        Ok(map)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn set_settings_batch(&self, settings: &HashMap<String, String>) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            )?;
            for (k, v) in settings {
                stmt.execute(params![k, v])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
