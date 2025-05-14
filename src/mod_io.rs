use serde::{Deserialize, Serialize};
use reqwest::blocking::Client;
use std::{error::Error, io::Read};

const MOD_IO_API_URL: &str = "https://api.mod.io/v1";
const MOD_IO_GAME_ID: u32 = 2475; // Deep Rock Galactic game ID

#[derive(Debug, Serialize, Deserialize)]
pub struct ModIoMod {
    pub id: u32,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub logo: ModIoLogo,
    pub submitted_by: ModIoUser,
    pub date_added: i64,
    pub date_updated: i64,
    pub stats: ModIoStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModIoLogo {
    pub filename: String,
    pub original: String,
    pub thumb_320x180: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModIoUser {
    pub username: String,
    pub profile_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModIoStats {
    pub downloads_total: u32,
    pub subscribers_total: u32,
    pub rating_total: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModIoResponse {
    pub data: Vec<ModIoMod>,
}

pub struct ModIoClient {
    client: Client,
    initialized: bool,
    user_id: Option<u32>,
}

impl ModIoClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            initialized: true,
            user_id: None,
        }
    }

    pub fn uninitialized() -> Self {
        Self {
            client: Client::new(),
            initialized: false,
            user_id: None,
        }
    }
    
    pub fn is_uninitialized(&self) -> bool {
        !self.initialized
    }
    
    // Get the API URL, using user-specific URL if user_id is available
    fn get_api_url(&self) -> String {
        if let Some(user_id) = self.user_id {
            format!("https://u-{}.modapi.io/v1", user_id)
        } else {
            MOD_IO_API_URL.to_string()
        }
    }
    
    // Get user ID from the API
    pub fn get_user_id(&mut self, api_key: &str) -> Result<u32, Box<dyn Error>> {
        // Use the standard API URL to get user info
        let url = format!("{}/me", MOD_IO_API_URL);
        
        println!("Fetching user info from mod.io: {}", url);
        
        let response = self.client.get(&url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .send()?;
        
        if response.status().is_success() {
            let body = response.text()?;
            println!("User info response: {}", body);
            
            // Parse the JSON to extract user ID
            let json: serde_json::Value = serde_json::from_str(&body)?;
            if let Some(user_id) = json.get("id").and_then(|id| id.as_u64()) {
                let user_id = user_id as u32;
                self.user_id = Some(user_id);
                println!("Got user ID: {}", user_id);
                return Ok(user_id);
            } else {
                return Err("User ID not found in response".into());
            }
        } else {
            let status = response.status();
            let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Error fetching user info: HTTP {}, {}", status, error_text).into());
        }
    }
    
    pub fn list_user_games(&mut self, api_key: &str) -> Result<(), Box<dyn Error>> {
        // First, get the user ID if we don't have it yet
        if self.user_id.is_none() {
            self.get_user_id(api_key)?;
        }
        
        // Now use the user-specific API URL
        let url = format!("{}/me/games", self.get_api_url());
        
        println!("Fetching user games from mod.io: {}", url);
        
        let response = self.client.get(&url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .send()?;
        
        // Check if the request was successful
        if response.status().is_success() {
            // Get the response body as text
            let body = response.text()?;
            
            // Debug print the response
            println!("Response from mod.io API:");
            println!("{}", body);
            
            // In a real implementation, you would parse this into a struct
            // let games: ModIoGamesResponse = serde_json::from_str(&body)?;
            // return Ok(games);
        } else {
            println!("Error fetching user games: HTTP {}", response.status());
            if let Ok(error_text) = response.text() {
                println!("Error details: {}", error_text);
            }
        }
        
        Ok(())
    }
    
    // Update other methods to use get_api_url()
    pub fn get_mods(&self, offset: u32, limit: u32) -> Result<Vec<ModIoMod>, Box<dyn Error>> {
        let url = format!("{}/games/{}/mods?offset={}&limit={}", 
                         self.get_api_url(), MOD_IO_GAME_ID, offset, limit);
        
        println!("Fetching mods from mod.io: {}", url);
        
        let response = self.client.get(&url)
            .header("Accept", "application/json")
            .send()?
            .json::<ModIoResponse>()?;
        
        Ok(response.data)
    }
    
    pub fn get_mod_by_id(&self, mod_id: u32) -> Result<ModIoMod, Box<dyn Error>> {
        let url = format!("{}/games/{}/mods/{}", 
                         self.get_api_url(), MOD_IO_GAME_ID, mod_id);
        
        println!("Fetching mod details from mod.io: {}", url);
        
        let response = self.client.get(&url)
            .header("Accept", "application/json")
            .send()?
            .json::<ModIoMod>()?;
        
        Ok(response)
    }

    pub fn parse_mod_io_url(url: &str) -> Option<(String, u32)> {
        // List of supported games
        const SUPPORTED_GAMES: &[&str] = &["drg", "deeprockgalactic"];
        
        // Parse URLs like "https://mod.io/g/drg/m/mod-hub#description"
        if url.contains("mod.io/g/") {
            // Extract the game name from the URL
            let parts: Vec<&str> = url.split("/g/").collect();
            if parts.len() > 1 {
                let game_parts: Vec<&str> = parts[1].split('/').collect();
                if game_parts.is_empty() {
                    return None;
                }
                
                let game_name = game_parts[0].to_lowercase();
                
                // Check if the game is supported
                if !SUPPORTED_GAMES.contains(&game_name.as_str()) {
                    return None;
                }
                
                // Extract the mod name from the URL
                if url.contains("/m/") {
                    let mod_parts: Vec<&str> = url.split("/m/").collect();
                    if mod_parts.len() > 1 {
                        // Extract just the mod name, removing any fragments or query parameters
                        let mod_name_with_extras = mod_parts[1];
                        let mod_name = mod_name_with_extras
                            .split('#').next().unwrap_or(mod_name_with_extras) // Remove fragment
                            .split('?').next().unwrap_or(mod_name_with_extras); // Remove query parameters
                        
                        // For now, we'll just return a dummy ID with the game name
                        // In a real implementation, you would query the mod.io API to get the actual mod ID
                        return Some((game_name.to_string(), 12345));
                    }
                }
            }
        }
        None
    }

/*
    pub fn get_mods(&self, offset: u32, limit: u32) -> Result<Vec<ModIoMod>, Box<dyn Error>> {
        let url = format!("{}/games/{}/mods?offset={}&limit={}", 
                         MOD_IO_API_URL, MOD_IO_GAME_ID, offset, limit);
        
        println!("Fetching mods from mod.io: {}", url);
        
        let response = self.client.get(&url)
            .header("Accept", "application/json")
            .send()?
            .json::<ModIoResponse>()?;
        
        Ok(response.data)
    }
    
    pub fn get_mod_by_id(&self, mod_id: u32) -> Result<ModIoMod, Box<dyn Error>> {
        let url = format!("{}/games/{}/mods/{}", 
                         MOD_IO_API_URL, MOD_IO_GAME_ID, mod_id);
        
        println!("Fetching mod details from mod.io: {}", url);
        
        let response = self.client.get(&url)
            .header("Accept", "application/json")
            .send()?
            .json::<ModIoMod>()?;
        
        Ok(response)
    }
*/
    
    pub fn convert_to_mod_entry(&self, mod_io_mod: &ModIoMod) -> crate::db::ModEntry {
        crate::db::ModEntry {
            mod_id: format!("modio_{}", mod_io_mod.id),
            mod_name: mod_io_mod.name.clone(),
            mod_link: format!("https://mod.io/g/drg/m/{}", mod_io_mod.id),
            download_folder: "downloads".to_string(),
            selected_version: "1.0.0".to_string(), // Default version
            installed: false,
            enabled: false,
        }
    }

/*
    pub fn list_user_games(&self, api_key: &str) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/me/games", MOD_IO_API_URL);
        
        println!("Fetching user games from mod.io: {}", url);
        
        let response = self.client.get(&url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .send()?;
        
        // Check if the request was successful
        if response.status().is_success() {
            // Get the response body as text
            let body = response.text()?;
            
            // Debug print the response
            println!("Response from mod.io API:");
            println!("{}", body);
            
            // In a real implementation, you would parse this into a struct
            // let games: ModIoGamesResponse = serde_json::from_str(&body)?;
            // return Ok(games);
        } else {
            println!("Error fetching user games: HTTP {}", response.status());
            if let Ok(error_text) = response.text() {
                println!("Error details: {}", error_text);
            }
        }
        
        Ok(())
    }
*/
}

impl Default for ModIoClient {
    fn default() -> Self {
        Self::uninitialized()
    }
}