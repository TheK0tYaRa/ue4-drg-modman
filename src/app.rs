use crate::db::{Database, ModEntry};
use crate::installer::ModInstaller;
use crate::mod_io::ModIoClient;
use crate::ui::render_ui;
use eframe::egui;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use keyring::Entry;

pub enum Tab {
    Browse,
    Installed,
    Settings,
}

pub enum ModAction {
    RequestDeleteConfirmation(String),
    CancelDeleteConfirmation(String),
    DeleteModVersion(String),
    UninstallMod(String),
    ToggleModEnabled(String, bool),
}

pub struct ModManager {
    pub mods: Vec<ModEntry>,
    pub selected_mods: HashSet<String>,
    pub search_query: String,
    pub show_installed_only: bool,
    pub current_tab: Tab,
    pub db: Database,
    pub profiles: Vec<String>,
    pub new_profile_name: String,
    pub show_delete_confirmation: bool,
    pub profile_to_delete: String,
    pub delete_confirmation_requested: bool,
    pub file_path: String,
    pub mod_delete_confirmation_requested: HashMap<String, bool>,
    pub mod_io_oauth_key: String,
    pub mod_io_client: ModIoClient,
    pub installer: ModInstaller,
    pub game_path: String,
    pub auto_update_mods: bool,
    pub enable_mod_debugging: bool,
    pub show_error_message: bool,
    pub error_message: String,
    pub notification_message: String,
    pub show_notification: bool,
    pub notification_time: f32,
}

impl ModManager {
    fn find_game_path() -> String {
        let possible_paths = [ //TODO: split it by platform
            // Steam default path on Windows
            "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Deep Rock Galactic\\FSD.exe",
            "C:\\Program Files\\Steam\\steamapps\\common\\Deep Rock Galactic\\FSD.exe",
            // Steam default path on Linux (via Proton)
            "~/.steam/steam/steamapps/common/Deep Rock Galactic/FSD.exe",
            // Microsoft Store / Game Pass path
            "C:\\Program Files\\WindowsApps\\CoffeeStainStudios.DeepRockGalactic",
            // Add more potential paths as needed
        ];

        for path in possible_paths.iter() {
            let expanded_path = if path.starts_with("~") {
                if let Some(home) = dirs::home_dir() {
                    home.join(&path[2..]).to_string_lossy().to_string()
                } else {
                    path.to_string()
                }
            } else {
                path.to_string()
            };

            if Path::new(&expanded_path).exists() {
                return expanded_path;
            }
        }

        // Return empty string if no valid path found
        String::new()
    }
        pub fn save_config(&mut self) {
            // For now, just print a message
            println!("Saving configuration with game path: {}", self.game_path);
            
            // In a real implementation, you would save to a config file
            // For example:
            // let config = Config {
            //     game_path: self.game_path.clone(),
            //     auto_update_mods: self.auto_update_mods,
            //     enable_mod_debugging: self.enable_mod_debugging,
            // };
            // let config_path = dirs::config_dir()
            //     .unwrap_or_else(|| std::path::PathBuf::from("."))
            //     .join("ue4-drg-modman")
            //     .join("config.json");
            // std::fs::create_dir_all(config_path.parent().unwrap()).ok();
            // std::fs::write(config_path, serde_json::to_string_pretty(&config).unwrap()).ok();
        }
        pub fn set_mod_io_oauth_key(&mut self, api_key: String) {
            if api_key != self.mod_io_oauth_key {
                self.mod_io_oauth_key = api_key;
                
                // Only initialize and call list_user_games if API key is not empty
                if !self.mod_io_oauth_key.is_empty() {
                    // Initialize ModIoClient if needed
                    if self.mod_io_client.is_uninitialized() {
                        self.mod_io_client = ModIoClient::new();
                    }
                    
                    // List user games
                    if let Err(e) = self.mod_io_client.list_user_games(&self.mod_io_oauth_key) {
                        self.error_message = format!("Error listing Mod.io games: {}", e);
                        self.show_error_message = true;
                    }
                }
            }
        }
        pub fn show_notification(&mut self, message: String) {
            self.notification_message = message;
            self.show_notification = true;
            self.notification_time = 5.0;
        }
    }

impl Default for ModManager {
    fn default() -> Self {
        // Initialize database
        let app_data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("ue4-drg-modman");
        
        std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");
        
        let db_path = app_data_dir.join("mods.db");
        let db = Database::new(&db_path).expect("Failed to initialize database");
        
        let profiles = db.get_profiles().unwrap_or_default();
        let mods = db.get_mods().unwrap_or_default();

        // Try to load the Mod.io API key from the keyring
        let mod_io_oauth_key = {
            let keyring_entry = Entry::new("ue4-drg-modman", "mod_io_oauth_key").unwrap();
            keyring_entry.get_password().unwrap_or_default()
        };

        Self {
            mods,
            selected_mods: HashSet::new(),
            search_query: String::new(),
            show_installed_only: false,
            current_tab: Tab::Browse,
            db,
            profiles,
            new_profile_name: String::new(),
            show_delete_confirmation: false,
            profile_to_delete: String::new(),
            delete_confirmation_requested: false,
            file_path: String::new(),
            mod_delete_confirmation_requested: HashMap::new(),
            mod_io_oauth_key,
            mod_io_client: ModIoClient::uninitialized(),
            installer: ModInstaller::new(app_data_dir),
            game_path: Self::find_game_path(),
            auto_update_mods: true,
            enable_mod_debugging: false,
            show_error_message: false,
            error_message: String::new(),
            notification_message: String::new(),
            show_notification: false,
            notification_time: 0.0,
        }
    }
}

impl eframe::App for ModManager {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Set dark theme
        // ctx.set_visuals(egui::Visuals::dark());
        
        render_ui(self, ctx, frame);
    }
}
