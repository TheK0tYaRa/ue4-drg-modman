use rusqlite::{Connection, Result, params};
use std::path::Path;

pub struct ModEntry {
    pub mod_id: String,
    pub mod_name: String,
    pub mod_link: String,
    pub download_folder: String,
    pub selected_version: String,
    pub enabled: bool,
}

pub struct Database {
    conn: Connection,
    current_profile: String,
}

impl Database {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        
        // Create profiles table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS profiles (
                name TEXT PRIMARY KEY
            )",
            [],
        )?;
        
        // Check if Default profile exists, create if not
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM profiles WHERE name = 'Default'",
            [],
            |row| row.get(0),
        )?;
        
        if count == 0 {
            conn.execute(
                "INSERT INTO profiles (name) VALUES ('Default')",
                [],
            )?;
        }
        
        Ok(Self {
            conn,
            current_profile: "Default".to_string(),
        })
    }
    
    pub fn create_profile(&self, profile_name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO profiles (name) VALUES (?1)",
            params![profile_name],
        )?;
        
        // Create table for this profile
        let table_name = format!("mods_{}", profile_name);
        let query = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                mod_id TEXT PRIMARY KEY,
                mod_name TEXT NOT NULL,
                mod_link TEXT NOT NULL,
                download_folder TEXT NOT NULL,
                selected_version TEXT NOT NULL,
                enabled INTEGER NOT NULL
            )",
            table_name
        );
        
        self.conn.execute(&query, [])?;
        
        Ok(())
    }

    pub fn delete_profile(&self, profile_name: &str) -> Result<()> {
        // Don't allow deleting the Default profile
        if profile_name == "Default" {
            return Err(rusqlite::Error::InvalidParameterName("Cannot delete Default profile".to_string()));
        }
        
        // Delete the profile from profiles table
        self.conn.execute(
            "DELETE FROM profiles WHERE name = ?1",
            params![profile_name],
        )?;
        
        // Drop the mods table for this profile
        let table_name = format!("mods_{}", profile_name);
        let query = format!("DROP TABLE IF EXISTS {}", table_name);
        self.conn.execute(&query, [])?;
        
        Ok(())
    }
   
    
    pub fn get_profiles(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM profiles ORDER BY name")?;
        let profiles = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(name)
        })?
        .collect::<Result<Vec<String>>>()?;
        
        Ok(profiles)
    }
    
    pub fn set_current_profile(&mut self, profile: String) {
        self.current_profile = profile;
    }
    
    pub fn get_current_profile(&self) -> &str {
        &self.current_profile
    }
    
    pub fn get_mods(&self) -> Result<Vec<ModEntry>> {
        let table_name = format!("mods_{}", self.current_profile);
        let query = format!(
            "SELECT mod_id, mod_name, mod_link, download_folder, selected_version, enabled 
             FROM {} ORDER BY mod_name",
            table_name
        );
        
        let mut stmt = self.conn.prepare(&query)?;
        let mods = stmt.query_map([], |row| {
            Ok(ModEntry {
                mod_id: row.get(0)?,
                mod_name: row.get(1)?,
                mod_link: row.get(2)?,
                download_folder: row.get(3)?,
                selected_version: row.get(4)?,
                enabled: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<ModEntry>>>()?;
        
        Ok(mods)
    }
    
    pub fn add_mod(&self, mod_entry: &ModEntry) -> Result<()> {
        let table_name = format!("mods_{}", self.current_profile);
        let query = format!(
            "INSERT OR REPLACE INTO {} 
             (mod_id, mod_name, mod_link, download_folder, selected_version, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            table_name
        );
        
        self.conn.execute(
            &query,
            params![
                mod_entry.mod_id,
                mod_entry.mod_name,
                mod_entry.mod_link,
                mod_entry.download_folder,
                mod_entry.selected_version,
                mod_entry.enabled
            ],
        )?;
        
        Ok(())
    }
    
    pub fn update_mod_status(&self, mod_id: &str, enabled: bool) -> Result<()> {
        let table_name = format!("mods_{}", self.current_profile);
        let query = format!(
            "UPDATE {} SET enabled = ?1 WHERE mod_id = ?2",
            table_name
        );
        
        self.conn.execute(&query, params![enabled, mod_id])?;
        
        Ok(())
    }
    
    pub fn update_mod_version(&self, mod_id: &str, version: &str) -> Result<()> {
        let table_name = format!("mods_{}", self.current_profile);
        let query = format!(
            "UPDATE {} SET selected_version = ?1 WHERE mod_id = ?2",
            table_name
        );
        
        self.conn.execute(&query, params![version, mod_id])?;
        
        Ok(())
    }
}