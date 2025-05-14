use crate::app::{ModAction, ModManager, Tab};
use crate::db::ModEntry;
use crate::mod_io::{ModIoMod, ModIoClient};
use eframe::egui;
use egui::{Color32, RichText};
use keyring::Entry;

pub fn render_ui(
    app: &mut ModManager,
    ctx: &egui::Context,
    frame: &mut eframe::Frame
) {
    // Set dark theme
    ctx.set_visuals(egui::Visuals::dark());
    
    // Get frame time for animations
    let frame_time = frame.info().cpu_usage.unwrap_or(0.016); // Default to 60 FPS if unknown
    
    // Render the main UI components
    render_top_panel(app, ctx);
    render_side_panel(app, ctx);
    render_central_panel(app, ctx);
    render_dialogs(app, ctx);
    
    // Render notifications on top
    render_notifications(app, ctx, frame_time);
}
//
fn render_notifications(app: &mut ModManager, ctx: &egui::Context, frame_time: f32) {
    if app.show_notification {
        // Update notification time
        app.notification_time -= frame_time;
        if app.notification_time <= 0.0 {
            app.show_notification = false;
        }
        
        // Calculate position and opacity
        let screen_rect = ctx.screen_rect();
        let notification_width = 300.0;
        let notification_height = 50.0;
        let margin = 20.0;
        
        let x_position = screen_rect.right() - notification_width - margin;
        let y_position = screen_rect.top() + margin;
        
        let rect = egui::Rect::from_min_size(
            egui::pos2(x_position, y_position),
            egui::vec2(notification_width, notification_height),
        );
        
        // Calculate opacity (fade out at the end)
        let opacity = if app.notification_time < 1.0 {
            app.notification_time
        } else {
            1.0
        };
        
        // Draw notification
        let notification_color = Color32::from_rgba_premultiplied(0, 150, 0, (opacity * 220.0) as u8);
        let text_color = Color32::from_rgba_premultiplied(255, 255, 255, (opacity * 255.0) as u8);
        
        egui::Window::new("Notification")
            .frame(egui::Frame::none().fill(notification_color))
            .title_bar(false)
            .resizable(false)
            .fixed_rect(rect)
            .show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new(&app.notification_message).color(text_color).strong());
                });
            });
    }
}
//
fn render_top_panel(app: &mut ModManager, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("DRG Mod Manager");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    // Refresh mod list
                    if let Ok(mods) = app.db.get_mods() {
                        app.mods = mods;
                    }
                }
            });
        });
        ui.separator();
        
        // Tab selection
        ui.horizontal(|ui| {
            if ui.selectable_label(matches!(app.current_tab, Tab::Browse), "Browse").clicked() {
                app.current_tab = Tab::Browse;
            }
            if ui.selectable_label(matches!(app.current_tab, Tab::Installed), "Installed").clicked() {
                app.current_tab = Tab::Installed;
            }
            if ui.selectable_label(matches!(app.current_tab, Tab::Settings), "Settings").clicked() {
                app.current_tab = Tab::Settings;
            }
        });
        
        // Mod file input section - only show in Browse tab
        if matches!(app.current_tab, Tab::Browse) {
            ui.horizontal(|ui| {
                // Add button to process the file path
                if ui.button("[+]").clicked() && !app.file_path.is_empty() {
                    // Create a new mod entry
                    let mod_id = format!("mod_{}", chrono::Utc::now().timestamp());
                    let is_url = app.file_path.starts_with("http");
                    
                    let mod_name = if is_url {
                        // Extract name from URL if possible
                        app.file_path.split('/').last().unwrap_or("New Mod").to_string()
                    } else {
                        // Extract name from file path
                        std::path::Path::new(&app.file_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("New Mod")
                            .to_string()
                    };
                    
                    let new_mod = ModEntry {
                        mod_id,
                        mod_name,
                        mod_link: app.file_path.clone(),
                        download_folder: "downloads".to_string(),
                        selected_version: "1.0.0".to_string(),
                        installed: false,
                        enabled: false,
                    };
                    
                    // Add the mod to the database
                    if let Ok(()) = app.db.add_mod(&new_mod) {
                        // Reload mods
                        if let Ok(mods) = app.db.get_mods() {
                            app.mods = mods;
                        }
                        // Clear the file path
                        app.file_path.clear();
                    }
                }
                
                ui.add_space(4.0);
                
                // File selector button
                if ui.button("Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        if let Some(path_str) = path.to_str() {
                            app.file_path = path_str.to_string();
                        }
                    }
                }
                
                ui.add_space(4.0);
                
                // File path input that stretches to fill available space
                ui.add(egui::TextEdit::singleline(&mut app.file_path)
                    .desired_width(ui.available_width())
                    .hint_text("Mod file path or URL...")
                );
            });
        }
    });
}

