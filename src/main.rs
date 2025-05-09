mod db;

use db::{Database, ModEntry};
use eframe::egui;
use egui::{Color32, RichText};
use std::collections::{HashSet, HashMap};

struct ModManager {
    mods: Vec<ModEntry>,
    selected_mods: HashSet<String>, // Changed from usize to String for mod_id
    search_query: String,
    show_installed_only: bool,
    current_tab: Tab,
    db: Database,
    profiles: Vec<String>,
    new_profile_name: String,
    show_delete_confirmation: bool,
    profile_to_delete: String,
    delete_confirmation_requested: bool,
    file_path: String,
    mod_delete_confirmation_requested: HashMap<String, bool>,
}

// struct Mod {
//     name: String,
//     author: String,
//     description: String,
//     installed: bool,
//     version: String,
// }

enum Tab {
    Browse,
    Installed,
    Settings,
}

enum ModAction {
    RequestDeleteConfirmation(String),
    CancelDeleteConfirmation(String),
    DeleteModVersion(String),
    UninstallMod(String),
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
        }
    }
}

impl ModManager {
    fn install_mod(&mut self, mod_id: &str) {
        // Find the mod in the list
        if let Some(mod_entry) = self.mods.iter().find(|m| m.mod_id == mod_id) {
            // Create the download directory if it doesn't exist
            let app_data_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("ue4-drg-modman");
            
            let download_dir = app_data_dir.join(&mod_entry.download_folder);
            std::fs::create_dir_all(&download_dir).unwrap_or_else(|_| {
                println!("Failed to create download directory: {:?}", download_dir);
            });
            
            // Create a version-specific directory
            let version_dir = download_dir.join(&mod_entry.selected_version);
            std::fs::create_dir_all(&version_dir).unwrap_or_else(|_| {
                println!("Failed to create version directory: {:?}", version_dir);
            });
            
            // Determine if it's a URL or file path
            let is_url = mod_entry.mod_link.starts_with("http://") || 
                         mod_entry.mod_link.starts_with("https://");
            
            if is_url {
                // TODO: Implement URL download
                // For now, just print a message
                println!("Would download from URL: {}", mod_entry.mod_link);
                
                // After download, update the mod status
                if let Ok(()) = self.db.update_mod_status(&mod_entry.mod_id, true) {
                    // Reload mods
                    if let Ok(mods) = self.db.get_mods() {
                        self.mods = mods;
                    }
                }
            } else {
                // It's a local file, copy it to the version directory
                let source_path = std::path::Path::new(&mod_entry.mod_link);
                if source_path.exists() {
                    let file_name = source_path.file_name().unwrap_or_else(|| {
                        std::ffi::OsStr::new("mod_file")
                    });
                    let dest_path = version_dir.join(file_name);
                    
                    match std::fs::copy(source_path, &dest_path) {
                        Ok(_) => {
                            println!("Copied mod file to: {:?}", dest_path);
                            
                            // Update the mod status
                            if let Ok(()) = self.db.update_mod_status(&mod_entry.mod_id, true) {
                                // Reload mods
                                if let Ok(mods) = self.db.get_mods() {
                                    self.mods = mods;
                                }
                            }
                        },
                        Err(e) => {
                            println!("Failed to copy mod file: {}", e);
                        }
                    }
                } else {
                    println!("Source file does not exist: {}", mod_entry.mod_link);
                }
            }
        }
    }
}

