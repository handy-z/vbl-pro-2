use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vbl_core::migrate::migrate_v1;
use vbl_core::persist;
use vbl_core::profile::VblSettings;

pub struct Store {
    dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct ActiveRef {
    name: String,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn default_dir() -> PathBuf {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("vbl-pro-2")
    }

    fn profiles_dir(&self) -> PathBuf {
        self.dir.join("profiles")
    }

    fn active_path(&self) -> PathBuf {
        self.dir.join("active.json")
    }

    fn profile_path(&self, name: &str) -> PathBuf {
        self.profiles_dir().join(format!("{}.json", sanitize(name)))
    }

    fn ensure_dirs(&self) {
        let _ = fs::create_dir_all(self.profiles_dir());
    }

    pub fn list_profiles(&self) -> Vec<String> {
        let mut names = Vec::new();
        if let Ok(entries) = fs::read_dir(self.profiles_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        names.push(stem.to_string());
                    }
                }
            }
        }
        names.sort();
        names
    }

    pub fn load_profile(&self, name: &str) -> Option<VblSettings> {
        let json = fs::read_to_string(self.profile_path(name)).ok()?;
        persist::deserialize_profile(&json).ok()
    }

    pub fn save_profile(&self, name: &str, settings: &VblSettings) -> std::io::Result<()> {
        self.ensure_dirs();
        let json = persist::serialize_profile(settings).map_err(std::io::Error::other)?;
        fs::write(self.profile_path(name), json)
    }

    pub fn delete_profile(&self, name: &str) -> std::io::Result<()> {
        let path = self.profile_path(name);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn active_name(&self) -> Option<String> {
        let bytes = fs::read(self.active_path()).ok()?;
        serde_json::from_slice::<ActiveRef>(&bytes)
            .ok()
            .map(|a| a.name)
    }

    pub fn set_active(&self, name: &str) -> std::io::Result<()> {
        self.ensure_dirs();
        let json = serde_json::to_vec(&ActiveRef {
            name: name.to_string(),
        })
        .map_err(std::io::Error::other)?;
        fs::write(self.active_path(), json)
    }

    pub fn load_active_or_init(&self) -> (String, VblSettings) {
        self.ensure_dirs();

        if let Some(name) = self.active_name() {
            if let Some(settings) = self.load_profile(&name) {
                return (name, settings);
            }
        }

        if let Some(settings) = try_migrate_v1() {
            let name = "imported-v1".to_string();
            let _ = self.save_profile(&name, &settings);
            let _ = self.set_active(&name);
            return (name, settings);
        }

        let name = "default".to_string();
        let settings = VblSettings::default();
        let _ = self.save_profile(&name, &settings);
        let _ = self.set_active(&name);
        (name, settings)
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn try_migrate_v1() -> Option<VblSettings> {
    let path = std::env::var_os("APPDATA")
        .map(PathBuf::from)?
        .join("com.vbl.pro")
        .join("config.json");
    let json = fs::read_to_string(path).ok()?;
    migrate_v1(&json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_store() -> Store {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        Store::new(std::env::temp_dir().join(format!(
            "vbl-store-test-{}-{}",
            std::process::id(),
            n
        )))
    }

    #[test]
    fn save_load_list_active_round_trip() {
        let store = temp_store();
        let settings = VblSettings::default();

        store.save_profile("alpha", &settings).unwrap();
        store.set_active("alpha").unwrap();

        assert!(store.list_profiles().contains(&"alpha".to_string()));
        assert_eq!(store.active_name().as_deref(), Some("alpha"));
        let loaded = store.load_profile("alpha").unwrap();
        assert_eq!(loaded.macro_keys, settings.macro_keys);
        assert_eq!(loaded.tap_ms, settings.tap_ms);

        let _ = fs::remove_dir_all(&store.dir);
    }

    #[test]
    fn init_writes_default_when_empty() {
        let store = temp_store();
        let (name, _settings) = store.load_active_or_init();

        assert!(name == "default" || name == "imported-v1");
        assert!(!store.list_profiles().is_empty());
        let _ = fs::remove_dir_all(&store.dir);
    }
}
