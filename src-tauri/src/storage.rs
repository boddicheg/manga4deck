use rusqlite::{Connection, Result};

use crate::logger::{
    info
};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &String) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(db_path).expect("Failed to open database");

        // initialize tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY, key TEXT, value TEXT)",
            [],
        )?;

        Ok(Database { conn })
    }   

    pub fn insert_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        // search setting if exist update else insert
        let mut stmt = self.conn.prepare("SELECT count(*) FROM settings WHERE key = ?")?;
        // cehck if length is 0 then insert else update
        let key_str: i32 = stmt.query_row([key], |row| row.get(0))?;
        if key_str != 0 {
            self.conn.execute(
                "UPDATE settings SET value = ? WHERE key = ?",
                [value.to_string(), key.to_string()],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO settings (key, value) VALUES (?, ?)",
                [key.to_string(), value.to_string()],
            )?;
        }

        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        match stmt.query_row([key], |row| row.get(0)) {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    } 
}


