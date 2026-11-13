//! Mise à jour du Lanceur lui-même (même mécanisme que l'updater de Lucania) : compare la version
//! installée à la dernière release GitHub, puis télécharge et lance `Lucania-Luncheur-Update.exe`.
//!
//! - `self_check_update` : interroge l'API GitHub (release la plus récente de `Pierre-ML/lucania_luncheur`).
//! - `self_install_update` : refusé en dev (`dev_mode`) et pendant une opération sur une app
//!   (`busy`). Téléchargement en flux (300 Mo maximum, contrôle « MZ ») avec l'événement
//!   `self-update-progress` (`{ downloaded, total }`, au plus ~10 par seconde, puis un dernier à
//!   100 %), puis lancement DIRECT de l'updater (exe graphique NSIS, aucun `cmd`) avec
//!   `/LANCEUR_WAITPID=<pid>` : son `.onInit` attend la fin de ce processus avant de copier les
//!   fichiers. `app.exit(0)` après 300 ms ; filet : si la sortie n'a pas abouti 10 s plus tard,
//!   le processus se termine lui-même (`TerminateProcess`).
//! - Une garde atomique (`INSTALLING`) rend impossible un second lancement ; elle n'est relâchée
//!   qu'en cas d'échec avant le lancement. Tant qu'elle est prise, aucune opération sur une app
//!   ne peut démarrer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::actions::Ops;
use crate::{download, github, process};

const REPO: &str = "Pierre-ML/lucania_luncheur";
const UPDATER_URL: &str = "https://github.com/Pierre-ML/lucania_luncheur/releases/latest/download/Lucania-Luncheur-Update.exe";
const UPDATER_FILE: &str = "Lucania-Luncheur-Update.exe";
/// Taille maximale acceptée pour l'exe de mise à jour.
const MAX_BYTES: u64 = 300 * 1024 * 1024;
const PROGRESS_EVENT: &str = "self-update-progress";
/// Argument lu par le `.onInit` de l'updater (src-tauri/windows/installer.nsi) : PID à attendre.
const WAIT_PID_ARG: &str = "/LANCEUR_WAITPID=";
/// Délai avant `app.exit(0)` (réponse à la page), puis délai du filet de terminaison forcée.
const EXIT_DELAY: Duration = Duration::from_millis(300);
const EXIT_FALLBACK: Duration = Duration::from_secs(10);

/// Vrai dès qu'une mise à jour du Lanceur est en cours (téléchargement ou lancement).
static INSTALLING: AtomicBool = AtomicBool::new(false);

/// Vrai pendant une mise à jour du Lanceur (les opérations sur les apps sont alors refusées).
pub fn in_progress() -> bool {
  INSTALLING.load(Ordering::SeqCst)
}

/// Prend la garde ; `None` si une installation est déjà en cours. Relâchée au `Drop`, sauf après `keep()`.
struct InstallGuard<'a> {
  flag: &'a AtomicBool,
  release: bool,
}

impl<'a> InstallGuard<'a> {
  fn acquire(flag: &'a AtomicBool) -> Option<Self> {
    flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).ok()?;
    Some(Self { flag, release: true })
  }

  /// L'installateur est lancé : la garde reste prise jusqu'à la fermeture de l'app.
  fn keep(mut self) {
    self.release = false;
  }
}

impl Drop for InstallGuard<'_> {
  fn drop(&mut self) {
    if self.release {
      self.flag.store(false, Ordering::SeqCst);
    }
  }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
  downloaded: u64,
  total: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
  pub current: String,
  pub latest: String,
  pub available: bool,
  pub url: String,
}

/// Interroge GitHub et indique si une version plus récente que celle installée est publiée.
#[tauri::command]
pub async fn self_check_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
  let current = app.package_info().version.to_string();
  let release = github::latest_release(REPO).await?;
  Ok(UpdateInfo {
    available: github::is_newer(&release.version, &current),
    current,
    latest: release.version,
    url: release.url.unwrap_or_default(),
  })
}

/// Télécharge l'exe de mise à jour, le lance (processus détaché) puis ferme le Lanceur.
#[tauri::command]
pub async fn self_install_update(app: tauri::AppHandle) -> Result<(), String> {
  if cfg!(debug_assertions) {
    return Err("dev_mode".into());
  }
  let guard = InstallGuard::acquire(&INSTALLING).ok_or_else(|| "busy".to_string())?;
  // Garde prise AVANT ce contrôle : aucune opération sur une app ne peut plus démarrer.
  if app.state::<Ops>().any() {
    return Err("busy".into());
  }

  let dir = download::work_dir("self");
  let path = download::download(UPDATER_URL, &dir, UPDATER_FILE, MAX_BYTES, None, |downloaded, total| {
    let _ = app.emit(PROGRESS_EVENT, Progress { downloaded, total });
  })
  .await?;

  process::spawn(&path, &[wait_pid_arg(std::process::id())], None).map_err(|_| "launch_failed".to_string())?;

  // Lancé : la garde reste prise (aucun second lancement possible) jusqu'à la fermeture.
  guard.keep();
  std::thread::spawn(move || {
    std::thread::sleep(EXIT_DELAY);
    app.exit(0);
    // Sortie normale : le processus disparaît avant la fin de ce délai. Sinon (sortie bloquée),
    // il se termine lui-même pour que l'updater, qui attend sa fin, puisse copier les fichiers.
    std::thread::sleep(EXIT_FALLBACK);
    terminate_self();
  });
  Ok(())
}

/// Argument de l'updater : PID du Lanceur, dont il attend la fin avant de copier les fichiers.
fn wait_pid_arg(pid: u32) -> String {
  format!("{WAIT_PID_ARG}{pid}")
}

/// Filet de sécurité : termine immédiatement le processus (sans destructeurs ni `atexit`).
fn terminate_self() -> ! {
  use windows_sys::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
  // SAFETY : pseudo-handle du processus courant, toujours valide.
  unsafe {
    TerminateProcess(GetCurrentProcess(), 0);
  }
  std::process::exit(0)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn garde_d_installation_unique() {
    let flag = AtomicBool::new(false);
    let first = InstallGuard::acquire(&flag).expect("première prise");
    assert!(InstallGuard::acquire(&flag).is_none());
    drop(first); // échec avant lancement : garde relâchée
    let again = InstallGuard::acquire(&flag).expect("reprise après échec");
    again.keep(); // installateur lancé : garde conservée
    assert!(InstallGuard::acquire(&flag).is_none());
  }

  #[test]
  fn argument_d_attente_de_l_updater() {
    // Contrat avec le .onInit de src-tauri/windows/installer.nsi (GetOptions "/LANCEUR_WAITPID=")
    assert_eq!(wait_pid_arg(1234), "/LANCEUR_WAITPID=1234");
    assert!(!wait_pid_arg(7).contains(' '));
  }
}
