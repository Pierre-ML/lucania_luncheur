//! Désinstallation du Lanceur lui-même : lance `uninstall.exe` (écrit par l'installateur NSIS à
//! côté de l'exe, voir src-tauri/windows/installer.nsi) puis ferme l'application pour que ses
//! fichiers ne soient pas verrouillés.
//!
//! - `self_uninstall` : refusé en dev (`dev_mode`, désinstalleur absent), pendant une opération sur
//!   une app ou une mise à jour du Lanceur (`busy`). Le désinstalleur est lancé SANS `/S` : c'est
//!   lui qui demande la confirmation finale et propose de garder ou supprimer les données.
//! - `app.exit(0)` après 300 ms ; filet : terminaison forcée 10 s plus tard.

use std::path::PathBuf;
use std::time::Duration;

use tauri::Manager;

use crate::actions::Ops;
use crate::{process, selfupdate};

/// Nom du désinstalleur écrit par `WriteUninstaller` dans le dossier d'installation.
const UNINSTALLER_FILE: &str = "uninstall.exe";
const EXIT_DELAY: Duration = Duration::from_millis(300);
const EXIT_FALLBACK: Duration = Duration::from_secs(10);

/// Chemin du désinstalleur à côté de l'exe courant ; `None` s'il est absent (ex. `tauri dev`).
fn uninstaller_path() -> Option<PathBuf> {
  let exe = std::env::current_exe().ok()?;
  let path = exe.parent()?.join(UNINSTALLER_FILE);
  path.is_file().then_some(path)
}

/// Lance le désinstalleur (processus détaché, interface normale) puis ferme le Lanceur.
#[tauri::command]
pub async fn self_uninstall(app: tauri::AppHandle) -> Result<(), String> {
  if selfupdate::in_progress() || app.state::<Ops>().any() {
    return Err("busy".into());
  }
  let path = uninstaller_path().ok_or_else(|| "dev_mode".to_string())?;
  process::spawn(&path, &[], path.parent()).map_err(|_| "launch_failed".to_string())?;

  std::thread::spawn(move || {
    std::thread::sleep(EXIT_DELAY);
    app.exit(0);
    std::thread::sleep(EXIT_FALLBACK);
    std::process::exit(0);
  });
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn desinstalleur_absent_hors_installation() {
    // L'exe de test n'a pas de uninstall.exe à côté de lui.
    assert!(uninstaller_path().is_none());
  }
}
