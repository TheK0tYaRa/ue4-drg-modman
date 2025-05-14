use crate::db::ModEntry;
use std::path::{Path, PathBuf};

pub struct ModInstaller {
    app_data_dir: PathBuf,
}

impl ModInstaller {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }
    
    pub fn install_mod(&self, mod_entry: &ModEntry) -> Result<(), String> {
        println!("Installing mod: {}", mod_entry.mod_name);
        
        // Create the download directory if it doesn't exist
        let download_dir = self.app_data_dir.join(&mod_entry.download_folder);
        std::fs::create_dir_all(&download_dir)
            .map_err(|e| format!("Failed to create download directory: {}", e))?;
        
        // Create a version-specific directory
        let version_dir = download_dir.join(&mod_entry.selected_version);
        std::fs::create_dir_all(&version_dir)
            .map_err(|e| format!("Failed to create version directory: {}", e))?;
        
        // Determine if it's a URL or file path
        let is_url = mod_entry.mod_link.starts_with("http://") || 
                     mod_entry.mod_link.starts_with("https://");
        
        if is_url {
            // Handle URL download
            self.download_from_url(mod_entry, &version_dir)
        } else {
            // Handle local file
            self.copy_local_file(mod_entry, &version_dir)
        }
    }
    
    fn download_from_url(&self, mod_entry: &ModEntry, version_dir: &Path) -> Result<(), String> {
        // TODO: Implement URL download
        println!("Would download from URL: {}", mod_entry.mod_link);
        
        // For now, just pretend it worked
        Ok(())
    }
    
    fn copy_local_file(&self, mod_entry: &ModEntry, version_dir: &Path) -> Result<(), String> {
        let source_path = std::path::Path::new(&mod_entry.mod_link);
        if !source_path.exists() {
            return Err(format!("Source file does not exist: {}", mod_entry.mod_link));
        }
        
        let file_name = source_path.file_name()
            .ok_or_else(|| "Invalid file path".to_string())?;
        
        let dest_path = version_dir.join(file_name);
        
        std::fs::copy(source_path, &dest_path)
            .map_err(|e| format!("Failed to copy mod file: {}", e))?;
        
        println!("Copied mod file to: {:?}", dest_path);
        Ok(())
    }
}