fn render_side_panel(app: &mut ModManager, ctx: &egui::Context) {
    egui::SidePanel::left("side_panel")
        .resizable(true)
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Profiles");
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("")
                    .selected_text(app.db.get_current_profile())
                    .show_ui(ui, |ui| {
                        for profile in &app.profiles {
                            if ui.selectable_label(
                                profile == app.db.get_current_profile(),
                                profile
                            ).clicked() {
                                app.db.set_current_profile(profile.clone());
                                // Reload mods for this profile
                                if let Ok(mods) = app.db.get_mods() {
                                    app.mods = mods;
                                }
                            }
                        }
                    });
                
                let current_profile = app.db.get_current_profile().to_string();
                if current_profile != "Default" {
                    ui.horizontal(|ui| {
                        if !app.delete_confirmation_requested {
                            if ui.button("🗑").clicked() {
                                app.delete_confirmation_requested = true;
                            }
                        } else {
                            // First button (cancel)
                            if ui.button("🗑").clicked() {
                                app.delete_confirmation_requested = false;
                            }
                            
                            // Second button (confirm - red)
                            if ui.add(egui::Button::new(
                                RichText::new("🗑").color(Color32::RED)
                            )).clicked() {
                                if let Ok(()) = app.db.delete_profile(&current_profile) {
                                    app.profiles = app.db.get_profiles().unwrap_or_default();
                                    app.db.set_current_profile("Default".to_string());
                                    if let Ok(mods) = app.db.get_mods() {
                                        app.mods = mods;
                                    }
                                }
                                app.delete_confirmation_requested = false;
                            }
                            
                            // Auto-cancel if mouse moves away
                            if !ui.ui_contains_pointer() {
                                app.delete_confirmation_requested = false;
                            }
                        }
                    });
                }
            });
            
            // Add profile creation UI
            ui.horizontal(|ui| {
                ui.label("New profile:");
                ui.text_edit_singleline(&mut app.new_profile_name);
            });

            if ui.button("Create Profile").clicked() && !app.new_profile_name.is_empty() {
                if let Ok(()) = app.db.create_profile(&app.new_profile_name) {
                    app.profiles = app.db.get_profiles().unwrap_or_default();
                    app.db.set_current_profile(app.new_profile_name.clone());
                    app.new_profile_name.clear();
                }
            }
            ui.separator();

            ui.heading("Filters");
            ui.separator();
            
            // Search field with on change trigger
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut app.search_query)
                    .on_hover_text("Search mods by name");
            });
            
            // Bool switch that slides to the side
            ui.horizontal(|ui| {
                ui.label("Installed only:");
                ui.add(egui::widgets::Checkbox::new(&mut app.show_installed_only, ""));
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
            ui.label(format!("{} mods", app.selected_mods.len()));
            
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                if ui.button("Install Selected").clicked() {
                    // Install selected mods
                    for mod_id in &app.selected_mods.clone() {
                        if let Some(mod_entry) = app.mods.iter().find(|m| &m.mod_id == mod_id) {
                            if let Ok(()) = app.installer.install_mod(mod_entry) {
                                if let Ok(()) = app.db.update_mod_installed(&mod_id, true) {
                                    // Mod installed successfully
                                }
                            }
                        }
                    }
                    
                    // Reload mods
                    if let Ok(mods) = app.db.get_mods() {
                        app.mods = mods;
                    }
                }
            });
        });
}