impl eframe::App for ModManager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set dark theme
        ctx.set_visuals(egui::Visuals::dark());

        // Flag to track if we need to reload mods
        // let needs_reload = false;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("DRG Mod Manager");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Refresh").clicked() {
                        // Refresh mod list
                    }
                });
            });
            ui.separator();
            
            // Tab selection
            ui.horizontal(|ui| {
                if ui.selectable_label(matches!(self.current_tab, Tab::Browse), "Browse").clicked() {
                    self.current_tab = Tab::Browse;
                }
                if ui.selectable_label(matches!(self.current_tab, Tab::Installed), "Installed").clicked() {
                    self.current_tab = Tab::Installed;
                }
                if ui.selectable_label(matches!(self.current_tab, Tab::Settings), "Settings").clicked() {
                    self.current_tab = Tab::Settings;
                }
                
            });
            
            // Mod file input section - only show in Browse tab
            if matches!(self.current_tab, Tab::Browse) {
                ui.horizontal(|ui| {
                    // Add button to process the file path
                    if ui.button("[+]").clicked() && !self.file_path.is_empty() {
                        // Determine if it's a URL or file path
                        let is_url = self.file_path.starts_with("http://") || 
                                    self.file_path.starts_with("https://");
                        
                        // Create a new mod entry
                        let mod_id = format!("mod_{}", chrono::Utc::now().timestamp());
                        let mod_name = if is_url {
                            // Extract name from URL if possible
                            self.file_path.split('/').last().unwrap_or("New Mod").to_string()
                        } else {
                            // Extract name from file path
                            std::path::Path::new(&self.file_path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("New Mod")
                                .to_string()
                        };
                        
                        let new_mod = ModEntry {
                            mod_id,
                            mod_name,
                            mod_link: self.file_path.clone(),
                            download_folder: "downloads".to_string(),
                            selected_version: "1.0.0".to_string(),
                            enabled: false,
                        };
                        
                        // Add the mod to the database
                        if let Ok(()) = self.db.add_mod(&new_mod) {
                            // Reload mods
                            if let Ok(mods) = self.db.get_mods() {
                                self.mods = mods;
                            }
                            // Clear the file path
                            self.file_path.clear();
                        }
                    }
                    
                    ui.add_space(4.0);
                    
                    // File selector button
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            if let Some(path_str) = path.to_str() {
                                self.file_path = path_str.to_string();
                            }
                        }
                    }
                    
                    ui.add_space(4.0);
                    
                    // File path input that stretches to fill available space
                    ui.add(egui::TextEdit::singleline(&mut self.file_path)
                        .desired_width(ui.available_width())
                        .hint_text("Mod file path or URL...")
                    );
                });
            }
        });

        egui::SidePanel::left("side_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Profiles");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_label("")
                        .selected_text(self.db.get_current_profile())
                        .show_ui(ui, |ui| {
                            for profile in &self.profiles {
                                if ui.selectable_label(
                                    profile == self.db.get_current_profile(),
                                    profile
                                ).clicked() {
                                    self.db.set_current_profile(profile.clone());
                                    // Reload mods for this profile
                                    if let Ok(mods) = self.db.get_mods() {
                                        self.mods = mods;
                                    }
                                }
                            }
                        });
                    
                        let current_profile = self.db.get_current_profile().to_string();
                        if current_profile != "Default" {
                            ui.horizontal(|ui| {
                                if !self.delete_confirmation_requested {
                                    if ui.button("🗑").clicked() {
                                        self.delete_confirmation_requested = true;
                                    }
                                } else {
                                    // First button (cancel)
                                    if ui.button("🗑").clicked() {
                                        self.delete_confirmation_requested = false;
                                    }
                                    
                                    // Second button (confirm - red)
                                    if ui.add(egui::Button::new(
                                        RichText::new("🗑").color(Color32::RED)
                                    )).clicked() {
                                        if let Ok(()) = self.db.delete_profile(&current_profile) {
                                            self.profiles = self.db.get_profiles().unwrap_or_default();
                                            self.db.set_current_profile("Default".to_string());
                                            if let Ok(mods) = self.db.get_mods() {
                                                self.mods = mods;
                                            }
                                        }
                                        self.delete_confirmation_requested = false;
                                    }
                                    
                                    // Auto-cancel if mouse moves away
                                    if !ui.ui_contains_pointer() {
                                        self.delete_confirmation_requested = false;
                                    }
                                }
                            });
                        }
                });                
                // Add profile creation UI
                ui.horizontal(|ui| {
                    ui.label("New profile:");
                    ui.text_edit_singleline(&mut self.new_profile_name);
                });

                if ui.button("Create Profile").clicked() && !self.new_profile_name.is_empty() {
                    if let Ok(()) = self.db.create_profile(&self.new_profile_name) {
                        self.profiles = self.db.get_profiles().unwrap_or_default();
                        self.db.set_current_profile(self.new_profile_name.clone());
                        self.new_profile_name.clear();
                    }
                }
                ui.separator();

                ui.heading("Filters");
                ui.separator();
                
                // Search field with on change trigger
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query)
                        .on_hover_text("Search mods by name");
                });
                
                // Bool switch that slides to the side
                ui.horizontal(|ui| {
                    ui.label("Installed only:");
                    ui.add(egui::widgets::Checkbox::new(&mut self.show_installed_only, ""));
                });
                
                // Collapsing section (rollout)
                egui::CollapsingHeader::new("Categories")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.selectable_value(&mut (), (), "Gameplay");
                        ui.selectable_value(&mut (), (), "Visual");
                        ui.selectable_value(&mut (), (), "Audio");
                        ui.selectable_value(&mut (), (), "Quality of Life");
                    });
                
                ui.separator();
                
                // Colored label
                ui.label(
                    RichText::new("Selected: ")
                        .color(Color32::from_rgb(255, 255, 255))
                        .background_color(Color32::from_rgb(45, 100, 45))
                        .strong()
                );
                ui.label(format!("{} mods", self.selected_mods.len()));
                
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    if ui.button("Install Selected").clicked() {
                        // Install logic
                    }
                });
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                match self.current_tab {
                    Tab::Browse | Tab::Installed => {
                        // Flag to track if we need to reload mods
                        let mut needs_reload = false;
                        // Store mod ID to install (if any)
                        let mut mod_to_install: Option<String> = None;
                        // Store mod actions to perform after rendering
                        let mut mod_actions: Vec<ModAction> = Vec::new();
                        
                        // Filter mods based on search and installed status
                        let filtered_mods: Vec<&ModEntry> = self.mods
                            .iter()
                            .filter(|m| {
                                m.mod_name.to_lowercase().contains(&self.search_query.to_lowercase()) &&
                                (!self.show_installed_only || m.enabled)
                            })
                            .collect();
                        
                        // Scrollable list with overlay scrollbar
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                            .show(ui, |ui| {
    

                            for mod_item in &filtered_mods {
                                let is_selected = self.selected_mods.contains(&mod_item.mod_id);
                                let response = ui.selectable_label(
                                    is_selected,
                                    "");
                                
                                // Make the whole row selectable
                                if response.clicked() {
                                    if is_selected {
                                        self.selected_mods.remove(&mod_item.mod_id);
                                    } else {
                                        self.selected_mods.insert(mod_item.mod_id.clone());
                                    }
                                }
                
                                                
                                // Draw the row content
                                let _ = response.rect.shrink(4.0);
                                let painter = ui.painter();
                                if is_selected {
                                    painter.rect_filled(
                                        response.rect,
                                        4.0,
                                        Color32::from_rgb(60, 80, 120),
                                    );
                                }
                                
                                ui.horizontal(|ui| {
                                    // Status indicator
                                    let status_color = if mod_item.enabled {
                                        Color32::from_rgb(100, 200, 100)
                                    } else {
                                        Color32::from_rgb(200, 100, 100)
                                    };
                                    
                                    ui.label(
                                        RichText::new(if mod_item.enabled { "✓" } else { "✗" })
                                            .color(status_color)
                                            .strong()
                                    );
                                    
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(&mod_item.mod_name).strong());
                                        ui.horizontal(|ui| {
                                            ui.label(format!("ID: {}", mod_item.mod_id));
                                            ui.label(format!("v{}", mod_item.selected_version));
                                        });
                                        ui.label(&mod_item.mod_link);
                                    });
                                    
                                    // let mod_id = mod_item.mod_id.clone();
                                    // let enabled = mod_item.enabled;
                                    
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        // Delete button with confirmation
                                        let mod_id = mod_item.mod_id.clone();
                                        let is_delete_requested = self.mod_delete_confirmation_requested.get(&mod_id).copied().unwrap_or(false);
                                        
                                        if !is_delete_requested {
                                            if ui.button("🗑").clicked() {
                                                // Store action to perform after the loop
                                                mod_actions.push(ModAction::RequestDeleteConfirmation(mod_id.clone()));
                                            }
                                        } else {
                                            // First button (cancel)
                                            if ui.button("🗑").clicked() {
                                                // Store action to perform after the loop
                                                mod_actions.push(ModAction::CancelDeleteConfirmation(mod_id.clone()));
                                            }
                                            
                                            // Second button (confirm - red) - to the left of the first one
                                            if ui.add(egui::Button::new(
                                                RichText::new("🗑").color(Color32::RED)
                                            )).clicked() {
                                                // Store action to perform after the loop
                                                if matches!(self.current_tab, Tab::Browse) {
                                                    mod_actions.push(ModAction::DeleteModVersion(mod_id.clone()));
                                                } else {
                                                    mod_actions.push(ModAction::UninstallMod(mod_id.clone()));
                                                }
                                            }
                                            
                                            // Auto-cancel if mouse moves away
                                            if !ui.ui_contains_pointer() {
                                                mod_actions.push(ModAction::CancelDeleteConfirmation(mod_id.clone()));
                                            }
                                        }

                                        // Show different buttons based on tab
                                        if matches!(self.current_tab, Tab::Browse) {
                                            // Show Install button in Browse tab
                                            if !mod_item.enabled {
                                                if ui.button("Install").clicked() {
                                                    // Store the mod ID to install after the loop
                                                    mod_to_install = Some(mod_id.clone());
                                                }
                                            }
                                        } else if matches!(self.current_tab, Tab::Installed) {
                                            // Show Enable/Disable button in Installed tab
                                            if ui.button(if mod_item.enabled { "Disable" } else { "Enable" }).clicked() {
                                                let new_status = !mod_item.enabled;
                                                
                                                // Store the action to perform after the loop
                                                if let Ok(()) = self.db.update_mod_status(&mod_id, new_status) {
                                                    // Flag that we need to reload mods after the loop
                                                    needs_reload = true;
                                                }
                                            }
                                        }
                                    });
                                });
                                
                                ui.separator();
                            }
                        });

                    // Process mod actions
                    for action in mod_actions {
                        match action {
                            ModAction::RequestDeleteConfirmation(mod_id) => {
                                self.mod_delete_confirmation_requested.insert(mod_id, true);
                            },
                            ModAction::CancelDeleteConfirmation(mod_id) => {
                                self.mod_delete_confirmation_requested.remove(&mod_id);
                            },
                            ModAction::DeleteModVersion(mod_id) => {
                                // Delete the selected version from cache
                                if let Some(mod_entry) = self.mods.iter().find(|m| m.mod_id == mod_id) {
                                    let app_data_dir = dirs::data_dir()
                                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                                        .join("ue4-drg-modman");
                                    
                                    let version_dir = app_data_dir
                                        .join(&mod_entry.download_folder)
                                        .join(&mod_entry.selected_version);
                                    
                                    if version_dir.exists() {
                                        if let Err(e) = std::fs::remove_dir_all(&version_dir) {
                                            println!("Failed to delete version directory: {}", e);
                                        }
                                    }
                                }
                                self.mod_delete_confirmation_requested.remove(&mod_id);
                                needs_reload = true;
                            },
                            ModAction::UninstallMod(mod_id) => {
                                // Update the mod status to disabled
                                if let Ok(()) = self.db.update_mod_status(&mod_id, false) {
                                    needs_reload = true;
                                }
                                self.mod_delete_confirmation_requested.remove(&mod_id);
                            },
                        }
                    }
    
                    // Apply any changes that were requested during rendering
                    if let Some(mod_id) = mod_to_install {
                        self.install_mod(&mod_id);
                    }
                    
                    if needs_reload {
                        if let Ok(mods) = self.db.get_mods() {
                            self.mods = mods;
                        }
                    }
                }, // End of Tab::Browse | Tab::Installed match arm
                Tab::Settings => {
                    ui.heading("Settings");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("Game Path:");
                        ui.text_edit_singleline(&mut String::new());
                        if ui.button("Browse").clicked() {
                            // Open file dialog
                        }
                    });
                    
                    ui.checkbox(&mut true, "Auto-update mods");
                    ui.checkbox(&mut false, "Enable mod debugging");
                    
                    ui.separator();
                    ui.label(
                        RichText::new("Warning: Modding may affect game performance")
                            .color(Color32::from_rgb(255, 200, 0))
                    );
                }
            }
        });
        if self.show_delete_confirmation {
            egui::Window::new("Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("Are you sure you want to delete profile '{}'?", self.profile_to_delete));
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            if let Ok(()) = self.db.delete_profile(&self.profile_to_delete) {
                                self.profiles = self.db.get_profiles().unwrap_or_default();
                                self.db.set_current_profile("Default".to_string());
                                if let Ok(mods) = self.db.get_mods() {
                                    self.mods = mods;
                                }
                            }
                            self.show_delete_confirmation = false;
                        }
                        if ui.button("No").clicked() {
                            self.show_delete_confirmation = false;
                        }
                    });
                });
        }
    }}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "DRG Mod Manager",
        options,
        Box::new(|_cc| -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(Box::new(ModManager::default()))
        }),
    )
}
