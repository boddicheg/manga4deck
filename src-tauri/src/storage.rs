use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

// Removed unused import

use crate::kavita::{
    Library,
    Series,
    SeriesCover,
    Volume,
    VolumeCover,
    MangaPicture,
};

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
            "CREATE TABLE IF NOT EXISTS series (id INTEGER PRIMARY KEY, library_id INTEGER, title TEXT, read INTEGER, pages INTEGER)",
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

        Ok(Database { conn: Arc::new(Mutex::new(conn)) })
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
    pub fn get_series(&self, library_id: &i32) -> Result<Vec<Series>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, title, read, pages FROM series WHERE library_id = ?")?;
        let series = stmt.query_map([library_id.to_string()], |row| { 
            Ok(Series { 
                id: row.get(0)?,
                title: row.get(1)?, 
                read: row.get(2)?,
                pages: row.get(3)?,
                library_id: library_id.clone(),
            })
        })?;
        Ok(series.collect::<Result<Vec<Series>, rusqlite::Error>>()?)
    }

    pub fn add_series(&self, series: &Series) -> Result<(), Box<dyn std::error::Error>> {
        // info(&format!("Adding series: {:?}", series));
        let conn = self.conn.lock().unwrap();
        // check if series already exist
        let mut stmt = conn.prepare("SELECT count(*) FROM series WHERE id = ?")?;
        let key_str: i32 = stmt.query_row([series.id.to_string()], |row| row.get(0))?;
        if key_str == 0 {
            conn.execute(
                "INSERT INTO series (id, library_id, title, read, pages) VALUES (?, ?, ?, ?, ?)",
                [series.id.to_string(), series.library_id.to_string(), series.title.to_string(), series.read.to_string(), series.pages.to_string()],
            )?;
        } else {
            // Update existing series with new progress and title
            conn.execute(
                "UPDATE series SET title = ?, read = ?, pages = ? WHERE id = ?",
                [series.title.to_string(), series.read.to_string(), series.pages.to_string(), series.id.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn add_series_cover(&self, series_cover: &SeriesCover) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // check if series cover already exists
        let mut stmt = conn.prepare("SELECT count(*) FROM series_cover WHERE series_id = ?")?;
        let key_str: i32 = stmt.query_row([series_cover.series_id.to_string()], |row| row.get(0))?;
        if key_str == 0 {
            conn.execute(
                "INSERT INTO series_cover (series_id, file) VALUES (?, ?)",
                [series_cover.series_id.to_string(), series_cover.file.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn get_series_cover(&self, series_id: &i32) -> Result<SeriesCover, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT file FROM series_cover WHERE series_id = ?")?;
        let series_cover = stmt.query_row([series_id.to_string()], |row| row.get(0))?;
        Ok(SeriesCover { series_id: series_id.clone(), file: series_cover })
    }
    // -------------------------------------------------------------------------
    // Volume methods
    pub fn get_volumes(&self, series_id: &i32) -> Result<Vec<Volume>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, title, read, pages, chapter_id, volume_id FROM volumes WHERE series_id = ?")?;
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

    pub fn get_volume_by_id(&self, volume_id: i32) -> Result<Option<Volume>, Box<dyn std::error::Error>> {
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
        }
        else {
            conn.execute(
                "UPDATE volumes SET title = ?, read = ?, pages = ? WHERE id = ?",
                [volume.title.to_string(), volume.read.to_string(), volume.pages.to_string(), volume.id.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn add_volume_cover(&self, volume_cover: &VolumeCover) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // check if volume cover already exists
        let mut stmt = conn.prepare("SELECT count(*) FROM volumes_cover WHERE volume_id = ?")?;
        let key_str: i32 = stmt.query_row([volume_cover.volume_id.to_string()], |row| row.get(0))?;
        if key_str == 0 {
            conn.execute(
                "INSERT INTO volumes_cover (volume_id, file) VALUES (?, ?)",
                [volume_cover.volume_id.to_string(), volume_cover.file.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn get_volume_cover(&self, volume_id: &i32) -> Result<VolumeCover, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT file FROM volumes_cover WHERE volume_id = ?")?;
        let volume_cover = stmt.query_row([volume_id.to_string()], |row| row.get(0))?;
        Ok(VolumeCover { volume_id: volume_id.clone(), file: volume_cover })
    }

    // -------------------------------------------------------------------------
    // Picture methods
    pub fn add_picture(&self, picture: &MangaPicture) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT count(*) FROM manga_pictures WHERE chapter_id = ? AND page = ?")?;
        let key_str: i32 = stmt.query_row([picture.chapter_id.to_string(), picture.page.to_string()], |row| row.get(0))?; 
        if key_str == 0 {
            conn.execute(
                "INSERT INTO manga_pictures (chapter_id, page, file) VALUES (?, ?, ?)",
                [picture.chapter_id.to_string(), picture.page.to_string(), picture.file.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn get_picture(&self, chapter_id: &i32, page: &i32) -> Result<String, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT file FROM manga_pictures WHERE chapter_id = ? AND page = ?")?;
        let picture = stmt.query_row([chapter_id.to_string(), page.to_string()], |row| row.get(0))?;
        Ok(picture)
    }

    // Helper: get chapter_id and pages for a volume
    pub fn get_volume_chapter_and_pages(&self, volume_id: i32) -> Option<(i32, i32)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT chapter_id, pages FROM volumes WHERE id = ?").ok()?;
        let mut rows = stmt.query([volume_id.to_string()]).ok()?;
        if let Some(row) = rows.next().ok()? {
            let chapter_id: i32 = row.get(0).ok()?;
            let pages: i32 = row.get(1).ok()?;
            Some((chapter_id, pages))
        } else {
            None
        }
    }

    // Helper: check if a picture is cached
    pub fn is_picture_cached(&self, chapter_id: i32, page: i32) -> bool {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT count(*) FROM manga_pictures WHERE chapter_id = ? AND page = ?").unwrap();
        let count: i32 = stmt.query_row([chapter_id.to_string(), page.to_string()], |r| r.get(0)).unwrap();
        count > 0
    }

    // Get all picture files for a series (through volumes)
    pub fn get_series_picture_files(&self, series_id: i32) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // Get all chapter_ids for volumes in this series
        let mut stmt = conn.prepare(
            "SELECT DISTINCT mp.file FROM manga_pictures mp INNER JOIN volumes v ON mp.chapter_id = v.chapter_id WHERE v.series_id = ?"
        )?;
        let rows = stmt.query_map([series_id.to_string()], |row| {
            Ok(row.get::<_, String>(0)?)
        })?;
        
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
        let mut stmt = conn.prepare(
            "SELECT COUNT(*) FROM manga_pictures mp INNER JOIN volumes v ON mp.chapter_id = v.chapter_id WHERE v.series_id = ?"
        ).unwrap();
        let count: i32 = stmt.query_row([series_id.to_string()], |r| r.get(0)).unwrap();
        count > 0
    }

    // Read Progress methods
    pub fn add_read_progress(&self, progress: &crate::kavita::ReadProgress) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO read_progress (library_id, series_id, volume_id, chapter_id, page) VALUES (?, ?, ?, ?, ?)",
            [
                progress.library_id.to_string(),
                progress.series_id.to_string(),
                progress.volume_id.to_string(),
                progress.chapter_id.to_string(),
                progress.page.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn get_read_progress(&self, series_id: i32) -> Result<Vec<crate::kavita::ReadProgress>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, library_id, series_id, volume_id, chapter_id, page FROM read_progress WHERE series_id = ?")?;
        let rows = stmt.query_map([series_id.to_string()], |row| {
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

    pub fn get_all_read_progress(&self) -> Result<Vec<crate::kavita::ReadProgress>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, library_id, series_id, volume_id, chapter_id, page FROM read_progress")?;
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

    pub fn clear_read_progress(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM read_progress", [])?;
        Ok(())
    }

}