fn render_central_panel(app: &mut ModManager, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        match app.current_tab {
            Tab::Browse | Tab::Installed => {
                render_mod_list(app, ui);
            },
            Tab::Settings => {
                ui.heading("Settings");
                ui.separator();
                
                ui.horizontal(|ui| {
                    ui.label("Game Path:");
                    ui.text_edit_singleline(&mut app.game_path)
                        .on_hover_text("Path to your Deep Rock Galactic installation");
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Select DRG Executable")
                            .pick_file() {
                            if let Some(path_str) = path.to_str() {
                                app.game_path = path_str.to_string();
                                // Save the game path to config
                                app.save_config();
                            }
                        }
                    }
                });

                ui.add_space(10.0);
                ui.heading("Mod.io Integration");
                ui.separator();

                // OAuth2 Key input with password masking
                let mut oauth_key = app.mod_io_oauth_key.clone();
                ui.horizontal(|ui| {
                    ui.label("OAuth2 Key:");
                    
                    // Use a password-style text edit (masked with asterisks)
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut oauth_key)
                            .password(true) // This masks the input with asterisks
                            .hint_text("Enter your Mod.io OAuth2 token")
                    );
                    
                    // Only update the OAuth2 key in memory, don't make API calls yet
                    if response.changed() {
                        app.mod_io_oauth_key = oauth_key;
                    }
                    
                    // Add a "Check" button that will validate the OAuth2 key
                    if ui.button("Check").clicked() && !app.mod_io_oauth_key.is_empty() {
                        // Initialize ModIoClient if needed
                        if app.mod_io_client.is_uninitialized() {
                            app.mod_io_client = ModIoClient::new();
                        }
                        
                        // List user games to validate the OAuth2 key
                        match app.mod_io_client.list_user_games(&app.mod_io_oauth_key) {
                            Ok(_) => {
                                // API key is valid, store it in the keyring
                                let keyring_entry = Entry::new("ue4-drg-modman", "mod_io_api_key").unwrap();
                                if let Err(e) = keyring_entry.set_password(&app.mod_io_oauth_key) {
                                    app.error_message = format!("Error saving OAuth2 key to keyring: {}", e);
                                    app.show_error_message = true;
                                } else {
                                    // Use notification instead of error message
                                    app.show_notification("OAuth2 validated successfully and saved to keyring.".to_string());
                                }
                            },
                            Err(e) => {
                                app.error_message = format!("Error validating Mod.io OAuth2: {}", e);
                                app.show_error_message = true;
                            }
                        }
                    }
                    
                    // Add delete button for clearing the API key
                    if !app.mod_io_oauth_key.is_empty() {
                        let mut mod_io_key_delete_requested = false;
                        
                        if !mod_io_key_delete_requested {
                            if ui.button("🗑").clicked() {
                                mod_io_key_delete_requested = true;
                            }
                        } else {
                            // First button (cancel)
                            if ui.button("🗑").clicked() {
                                mod_io_key_delete_requested = false;
                            }
                            
                            // Second button (confirm - red)
                            if ui.add(egui::Button::new(
                                RichText::new("🗑").color(Color32::RED)
                            )).clicked() {
                                // Clear the OAuth2 key from memory
                                app.mod_io_oauth_key.clear();
                                
                                // Remove from keyring
                                let keyring_entry = Entry::new("ue4-drg-modman", "mod_io_oauth_key").unwrap();
                                if let Err(e) = keyring_entry.delete_credential() {
                                    // Only show error if it's not because the credential doesn't exist
                                    if !e.to_string().contains("No such keyring entry") {
                                        app.error_message = format!("Error removing OAuth2 key from keyring: {}", e);
                                        app.show_error_message = true;
                                    }
                                }
                                
                                mod_io_key_delete_requested = false;
                            }
                            
                            // Auto-cancel if mouse moves away
                            if !ui.ui_contains_pointer() {
                                mod_io_key_delete_requested = false;
                            }
                        }
                    }
                });
                
                // Display OAuth2 key status
                if app.mod_io_oauth_key.is_empty() {
                    ui.label(RichText::new("No OAuth2 token. Mod.io integration is disabled.")
                        .color(Color32::from_rgb(255, 200, 0)));
                } else {
                    ui.label(RichText::new("Click 'Check' to validate the token.")
                        .color(Color32::from_rgb(100, 200, 100)));
                }
                
                // Add help text explaining how to get an OAuth Access token
                ui.collapsing("How to get a Mod.io OAuth2 token", |ui| {
                    ui.label("1. Create an account on mod.io");
                    ui.label("2. Go to your account settings");
                    ui.label("3. Navigate to the 'OAuth2 Access' section");
                    ui.label("4. Generate a new OAuth 2.0 token with the 'read' scope");
                    ui.label("5. Copy the token and paste it here");
                    
                    // if ui.button("Open mod.io").clicked() { // i forgot which dependency contains open::that()
                    //     if let Err(e) = open::that("https://mod.io") {
                    //         app.error_message = format!("Failed to open browser: {}", e);
                    //         app.show_error_message = true;
                    //     }
                    // }
                });

                ui.add_space(10.0);
                
                ui.checkbox(&mut app.auto_update_mods, "Auto-update mods")
                    .on_hover_text("Automatically check for mod updates on startup");
                
                ui.checkbox(&mut app.enable_mod_debugging, "Enable mod debugging")
                    .on_hover_text("Enable additional logging for mod operations");
                
                ui.separator();
                ui.label(
                    RichText::new("Warning: Modding may affect game performance")
                        .color(Color32::from_rgb(255, 200, 0))
                );
            }
        }
    });
}

