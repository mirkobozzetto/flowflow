//! One JSON record per service in the Apple keychain. A file replaces the
//! keychain when `path_env` is set (tests) and off Apple platforms.
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub struct TokenStore {
    pub service: &'static str,
    pub account: &'static str,
    pub path_env: &'static str,
    pub desktop_file: &'static str,
}

impl TokenStore {
    pub fn load<T: DeserializeOwned>(&self) -> Result<Option<T>, String> {
        match self.override_path() {
            Some(path) => load_file(&path),
            None => self.platform_load(),
        }
    }

    pub fn save<T: Serialize>(&self, record: &T) -> Result<(), String> {
        match self.override_path() {
            Some(path) => save_file(&path, record),
            None => self.platform_save(record),
        }
    }

    pub fn delete(&self) {
        match self.override_path() {
            Some(path) => {
                let _ = std::fs::remove_file(path);
            }
            None => self.platform_delete(),
        }
    }

    fn override_path(&self) -> Option<PathBuf> {
        std::env::var_os(self.path_env)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn platform_load<T: DeserializeOwned>(&self) -> Result<Option<T>, String> {
        const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
        match security_framework::passwords::get_generic_password(
            self.service,
            self.account,
        ) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| error.to_string()),
            Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn platform_save<T: Serialize>(&self, record: &T) -> Result<(), String> {
        let bytes =
            serde_json::to_vec(record).map_err(|error| error.to_string())?;
        security_framework::passwords::set_generic_password(
            self.service,
            self.account,
            &bytes,
        )
        .map_err(|error| error.to_string())
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn platform_delete(&self) {
        let _ = security_framework::passwords::delete_generic_password(
            self.service,
            self.account,
        );
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn platform_load<T: DeserializeOwned>(&self) -> Result<Option<T>, String> {
        load_file(&self.desktop_path())
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn platform_save<T: Serialize>(&self, record: &T) -> Result<(), String> {
        save_file(&self.desktop_path(), record)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn platform_delete(&self) {
        let _ = std::fs::remove_file(self.desktop_path());
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn desktop_path(&self) -> PathBuf {
        crate::infrastructure::persistence::desktop_data_dir()
            .join(self.desktop_file)
    }
}

fn load_file<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| error.to_string()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn save_file<T: Serialize>(path: &Path, record: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes =
        serde_json::to_vec(record).map_err(|error| error.to_string())?;
    std::fs::write(path, bytes).map_err(|error| error.to_string())
}
