use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

use crate::logger::{
    info
};

use crate::kavita::{
    Library,
    Series,
    SeriesCover,
    Volume,
    VolumeCover,
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
            })
        })?;
        Ok(volumes.collect::<Result<Vec<Volume>, rusqlite::Error>>()?)
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
}