fn render_dialogs(app: &mut ModManager, ctx: &egui::Context) {
    if app.show_delete_confirmation {
        egui::Window::new("Confirm Deletion")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Are you sure you want to delete profile '{}'?", app.profile_to_delete));
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        if let Ok(()) = app.db.delete_profile(&app.profile_to_delete) {
                            app.profiles = app.db.get_profiles().unwrap_or_default();
                            app.db.set_current_profile("Default".to_string());
                            if let Ok(mods) = app.db.get_mods() {
                                app.mods = mods;
                            }
                        }
                        app.show_delete_confirmation = false;
                    }
                    if ui.button("No").clicked() {
                        app.show_delete_confirmation = false;
                    }
                });
            });
    }
    
    // Add any other dialog windows here
    if app.show_error_message {
        egui::Window::new("Error")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(&app.error_message);
                if ui.button("OK").clicked() {
                    app.show_error_message = false;
                }
            });
    }
}
//
pub fn render_mod_list(
    app: &mut ModManager,
    ui: &mut egui::Ui
) {
    // Filter mods based on search and tab
    // Clone the filtered mods to avoid borrowing app
    let filtered_mods: Vec<ModEntry> = app.mods
        .iter()
        .filter(|m| {
            // Always filter by search query
            let matches_search = m.mod_name.to_lowercase().contains(&app.search_query.to_lowercase());
            
            match app.current_tab {
                Tab::Browse => {
                    // In Browse tab, show all mods with optional filter for installed status
                    matches_search && (!app.show_installed_only || m.installed)
                },
                Tab::Installed => {
                    // In Installed tab, only show mods that are installed in the current profile
                    matches_search && m.installed
                },
                _ => false,
            }
        })
        .cloned() // Clone each ModEntry
        .collect();
    
    // Track changes that need to be applied after rendering
    let mut needs_reload = false;
    let mut mod_to_install: Option<String> = None;
    let mut mod_actions: Vec<ModAction> = Vec::new();
    
    // Render the scrollable list of mods
    render_mod_scrollable_list(app, ui, &filtered_mods, &mut mod_actions, &mut mod_to_install);
    
    // Process actions collected during rendering
    process_mod_actions(app, &mod_actions, &mut needs_reload);
    
    // Handle installation requests
    if let Some(mod_id) = mod_to_install {
        install_mod(app, &mod_id, &mut needs_reload);
    }
    
    // Reload mods if needed
    if needs_reload {
        reload_mods(app);
    }
}

