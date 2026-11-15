use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

// Removed unused import

use crate::kavita::{
    Library, MangaPicture, Series, SeriesCover, SeriesMetadata, Volume, VolumeCover,
};

pub(crate) fn valid_picture_file(path: &str) -> bool {
    image::ImageReader::open(path)
        .ok()
        .and_then(|reader| reader.with_guessed_format().ok())
        .and_then(|reader| reader.into_dimensions().ok())
        .is_some_and(|(width, height)| width > 0 && height > 0)
}

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &String) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(db_path).expect("Failed to open database");

        // initialize tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY, key TEXT, value TEXT)",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS libraries (id INTEGER PRIMARY KEY, title TEXT)",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS series (id INTEGER PRIMARY KEY, library_id INTEGER, title TEXT, read INTEGER, pages INTEGER, last_updated TEXT)",
            [],
        )?;
        // Upgrade existing caches without removing saved progress or downloads.
        let has_update_date = {
            let mut stmt = conn.prepare("PRAGMA table_info(series)")?;
            let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
            columns
                .collect::<Result<Vec<_>>>()?
                .iter()
                .any(|name| name == "last_updated")
        };
        if !has_update_date {
            conn.execute("ALTER TABLE series ADD COLUMN last_updated TEXT", [])?;
        }
        conn.execute(
            "CREATE TABLE IF NOT EXISTS series_metadata (series_id INTEGER PRIMARY KEY, data TEXT NOT NULL)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS series_cover (series_id INTEGER PRIMARY KEY, file TEXT)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS volumes (id INTEGER PRIMARY KEY, series_id INTEGER, chapter_id INTEGER, volume_id INTEGER, title TEXT, read INTEGER, pages INTEGER)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS volumes_cover (volume_id INTEGER PRIMARY KEY, file TEXT)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS manga_pictures (id INTEGER PRIMARY KEY AUTOINCREMENT, chapter_id INTEGER, page INTEGER, file TEXT)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS read_progress (id INTEGER PRIMARY KEY AUTOINCREMENT, library_id INTEGER, series_id INTEGER, volume_id INTEGER, chapter_id INTEGER, page INTEGER)",
            [],
        )?;

        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // search setting if exist update else insert
        let mut stmt = conn.prepare("SELECT count(*) FROM settings WHERE key = ?")?;
        // cehck if length is 0 then insert else update
        let key_str: i32 = stmt.query_row([key], |row| row.get(0))?;
        if key_str != 0 {
            conn.execute(
                "UPDATE settings SET value = ? WHERE key = ?",
                [value.to_string(), key.to_string()],
            )?;
        } else {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?, ?)",
                [key.to_string(), value.to_string()],
            )?;
        }

        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        match stmt.query_row([key], |row| row.get(0)) {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn clean(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // conn.execute("DELETE FROM settings", [])?;
        conn.execute("DELETE FROM libraries", [])?;
        conn.execute("DELETE FROM series", [])?;
        conn.execute("DELETE FROM series_metadata", [])?;
        conn.execute("DELETE FROM series_cover", [])?;
        conn.execute("DELETE FROM volumes", [])?;
        conn.execute("DELETE FROM volumes_cover", [])?;
        conn.execute("DELETE FROM manga_pictures", [])?;
        // conn.execute("DROP TABLE IF EXISTS series", [])?;
        // conn.execute("DROP TABLE IF EXISTS series_cover", [])?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Library methods
    pub fn get_libraries(&self) -> Result<Vec<Library>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, title FROM libraries")?;
        let libraries = stmt.query_map([], |row| {
            Ok(Library {
                id: row.get(0)?,
                title: row.get(1)?,
            })
        })?;
        Ok(libraries.collect::<Result<Vec<Library>, rusqlite::Error>>()?)
    }

    pub fn add_library(&self, library: &Library) -> Result<(), Box<dyn std::error::Error>> {
        // info(&format!("Adding library: {:?}", library));
        let conn = self.conn.lock().unwrap();
        // check if library already exists
        let mut stmt = conn.prepare("SELECT count(*) FROM libraries WHERE id = ?")?;
        let key_str: i32 = stmt.query_row([library.id.to_string()], |row| row.get(0))?;
        if key_str == 0 {
            conn.execute(
                "INSERT INTO libraries (id, title) VALUES (?, ?)",
                [library.id.to_string(), library.title.to_string()],
            )?;
        }
        Ok(())
    }
    // -------------------------------------------------------------------------
    // Series methods
    pub fn save_series_metadata(
        &self,
        series_id: i32,
        metadata: &SeriesMetadata,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_string(metadata)?;
        self.conn.lock().unwrap().execute(
            "INSERT INTO series_metadata (series_id, data) VALUES (?, ?) ON CONFLICT(series_id) DO UPDATE SET data = excluded.data",
            rusqlite::params![series_id, data],
        )?;
        Ok(())
    }

    pub fn get_series_metadata(
        &self,
        series_id: i32,
    ) -> Result<SeriesMetadata, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM series_metadata WHERE series_id = ?")?;
        match stmt.query_row([series_id], |row| row.get::<_, String>(0)) {
            Ok(data) => Ok(serde_json::from_str(&data)?),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(SeriesMetadata::default()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn get_series(&self, library_id: &i32) -> Result<Vec<Series>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // Parse dates instead of comparing text: UTC offsets and fractional
        // seconds may differ. Unknown/invalid dates sort after dated series.
        let mut stmt = conn.prepare(
            "SELECT id, title, read, pages, last_updated FROM series WHERE library_id = ?
             ORDER BY julianday(last_updated) DESC, title COLLATE NOCASE ASC, id ASC",
        )?;
        let series = stmt.query_map([library_id.to_string()], |row| {
            Ok(Series {
                id: row.get(0)?,
                title: row.get(1)?,
                read: row.get(2)?,
                pages: row.get(3)?,
                library_id: library_id.clone(),
                is_cached: false,
                last_updated: row.get(4)?,
            })
        })?;
        Ok(series.collect::<Result<Vec<Series>, rusqlite::Error>>()?)
    }

    pub fn get_series_title(
        &self,
        series_id: i32,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        match conn.query_row(
            "SELECT title FROM series WHERE id = ?",
            [series_id],
            |row| row.get(0),
        ) {
            Ok(title) => Ok(Some(title)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn get_series_library_id(
        &self,
        series_id: i32,
    ) -> Result<Option<i32>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT library_id FROM series WHERE id = ?")?;
        match stmt.query_row([series_id.to_string()], |row| row.get(0)) {
            Ok(library_id) => Ok(Some(library_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn add_series(&self, series: &Series) -> Result<(), Box<dyn std::error::Error>> {
        // info(&format!("Adding series: {:?}", series));
        let conn = self.conn.lock().unwrap();
        // check if series already exist
        let mut stmt = conn.prepare("SELECT count(*) FROM series WHERE id = ?")?;
        let key_str: i32 = stmt.query_row([series.id.to_string()], |row| row.get(0))?;
        if key_str == 0 {
            conn.execute(
                "INSERT INTO series (id, library_id, title, read, pages, last_updated) VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    series.id,
                    series.library_id,
                    series.title,
                    series.read,
                    series.pages,
                    series.last_updated,
                ],
            )?;
        } else {
            // Refresh the server timestamp along with metadata and progress.
            conn.execute(
                "UPDATE series SET title = ?, read = ?, pages = ?, last_updated = ? WHERE id = ?",
                rusqlite::params![
                    series.title,
                    series.read,
                    series.pages,
                    series.last_updated,
                    series.id,
                ],
            )?;
        }
        Ok(())
    }

    pub fn add_series_cover(
        &self,
        series_cover: &SeriesCover,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // check if series cover already exists
        let mut stmt = conn.prepare("SELECT count(*) FROM series_cover WHERE series_id = ?")?;
        let key_str: i32 =
            stmt.query_row([series_cover.series_id.to_string()], |row| row.get(0))?;
        if key_str == 0 {
            conn.execute(
                "INSERT INTO series_cover (series_id, file) VALUES (?, ?)",
                [
                    series_cover.series_id.to_string(),
                    series_cover.file.to_string(),
                ],
            )?;
        } else {
            conn.execute(
                "UPDATE series_cover SET file = ? WHERE series_id = ?",
                [
                    series_cover.file.to_string(),
                    series_cover.series_id.to_string(),
                ],
            )?;
        }
        Ok(())
    }

    pub fn get_series_cover(
        &self,
        series_id: &i32,
    ) -> Result<SeriesCover, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT file FROM series_cover WHERE series_id = ?")?;
        let series_cover = stmt.query_row([series_id.to_string()], |row| row.get(0))?;
        Ok(SeriesCover {
            series_id: series_id.clone(),
            file: series_cover,
        })
    }
    // -------------------------------------------------------------------------
    // Volume methods
    pub fn get_volumes(&self, series_id: &i32) -> Result<Vec<Volume>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, read, pages, chapter_id, volume_id FROM volumes WHERE series_id = ?",
        )?;
        let volumes = stmt.query_map([series_id.to_string()], |row| {
            Ok(Volume {
                id: row.get(0)?,
                title: row.get(1)?,
                read: row.get(2)?,
                pages: row.get(3)?,
                chapter_id: row.get(4)?,
                volume_id: row.get(5)?,
                series_id: series_id.clone(),
                is_cached: false,
            })
        })?;
        Ok(volumes.collect::<Result<Vec<Volume>, rusqlite::Error>>()?)
    }

    pub fn get_volume_by_id(
        &self,
        volume_id: i32,
    ) -> Result<Option<Volume>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, series_id, chapter_id, volume_id, title, read, pages FROM volumes WHERE id = ?")?;
        let mut rows = stmt.query_map([volume_id.to_string()], |row| {
            Ok(Volume {
                id: row.get(0)?,
                series_id: row.get(1)?,
                chapter_id: row.get(2)?,
                volume_id: row.get(3)?,
                title: row.get(4)?,
                read: row.get(5)?,
                pages: row.get(6)?,
                is_cached: false,
            })
        })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Prefer the last opened item, then recent pending progress, then a read item
    /// from the local library. The saved item remains available after uploads
    /// remove acknowledged rows from `read_progress`.
    pub fn continue_reading(
        &self,
    ) -> Result<Vec<(Volume, String, i32)>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT v.id, v.series_id, v.chapter_id, v.volume_id, v.title, v.read, v.pages,
                    s.title, s.library_id
             FROM volumes v JOIN series s ON s.id = v.series_id
             WHERE v.pages > 0 AND (v.read > 0 OR v.id = CAST(
                 (SELECT value FROM settings WHERE key = 'last_read_volume' LIMIT 1) AS INTEGER))
             ORDER BY (v.id = COALESCE(CAST(
                 (SELECT value FROM settings WHERE key = 'last_read_volume' LIMIT 1) AS INTEGER), 0)) DESC,
                 COALESCE((SELECT MAX(rp.id) FROM read_progress rp
                           WHERE rp.series_id = v.series_id AND rp.chapter_id = v.chapter_id
                             AND rp.volume_id = v.volume_id), 0) DESC,
                 (v.read < v.pages) DESC, v.rowid DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                Volume {
                    id: row.get(0)?,
                    series_id: row.get(1)?,
                    chapter_id: row.get(2)?,
                    volume_id: row.get(3)?,
                    title: row.get(4)?,
                    read: row.get(5)?,
                    pages: row.get(6)?,
                    is_cached: false,
                },
                row.get(7)?,
                row.get(8)?,
            ))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn add_volume(&self, volume: &Volume) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // check if volume already exists
        let mut stmt = conn.prepare("SELECT count(*) FROM volumes WHERE id = ?")?;
        let key_str: i32 = stmt.query_row([volume.id.to_string()], |row| row.get(0))?;
        if key_str == 0 {
            conn.execute(
                "INSERT INTO volumes (id, series_id, chapter_id, volume_id, title, read, pages) VALUES (?, ?, ?, ?, ?, ?, ?)",
                [volume.id.to_string(), volume.series_id.to_string(), volume.chapter_id.to_string(), volume.volume_id.to_string(), volume.title.to_string(), volume.read.to_string(), volume.pages.to_string()],
            )?;
        } else {
            conn.execute(
                "UPDATE volumes SET title = ?, read = ?, pages = ? WHERE id = ?",
                [
                    volume.title.to_string(),
                    volume.read.to_string(),
                    volume.pages.to_string(),
                    volume.id.to_string(),
                ],
            )?;
        }
        Ok(())
    }

    /// Refresh shelf metadata atomically, including PDF books split into chapter entries.
    /// Cached pictures and offline progress stay attached to their Kavita chapter IDs.
    pub fn replace_volumes(
        &self,
        series_id: i32,
        volumes: &[Volume],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.conn.lock().unwrap();
        let transaction = conn.transaction()?;
        transaction.execute("DELETE FROM volumes WHERE series_id = ?", [series_id])?;
        for volume in volumes {
            transaction.execute(
                "INSERT INTO volumes (id, series_id, chapter_id, volume_id, title, read, pages) VALUES (?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![volume.id, volume.series_id, volume.chapter_id, volume.volume_id, volume.title, volume.read, volume.pages],
            )?;
        }
        // A shelf refresh may race a reconnect upload. Keep unsynced local progress
        // instead of replacing it with older server metadata.
        transaction.execute(
            "UPDATE volumes SET read = MIN(pages, (
                SELECT rp.page + 1 FROM read_progress rp
                WHERE rp.series_id = volumes.series_id AND rp.chapter_id = volumes.chapter_id
                AND rp.volume_id = volumes.volume_id ORDER BY rp.id DESC LIMIT 1
             )) WHERE series_id = ? AND pages > 0 AND EXISTS (
                SELECT 1 FROM read_progress rp WHERE rp.series_id = volumes.series_id
                AND rp.chapter_id = volumes.chapter_id AND rp.volume_id = volumes.volume_id
             )",
            [series_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn add_volume_cover(
        &self,
        volume_cover: &VolumeCover,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // check if volume cover already exists
        let mut stmt = conn.prepare("SELECT count(*) FROM volumes_cover WHERE volume_id = ?")?;
        let key_str: i32 =
            stmt.query_row([volume_cover.volume_id.to_string()], |row| row.get(0))?;
        if key_str == 0 {
            conn.execute(
                "INSERT INTO volumes_cover (volume_id, file) VALUES (?, ?)",
                [
                    volume_cover.volume_id.to_string(),
                    volume_cover.file.to_string(),
                ],
            )?;
        } else {
            conn.execute(
                "UPDATE volumes_cover SET file = ? WHERE volume_id = ?",
                [
                    volume_cover.file.to_string(),
                    volume_cover.volume_id.to_string(),
                ],
            )?;
        }
        Ok(())
    }

    pub fn get_volume_cover(
        &self,
        volume_id: &i32,
    ) -> Result<VolumeCover, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT file FROM volumes_cover WHERE volume_id = ?")?;
        let volume_cover = stmt.query_row([volume_id.to_string()], |row| row.get(0))?;
        Ok(VolumeCover {
            volume_id: volume_id.clone(),
            file: volume_cover,
        })
    }

    // -------------------------------------------------------------------------
    // Picture methods
    pub fn add_picture(&self, picture: &MangaPicture) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT count(*) FROM manga_pictures WHERE chapter_id = ? AND page = ?")?;
        let key_str: i32 = stmt.query_row(
            [picture.chapter_id.to_string(), picture.page.to_string()],
            |row| row.get(0),
        )?;
        if key_str == 0 {
            conn.execute(
                "INSERT INTO manga_pictures (chapter_id, page, file) VALUES (?, ?, ?)",
                [
                    picture.chapter_id.to_string(),
                    picture.page.to_string(),
                    picture.file.to_string(),
                ],
            )?;
        } else {
            conn.execute(
                "UPDATE manga_pictures SET file = ? WHERE chapter_id = ? AND page = ?",
                [
                    picture.file.to_string(),
                    picture.chapter_id.to_string(),
                    picture.page.to_string(),
                ],
            )?;
        }
        Ok(())
    }

    pub fn get_picture(
        &self,
        chapter_id: &i32,
        page: &i32,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT file FROM manga_pictures WHERE chapter_id = ? AND page = ?")?;
        let picture =
            stmt.query_row([chapter_id.to_string(), page.to_string()], |row| row.get(0))?;
        Ok(picture)
    }

    // Helper: get chapter_id and pages for a volume
    pub fn get_volume_chapter_and_pages(&self, volume_id: i32) -> Option<(i32, i32)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT chapter_id, pages FROM volumes WHERE id = ?")
            .ok()?;
        let mut rows = stmt.query([volume_id.to_string()]).ok()?;
        if let Some(row) = rows.next().ok()? {
            let chapter_id: i32 = row.get(0).ok()?;
            let pages: i32 = row.get(1).ok()?;
            Some((chapter_id, pages))
        } else {
            None
        }
    }

    pub fn is_volume_fully_cached(&self, volume_id: i32) -> bool {
        let Ok(Some(volume)) = self.get_volume_by_id(volume_id) else {
            return false;
        };
        if volume.pages <= 0 {
            return false;
        }
        let files: Result<std::collections::HashMap<i32, String>, rusqlite::Error> = (|| {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT page, file FROM manga_pictures WHERE chapter_id = ? AND page >= 0 AND page < ?")?;
            let rows = stmt.query_map([volume.chapter_id, volume.pages], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;
            rows.collect()
        })();
        let Ok(files) = files else {
            return false;
        };
        // Validate the exact required indices, not an aggregate count that can
        // include stale/out-of-range rows or files removed from disk.
        (0..volume.pages).all(|page| {
            files
                .get(&page)
                .is_some_and(|file| valid_picture_file(file))
        })
    }

    pub fn is_series_fully_cached(&self, series_id: i32) -> bool {
        let Ok(volumes) = self.get_volumes(&series_id) else {
            return false;
        };
        let unread: Vec<_> = volumes
            .iter()
            .filter(|v| v.pages > 0 && v.read < v.pages)
            .collect();
        !unread.is_empty() && unread.iter().all(|v| self.is_volume_fully_cached(v.id))
    }

    // Get all picture files for a series (through volumes)
    pub fn get_series_picture_files(
        &self,
        series_id: i32,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // Get all chapter_ids for volumes in this series
        let mut stmt = conn.prepare(
            "SELECT DISTINCT mp.file FROM manga_pictures mp INNER JOIN volumes v ON mp.chapter_id = v.chapter_id WHERE v.series_id = ?"
        )?;
        let rows = stmt.query_map([series_id.to_string()], |row| Ok(row.get::<_, String>(0)?))?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }
        Ok(files)
    }

    // Delete all cached pictures for a series
    pub fn delete_series_cache(&self, series_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // Delete all pictures for chapters in volumes of this series
        conn.execute(
            "DELETE FROM manga_pictures WHERE chapter_id IN (SELECT DISTINCT chapter_id FROM volumes WHERE series_id = ?)",
            [series_id.to_string()],
        )?;
        Ok(())
    }

    // Check if series has any cached volumes
    pub fn has_cached_volumes(&self, series_id: i32) -> bool {
        let conn = self.conn.lock().unwrap();
        let stmt = conn.prepare(
            "SELECT COUNT(*) FROM manga_pictures mp INNER JOIN volumes v ON mp.chapter_id = v.chapter_id WHERE v.series_id = ?"
        );
        match stmt {
            Ok(mut stmt) => {
                let count = stmt.query_row([series_id.to_string()], |r| r.get::<_, i32>(0));
                matches!(count, Ok(c) if c > 0)
            }
            Err(_) => false,
        }
    }

    // Read Progress methods
    pub fn add_read_progress(
        &self,
        progress: &crate::kavita::ReadProgress,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.conn.lock().unwrap();
        let transaction = conn.transaction()?;
        // Only the newest resume position for a chapter needs uploading. Insert a
        // new ID so an in-flight upload cannot delete a newer update's record.
        transaction.execute(
            "DELETE FROM read_progress WHERE library_id = ? AND series_id = ? AND volume_id = ? AND chapter_id = ?",
            rusqlite::params![progress.library_id, progress.series_id, progress.volume_id, progress.chapter_id],
        )?;
        transaction.execute(
            "INSERT INTO read_progress (library_id, series_id, volume_id, chapter_id, page) VALUES (?, ?, ?, ?, ?)",
            [
                progress.library_id.to_string(),
                progress.series_id.to_string(),
                progress.volume_id.to_string(),
                progress.chapter_id.to_string(),
                progress.page.to_string(),
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn get_all_read_progress(
        &self,
    ) -> Result<Vec<crate::kavita::ReadProgress>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, library_id, series_id, volume_id, chapter_id, page FROM read_progress ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::kavita::ReadProgress {
                id: Some(row.get(0)?),
                library_id: row.get(1)?,
                series_id: row.get(2)?,
                volume_id: row.get(3)?,
                chapter_id: row.get(4)?,
                page: row.get(5)?,
            })
        })?;

        let mut progress = Vec::new();
        for row in rows {
            progress.push(row?);
        }
        Ok(progress)
    }

    pub fn has_read_progress(&self, id: i32) -> Result<bool, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM read_progress WHERE id = ?)",
            [id],
            |row| row.get(0),
        )?)
    }

    pub fn delete_read_progress(&self, id: i32) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM read_progress WHERE id = ?", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCache(std::path::PathBuf);

    impl TestCache {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "manga4deck-storage-test-{}",
                crate::kavita::generate_hash_from_now()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn cache_page(&self, db: &Database, chapter_id: i32, page: i32) {
            let file = self.0.join(format!("{chapter_id}-{page}.png"));
            image::DynamicImage::new_rgb8(2, 3).save(&file).unwrap();
            db.add_picture(&MangaPicture {
                chapter_id,
                page,
                file: file.to_string_lossy().into_owned(),
            })
            .unwrap();
        }
    }

    impl Drop for TestCache {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn volume(id: i32, series_id: i32, chapter_id: i32, read: i32, pages: i32) -> Volume {
        Volume {
            id,
            series_id,
            chapter_id,
            volume_id: id,
            title: format!("Volume {id}"),
            read,
            pages,
            is_cached: false,
        }
    }

    #[test]
    fn cached_series_ignores_completed_volumes_and_stays_scoped_to_its_series() {
        let db = Database::new(&":memory:".to_string()).unwrap();
        let cache = TestCache::new();

        // Series 1 has one completed volume, which the cache worker skips, and
        // one unread volume, which has been fully cached.
        db.add_volume(&volume(11, 1, 101, 2, 2)).unwrap();
        db.add_volume(&volume(12, 1, 102, 0, 2)).unwrap();
        cache.cache_page(&db, 102, 0);
        cache.cache_page(&db, 102, 1);

        // A different series is only partially cached.
        db.add_volume(&volume(21, 2, 201, 0, 2)).unwrap();
        cache.cache_page(&db, 201, 0);

        assert!(db.is_series_fully_cached(1));
        assert!(!db.is_series_fully_cached(2));

        cache.cache_page(&db, 201, 1);
        assert!(db.is_series_fully_cached(2));
    }

    #[test]
    fn series_with_no_unread_pages_is_not_an_offline_series() {
        let db = Database::new(&":memory:".to_string()).unwrap();
        db.add_volume(&volume(11, 1, 101, 2, 2)).unwrap();

        assert!(!db.is_series_fully_cached(1));
    }

    #[test]
    fn refreshing_pdf_metadata_replaces_aggregate_volumes_and_preserves_page_cache() {
        let db = Database::new(&":memory:".to_string()).unwrap();
        let cache = TestCache::new();
        db.add_volume(&volume(11, 1, 101, 0, 2)).unwrap();
        db.add_volume(&volume(21, 2, 201, 0, 1)).unwrap();
        cache.cache_page(&db, 101, 0);
        let mut book = volume(-101, 1, 101, 0, 2);
        book.volume_id = 11;
        db.replace_volumes(1, &[book.clone()]).unwrap();
        assert!(db.get_volume_by_id(11).unwrap().is_none());
        assert_eq!(db.get_volume_by_id(-101).unwrap().unwrap().volume_id, 11);
        assert!(db.get_volume_by_id(21).unwrap().is_some());
        assert!(valid_picture_file(&db.get_picture(&101, &0).unwrap()));
        assert!(!db.is_volume_fully_cached(-101));
        cache.cache_page(&db, 101, 1);
        assert!(db.is_volume_fully_cached(-101));
        assert!(db.is_series_fully_cached(1));
        // Invalid replacement data must roll back instead of losing the existing shelf.
        assert!(db.replace_volumes(1, &[book.clone(), book]).is_err());
        assert_eq!(db.get_volumes(&1).unwrap().len(), 1);
    }
}
