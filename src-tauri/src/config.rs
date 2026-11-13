//! Préférences de l'interface : `config.json` dans `%LOCALAPPDATA%\com.lanceur.desktop`.
//!
//! - Valeurs validées : `lang` ∈ { `fr`, `en` }, `theme` court (32 caractères au plus) et
//!   alphanumérique (tiret autorisé). `config_set` refuse une valeur invalide (`invalid_config`).
//! - Lecture : fichier absent, trop gros, illisible ou valeur invalide = valeurs par défaut (`null`).
//! - Écriture atomique : fichier temporaire puis renommage, sous un verrou.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::download::io_error;

const FILE: &str = "config.json";
const MAX_FILE_BYTES: u64 = 16 * 1024;
const MAX_THEME_LEN: usize = 32;

/// Sérialise les écritures (le fichier temporaire est unique).
static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
  #[serde(default)]
  pub lang: Option<String>,
  #[serde(default)]
  pub theme: Option<String>,
}

fn valid_lang(s: &str) -> bool {
  s == "fr" || s == "en"
}

fn valid_theme(s: &str) -> bool {
  (1..=MAX_THEME_LEN).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Écarte les valeurs invalides (lecture d'un fichier modifié à la main).
fn sanitize(c: Config) -> Config {
  Config { lang: c.lang.filter(|l| valid_lang(l)), theme: c.theme.filter(|t| valid_theme(t)) }
}

fn is_valid(c: &Config) -> bool {
  c.lang.as_deref().is_none_or(valid_lang) && c.theme.as_deref().is_none_or(valid_theme)
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
  app.path().app_local_data_dir().map(|d| d.join(FILE)).map_err(|_| "io:NotFound".to_string())
}

fn read(path: &Path) -> Config {
  let ok_size = std::fs::metadata(path).is_ok_and(|m| m.len() <= MAX_FILE_BYTES);
  ok_size
    .then(|| std::fs::read_to_string(path).ok())
    .flatten()
    .and_then(|s| serde_json::from_str::<Config>(&s).ok())
    .map(sanitize)
    .unwrap_or_default()
}

fn write(path: &Path, config: &Config) -> Result<(), String> {
  let _lock = WRITE_LOCK.lock().unwrap();
  if let Some(dir) = path.parent() {
    std::fs::create_dir_all(dir).map_err(|e| io_error(&e))?;
  }
  let json = serde_json::to_string_pretty(config).map_err(|_| "io:InvalidData".to_string())?;
  let tmp = path.with_extension("json.tmp");
  std::fs::write(&tmp, json).map_err(|e| io_error(&e))?;
  std::fs::rename(&tmp, path).map_err(|e| {
    let _ = std::fs::remove_file(&tmp);
    io_error(&e)
  })
}

#[tauri::command]
pub async fn config_get(app: tauri::AppHandle) -> Result<Config, String> {
  Ok(read(&config_path(&app)?))
}

#[tauri::command]
pub async fn config_set(app: tauri::AppHandle, config: Config) -> Result<(), String> {
  if !is_valid(&config) {
    return Err("invalid_config".into());
  }
  write(&config_path(&app)?, &config)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validation_des_valeurs() {
    assert!(is_valid(&Config { lang: Some("fr".into()), theme: Some("dark".into()) }));
    assert!(is_valid(&Config::default()));
    assert!(!is_valid(&Config { lang: Some("de".into()), theme: None }));
    assert!(!is_valid(&Config { lang: None, theme: Some("x y".into()) }));
    assert!(!is_valid(&Config { lang: None, theme: Some("t".repeat(33)) }));
    let s = sanitize(Config { lang: Some("xx".into()), theme: Some("light".into()) });
    assert_eq!(s, Config { lang: None, theme: Some("light".into()) });
  }

  #[test]
  fn lecture_et_ecriture() {
    let dir = std::env::temp_dir().join(format!("lanceur-config-{}", std::process::id()));
    let path = dir.join(FILE);
    assert_eq!(read(&path), Config::default(), "fichier absent");
    let c = Config { lang: Some("en".into()), theme: Some("dark".into()) };
    write(&path, &c).unwrap();
    assert_eq!(read(&path), c);
    std::fs::write(&path, "{ illisible").unwrap();
    assert_eq!(read(&path), Config::default());
    let _ = std::fs::remove_dir_all(&dir);
  }
}