fn render_mod_scrollable_list(
    app: &mut ModManager, 
    ui: &mut egui::Ui, 
    filtered_mods: &[ModEntry], // Changed from &[&ModEntry] to &[ModEntry]
    mod_actions: &mut Vec<ModAction>,
    mod_to_install: &mut Option<String>
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .show(ui, |ui| {
            for mod_item in filtered_mods {
                render_mod_row(app, ui, mod_item, mod_actions, mod_to_install);
                ui.separator();
            }
        });
}

fn render_mod_row(
    app: &mut ModManager, 
    ui: &mut egui::Ui, 
    mod_item: &ModEntry,
    mod_actions: &mut Vec<ModAction>,
    mod_to_install: &mut Option<String>
) {
    let is_selected = app.selected_mods.contains(&mod_item.mod_id);
    let response = ui.selectable_label(is_selected, "");
    
    // Make the whole row selectable
    if response.clicked() {
        if is_selected {
            app.selected_mods.remove(&mod_item.mod_id);
        } else {
            app.selected_mods.insert(mod_item.mod_id.clone());
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
        render_mod_status(ui, mod_item);
        
        // Mod details
        render_mod_details(ui, mod_item);
        
        // Action buttons
        render_mod_actions(app, ui, mod_item, mod_actions, mod_to_install);
    });
}

fn render_mod_status(
    ui: &mut egui::Ui,
    mod_item: &ModEntry
) {
    let status_color = if mod_item.enabled {
        Color32::from_rgb(100, 200, 100) // Green for enabled
    } else if mod_item.installed {
        Color32::from_rgb(200, 200, 100) // Yellow for installed but not enabled
    } else {
        Color32::from_rgb(200, 100, 100) // Red for not installed
    };
    
    let status_text = if mod_item.enabled {
        "✓" // Enabled
    } else if mod_item.installed {
        "⚙" // Installed but not enabled
    } else {
        "✗" // Not installed
    };
    
    ui.label(
        RichText::new(status_text)
            .color(status_color)
            .strong()
    );
}

fn render_mod_details(
    ui: &mut egui::Ui,
    mod_item: &ModEntry
) {
    ui.vertical(|ui| {
        ui.label(RichText::new(&mod_item.mod_name).strong());
        ui.horizontal(|ui| {
            ui.label(format!("ID: {}", mod_item.mod_id));
            ui.label(format!("v{}", mod_item.selected_version));
        });
        ui.label(&mod_item.mod_link);
    });
}

fn render_mod_actions(
    app: &mut ModManager, 
    ui: &mut egui::Ui, 
    mod_item: &ModEntry,
    mod_actions: &mut Vec<ModAction>,
    mod_to_install: &mut Option<String>
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        // Delete button with confirmation
        render_delete_button(app, ui, mod_item, mod_actions);

        // Show different buttons based on tab
        if matches!(app.current_tab, Tab::Browse) {
            render_browse_tab_buttons(ui, mod_item, mod_to_install);
        } else if matches!(app.current_tab, Tab::Installed) {
            render_installed_tab_buttons(app, ui, mod_item, mod_actions);
        }
    });
}

