use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub db_path: PathBuf,
    pub diffs_dir: PathBuf,
}

impl StorageConfig {
    pub fn default_paths() -> Option<StorageConfig> {
        let proj_dirs = ProjectDirs::from("", "", "dromos")?;
        let data_dir = proj_dirs.data_dir();

        Some(StorageConfig {
            db_path: data_dir.join("dromos.db"),
            diffs_dir: data_dir.join("diffs"),
        })
    }

    pub fn ensure_dirs_exist(&self) -> std::io::Result<()> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(&self.diffs_dir)?;
        Ok(())
    }
}
