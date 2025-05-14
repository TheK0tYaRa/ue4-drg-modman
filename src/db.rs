use rusqlite::{Connection, Result, params};
use std::path::Path;

#[derive(Clone)]
pub struct ModEntry {
    pub mod_id: String,
    pub mod_name: String,
    pub mod_link: String,
    pub download_folder: String,
    pub selected_version: String,
    pub installed: bool,
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
        
        // Create global mods table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mods_global (
                mod_id TEXT PRIMARY KEY,
                mod_name TEXT NOT NULL,
                mod_link TEXT NOT NULL,
                download_folder TEXT NOT NULL
            )",
            [],
        )?;
        
        // Create versions table to store all available versions
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mod_versions (
                mod_id TEXT,
                version TEXT,
                PRIMARY KEY (mod_id, version),
                FOREIGN KEY(mod_id) REFERENCES mods_global(mod_id)
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
        
        // Create table for Default profile if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mods_Default (
                mod_id TEXT PRIMARY KEY,
                selected_version TEXT NOT NULL,
                installed INTEGER NOT NULL,
                enabled INTEGER NOT NULL,
                FOREIGN KEY(mod_id) REFERENCES mods_global(mod_id)
            )",
            [],
        )?;
        
        // Get all profiles and ensure they have tables
        // Create a scope for the statement to ensure it's dropped before we move conn
        {
            let mut stmt = conn.prepare("SELECT name FROM profiles")?;
            let profile_names = stmt.query_map([], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<Result<Vec<String>>>()?;
            
            for profile_name in profile_names {
                if profile_name != "Default" {
                    let table_name = format!("mods_{}", profile_name);
                    let query = format!(
                        "CREATE TABLE IF NOT EXISTS {} (
                            mod_id TEXT PRIMARY KEY,
                            selected_version TEXT NOT NULL,
                            installed INTEGER NOT NULL,
                            enabled INTEGER NOT NULL,
                            FOREIGN KEY(mod_id) REFERENCES mods_global(mod_id)
                        )",
                        table_name
                    );
                    
                    conn.execute(&query, [])?;
                }
            }
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
                selected_version TEXT NOT NULL,
                installed INTEGER NOT NULL,
                enabled INTEGER NOT NULL,
                FOREIGN KEY(mod_id) REFERENCES mods_global(mod_id)
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
        // First, get all mods from global table
        let mut stmt = self.conn.prepare(
            "SELECT mod_id, mod_name, mod_link, download_folder 
             FROM mods_global"
        )?;
        
        let global_mods = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // mod_id
                row.get::<_, String>(1)?, // mod_name
                row.get::<_, String>(2)?, // mod_link
                row.get::<_, String>(3)?, // download_folder
            ))
        })?
        .collect::<Result<Vec<(String, String, String, String)>>>()?;
        
        // Now get the installed/enabled status and selected version from the current profile
        let table_name = format!("mods_{}", self.current_profile);
        let query = format!(
            "SELECT mod_id, selected_version, installed, enabled FROM {}",
            table_name
        );
        
        let mut stmt = self.conn.prepare(&query)?;
        let profile_mods = stmt.query_map([], |row| {
            let mod_id: String = row.get(0)?;
            let selected_version: String = row.get(1)?;
            let installed: bool = row.get(2)?;
            let enabled: bool = row.get(3)?;
            Ok((mod_id, selected_version, installed, enabled))
        })?
        .collect::<Result<Vec<(String, String, bool, bool)>>>()?;
        
        // Create maps for profile data
        let profile_data: std::collections::HashMap<String, (String, bool, bool)> = profile_mods
            .into_iter()
            .map(|(id, ver, installed, enabled)| (id, (ver, installed, enabled)))
            .collect();
        
        // Combine the data
        let mut result = Vec::new();
        for (mod_id, mod_name, mod_link, download_folder) in global_mods {
            let (selected_version, installed, enabled) = profile_data
                .get(&mod_id)
                .cloned()
                .unwrap_or(("1.0.0".to_string(), false, false));
            
            result.push(ModEntry {
                mod_id,
                mod_name,
                mod_link,
                download_folder,
                selected_version,
                installed,
                enabled,
            });
        }
        
        Ok(result)
    }

    pub fn add_mod(&self, mod_entry: &ModEntry) -> Result<()> {
        // First, add or update the mod in the global table
        self.conn.execute(
            "INSERT OR REPLACE INTO mods_global 
             (mod_id, mod_name, mod_link, download_folder)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                mod_entry.mod_id,
                mod_entry.mod_name,
                mod_entry.mod_link,
                mod_entry.download_folder
            ],
        )?;
        
        // Add the version to the versions table
        self.conn.execute(
            "INSERT OR IGNORE INTO mod_versions 
             (mod_id, version)
             VALUES (?1, ?2)",
            params![
                mod_entry.mod_id,
                mod_entry.selected_version
            ],
        )?;
        
        // Then, add an entry in the current profile table if it doesn't exist
        let table_name = format!("mods_{}", self.current_profile);
        let query = format!(
            "INSERT OR IGNORE INTO {} 
             (mod_id, selected_version, installed, enabled)
             VALUES (?1, ?2, ?3, ?4)",
            table_name
        );
        
        self.conn.execute(
            &query,
            params![
                mod_entry.mod_id,
                mod_entry.selected_version,
                mod_entry.installed,
                mod_entry.enabled
            ],
        )?;
        
        Ok(())
    }

    pub fn update_mod_status(&self, mod_id: &str, installed: bool, enabled: bool) -> Result<()> {
        // Update both statuses in the current profile table
        let table_name = format!("mods_{}", self.current_profile);
        let query = format!(
            "UPDATE {} SET installed = ?1, enabled = ?2 WHERE mod_id = ?3",
            table_name
        );
        
        self.conn.execute(&query, params![installed, enabled, mod_id])?;
        
        Ok(())
    }

    pub fn update_mod_installed(&self, mod_id: &str, installed: bool) -> Result<()> {
        // Update just the installed status
        let table_name = format!("mods_{}", self.current_profile);
        let query = format!(
            "UPDATE {} SET installed = ?1 WHERE mod_id = ?2",
            table_name
        );
        
        self.conn.execute(&query, params![installed, mod_id])?;
        
        Ok(())
    }

    pub fn update_mod_enabled(&self, mod_id: &str, enabled: bool) -> Result<()> {
        // Update just the enabled status
        let table_name = format!("mods_{}", self.current_profile);
        let query = format!(
            "UPDATE {} SET enabled = ?1 WHERE mod_id = ?2",
            table_name
        );
        
        self.conn.execute(&query, params![enabled, mod_id])?;
        
        Ok(())
    }
}