fn render_delete_button(
    app: &ModManager, 
    ui: &mut egui::Ui, 
    mod_item: &ModEntry,
    mod_actions: &mut Vec<ModAction>
) {
    let mod_id = mod_item.mod_id.clone();
    let is_delete_requested = app.mod_delete_confirmation_requested.get(&mod_id).copied().unwrap_or(false);
    
    if !is_delete_requested {
        if ui.button("🗑").clicked() {
            mod_actions.push(ModAction::RequestDeleteConfirmation(mod_id.clone()));
        }
    } else {
        // First button (cancel)
        if ui.button("🗑").clicked() {
            mod_actions.push(ModAction::CancelDeleteConfirmation(mod_id.clone()));
        }
        
        // Second button (confirm - red) - to the left of the first one
        if ui.add(egui::Button::new(
            RichText::new("🗑").color(Color32::RED)
        )).clicked() {
            if matches!(app.current_tab, Tab::Browse) {
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
}

fn render_browse_tab_buttons(
    ui: &mut egui::Ui, 
    mod_item: &ModEntry,
    mod_to_install: &mut Option<String>
) {
    // Show Install button in Browse tab if not installed
    if !mod_item.installed {
        if ui.button("Install").clicked() {
            *mod_to_install = Some(mod_item.mod_id.clone());
        }
    }
}

fn render_installed_tab_buttons(
    app: &ModManager, 
    ui: &mut egui::Ui, 
    mod_item: &ModEntry,
    mod_actions: &mut Vec<ModAction>
) {
    // Show Enable/Disable button in Installed tab
    if ui.button(if mod_item.enabled { "Disable" } else { "Enable" }).clicked() {
        let mod_id = mod_item.mod_id.clone();
        let new_status = !mod_item.enabled;
        
        // We'll handle this in process_mod_actions
        mod_actions.push(ModAction::ToggleModEnabled(mod_id, new_status));
    }
}

fn process_mod_actions(
    app: &mut ModManager,
    mod_actions: &[ModAction],
    needs_reload: &mut bool
) {
    for action in mod_actions {
        match action {
            ModAction::RequestDeleteConfirmation(mod_id) => {
                app.mod_delete_confirmation_requested.insert(mod_id.clone(), true);
            },
            ModAction::CancelDeleteConfirmation(mod_id) => {
                app.mod_delete_confirmation_requested.remove(mod_id);
            },
            ModAction::DeleteModVersion(mod_id) => {
                delete_mod_version(app, mod_id);
                *needs_reload = true;
            },
            ModAction::UninstallMod(mod_id) => {
                if let Ok(()) = app.db.update_mod_installed(mod_id, false) {
                    *needs_reload = true;
                }
                app.mod_delete_confirmation_requested.remove(mod_id);
            },
            ModAction::ToggleModEnabled(mod_id, enabled) => {
                if let Ok(()) = app.db.update_mod_enabled(mod_id, *enabled) {
                    *needs_reload = true;
                }
            },
        }
    }
}

fn delete_mod_version(
    app: &mut ModManager,
    mod_id: &str
) {
    if let Some(mod_entry) = app.mods.iter().find(|m| m.mod_id == mod_id) {
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
    app.mod_delete_confirmation_requested.remove(mod_id);
}

fn install_mod(
    app: &mut ModManager,
    mod_id: &str,
    needs_reload: &mut bool
) {
    if let Some(mod_entry) = app.mods.iter().find(|m| m.mod_id == mod_id) {
        if let Ok(()) = app.installer.install_mod(mod_entry) {
            if let Ok(()) = app.db.update_mod_installed(mod_id, true) {
                *needs_reload = true;
            }
        }
    }
}

fn reload_mods(
    app: &mut ModManager
) {
    if let Ok(mods) = app.db.get_mods() {
        app.mods = mods;
    }
}
