//! Commandes de l'interface sur les apps du registre : liste, rafraîchissement, installation,
//! mise à jour, désinstallation, lancement, annulation.
//!
//! - Une seule opération à la fois par app (`OpGuard`, sinon `busy`), et aucune pendant la mise à
//!   jour du Lanceur lui-même. L'opération en cours est visible dans `AppView.busy`.
//! - Seuls sont exécutés : l'installateur téléchargé depuis une URL construite à partir du registre
//!   validé (`https://github.com/<repo>/releases/latest/download/<setup|updater>`), l'exe et le
//!   désinstallateur de l'installation détectée. Arguments passés un à un, jamais de shell.
//! - Événement `app-progress` : `{ id, phase: "download" | "closing" | "running", downloaded, total }`.
//! - Erreurs : codes stables (`unknown_app`, `busy`, `not_installed`, `already_installed`,
//!   `installer_failed[:détail]` (détail : code de sortie, `elevation`, `spawn`, `timeout`,
//!   `not_detected`, `version`), `uninstall_failed[:détail]`, `still_running`, `launch_failed`,
//!   `cancelled`, plus ceux de `download` et `github`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::detect::{self, Installed};
use crate::download::{self, Cancel};
use crate::github::{self, Releases};
use crate::process;
use crate::registry::{self, AppDef, Description, Registry};

const PROGRESS_EVENT: &str = "app-progress";
/// Attente maximale d'un installateur (setup ou mise à jour).
const INSTALLER_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// Attente maximale de la disparition de la clé de désinstallation.
const UNINSTALL_TIMEOUT: Duration = Duration::from_secs(90);
/// Attente maximale de la fermeture de l'app après `WM_CLOSE`.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(10);
/// Code Windows ERROR_ELEVATION_REQUIRED : l'installateur exige les droits administrateur.
const ERROR_ELEVATION_REQUIRED: i32 = 740;

// ---------------------------------------------------------------------------------------------
// Opérations en cours

struct Op {
  kind: &'static str,
  cancel: Arc<Cancel>,
}

/// Opérations en cours, par app (état Tauri).
#[derive(Default)]
pub struct Ops(Mutex<HashMap<String, Op>>);

impl Ops {
  /// Opération en cours sur cette app : `install`, `update` ou `uninstall`.
  fn busy(&self, id: &str) -> Option<&'static str> {
    self.0.lock().unwrap().get(id).map(|op| op.kind)
  }

  /// Vrai si une opération est en cours sur au moins une app.
  pub fn any(&self) -> bool {
    !self.0.lock().unwrap().is_empty()
  }

  fn cancel(&self, id: &str) {
    if let Some(op) = self.0.lock().unwrap().get(id) {
      op.cancel.cancel();
    }
  }
}

/// Garde d'une opération : prise si l'app est libre (et le Lanceur pas en cours de mise à jour),
/// relâchée au `Drop`.
struct OpGuard<'a> {
  ops: &'a Ops,
  id: String,
  cancel: Arc<Cancel>,
}

impl<'a> OpGuard<'a> {
  fn acquire(ops: &'a Ops, id: &str, kind: &'static str, self_updating: impl FnOnce() -> bool) -> Option<Self> {
    let mut map = ops.0.lock().unwrap();
    // Vérifié sous le verrou : la mise à jour du Lanceur prend sa garde PUIS regarde `Ops::any`.
    if map.contains_key(id) || self_updating() {
      return None;
    }
    let cancel = Arc::new(Cancel::default());
    map.insert(id.to_string(), Op { kind, cancel: cancel.clone() });
    Some(Self { ops, id: id.to_string(), cancel })
  }
}

impl Drop for OpGuard<'_> {
  fn drop(&mut self) {
    self.ops.0.lock().unwrap().remove(&self.id);
  }
}

// ---------------------------------------------------------------------------------------------
// Vues

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppView {
  id: String,
  name: String,
  description: Description,
  icon: String,
  repo: String,
  installed: bool,
  installed_version: Option<String>,
  latest_version: Option<String>,
  update_available: bool,
  running: bool,
  busy: Option<String>,
  release_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
  apps: Vec<AppView>,
  warning: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
  id: &'a str,
  phase: &'static str,
  downloaded: u64,
  total: Option<u64>,
}

fn emit(app: &AppHandle, id: &str, phase: &'static str, downloaded: u64, total: Option<u64>) {
  let _ = app.emit(PROGRESS_EVENT, Progress { id, phase, downloaded, total });
}

/// Vues des apps données (aucun réseau : registre Windows, processus, cache des releases).
fn views_of(app: &AppHandle, defs: &[AppDef]) -> Vec<AppView> {
  let releases = app.state::<Releases>();
  let ops = app.state::<Ops>();
  let installed: Vec<Option<Installed>> = defs.iter().map(detect::detect).collect();
  let exes: Vec<&Path> = installed.iter().flatten().map(|i| i.exe.as_path()).collect();
  let mut running = process::pids_of_many(&exes).into_iter().map(|pids| !pids.is_empty());
  defs
    .iter()
    .zip(installed.iter())
    .map(|(def, inst)| {
      let release = releases.cached(&def.repo);
      let installed_version = inst.as_ref().and_then(|i| i.version.clone());
      let latest_version = release.as_ref().map(|r| r.version.clone());
      let update_available = matches!((&installed_version, &latest_version), (Some(i), Some(l)) if github::is_newer(l, i));
      AppView {
        id: def.id.clone(),
        name: def.name.clone(),
        description: def.description.clone(),
        icon: def.icon.clone(),
        repo: def.repo.clone(),
        installed: inst.is_some(),
        running: inst.is_some() && running.next().unwrap_or(false),
        installed_version,
        latest_version,
        update_available,
        busy: ops.busy(&def.id).map(str::to_string),
        release_url: release.and_then(|r| r.url),
      }
    })
    .collect()
}

fn views(app: &AppHandle) -> Vec<AppView> {
  views_of(app, &app.state::<Registry>().enabled())
}

fn view(app: &AppHandle, def: &AppDef) -> Result<AppView, String> {
  views_of(app, std::slice::from_ref(def)).pop().ok_or_else(|| "unknown_app".to_string())
}

fn find(app: &AppHandle, id: &str) -> Result<AppDef, String> {
  app.state::<Registry>().find(id).ok_or_else(|| "unknown_app".to_string())
}

fn release_asset_url(def: &AppDef, file: &str) -> String {
  format!("https://github.com/{}/releases/latest/download/{file}", def.repo)
}

// ---------------------------------------------------------------------------------------------
// Exécution des installateurs

/// Lance un installateur téléchargé, attend sa fin et le supprime.
async fn run_installer(path: &Path, args: &[String]) -> Result<(), String> {
  let result = async {
    let mut child = process::spawn(path, args, None).map_err(|e| match e.raw_os_error() {
      Some(ERROR_ELEVATION_REQUIRED) => "installer_failed:elevation".to_string(),
      _ => "installer_failed:spawn".to_string(),
    })?;
    match process::wait_child(&mut child, INSTALLER_TIMEOUT).await {
      Some(0) => Ok(()),
      Some(code) => Err(format!("installer_failed:{code}")),
      None => Err("installer_failed:timeout".to_string()),
    }
  }
  .await;
  let _ = tokio::fs::remove_file(path).await;
  result
}

/// Télécharge un fichier de la dernière release de l'app vers `%TEMP%\Lanceur\<id>\`.
async fn download_asset(app: &AppHandle, def: &AppDef, file: &str, cancel: &Cancel) -> Result<PathBuf, String> {
  let url = release_asset_url(def, file);
  let dir = download::work_dir(&def.id);
  let id = def.id.as_str();
  download::download(&url, &dir, file, def.max_bytes, Some(cancel), |d, t| emit(app, id, "download", d, t)).await
}

/// Supprime un dossier de données, en réessayant : le désinstallateur (copie dans %TEMP%) peut
/// encore être en train de le vider.
async fn remove_data_dir(dir: PathBuf) {
  for _ in 0..5 {
    if !dir.exists() {
      return;
    }
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
  }
}

// ---------------------------------------------------------------------------------------------
// Commandes

/// Liste des apps (aucun réseau).
#[tauri::command]
pub async fn apps_list(app: AppHandle) -> Result<Vec<AppView>, String> {
  Ok(views(&app))
}

/// Rafraîchit le registre distant (cache 10 min) et la dernière release de chaque app
/// (cache 5 min) ; `force` ignore les caches. `warning` : `offline` ou `rate_limited`.
#[tauri::command]
pub async fn apps_refresh(app: AppHandle, force: bool) -> Result<RefreshResult, String> {
  let mut warnings: Vec<String> = Vec::new();
  if app.state::<Registry>().needs_refresh(force) {
    match github::fetch_text(registry::REMOTE_URL, registry::REMOTE_MAX_BYTES).await {
      Ok(text) => {
        app.state::<Registry>().apply_remote(&text);
      }
      Err(e) if e == "offline" || e == "rate_limited" => warnings.push(e),
      Err(_) => {}
    }
  }
  let mut repos: Vec<String> = app.state::<Registry>().enabled().into_iter().map(|a| a.repo).collect();
  let mut seen = HashSet::new();
  repos.retain(|r| seen.insert(r.to_ascii_lowercase()));
  if let Some(w) = app.state::<Releases>().refresh(&repos, force).await {
    warnings.push(w);
  }
  let warning = ["rate_limited", "offline"].into_iter().find(|w| warnings.iter().any(|x| x == w)).map(str::to_string);
  Ok(RefreshResult { apps: views(&app), warning })
}

/// Première installation : télécharge `<setup>`, l'exécute avec `installArgs`, vérifie la détection.
#[tauri::command]
pub async fn app_install(app: AppHandle, id: String) -> Result<AppView, String> {
  let def = find(&app, &id)?;
  {
    let ops = app.state::<Ops>();
    let guard = OpGuard::acquire(&ops, &def.id, "install", crate::selfupdate::in_progress).ok_or_else(|| "busy".to_string())?;
    if detect::detect(&def).is_some() {
      return Err("already_installed".into());
    }
    let path = download_asset(&app, &def, &def.setup, &guard.cancel).await?;
    if guard.cancel.is_cancelled() {
      let _ = tokio::fs::remove_file(&path).await;
      return Err("cancelled".into());
    }
    emit(&app, &def.id, "running", 0, None);
    run_installer(&path, &def.install_args).await?;
    if detect::detect(&def).is_none() {
      return Err("installer_failed:not_detected".into());
    }
  }
  view(&app, &def)
}

/// Mise à jour : télécharge `<updater>`, ferme proprement l'app, exécute l'updater avec
/// `updateArgs` (+ `waitPidArg<pid>` si l'app tourne encore), vérifie la nouvelle version.
#[tauri::command]
pub async fn app_update(app: AppHandle, id: String) -> Result<AppView, String> {
  let def = find(&app, &id)?;
  {
    let ops = app.state::<Ops>();
    let guard = OpGuard::acquire(&ops, &def.id, "update", crate::selfupdate::in_progress).ok_or_else(|| "busy".to_string())?;
    let before = detect::detect(&def).ok_or_else(|| "not_installed".to_string())?;
    let path = download_asset(&app, &def, &def.updater, &guard.cancel).await?;
    if guard.cancel.is_cancelled() {
      let _ = tokio::fs::remove_file(&path).await;
      return Err("cancelled".into());
    }

    emit(&app, &def.id, "closing", 0, None);
    let alive = process::close_gracefully(&before.exe, CLOSE_TIMEOUT).await;
    let mut args = def.update_args.clone();
    if let Some(pid) = alive.first() {
      match &def.wait_pid_arg {
        Some(arg) => args.push(format!("{arg}{pid}")),
        None => {
          let _ = tokio::fs::remove_file(&path).await;
          return Err("still_running".into());
        }
      }
    }

    emit(&app, &def.id, "running", 0, None);
    run_installer(&path, &args).await?;
    let after = detect::detect(&def).ok_or_else(|| "installer_failed:not_detected".to_string())?;
    let latest = app.state::<Releases>().cached(&def.repo).map(|r| r.version);
    let changed = after.version != before.version;
    let reached = matches!((&latest, &after.version), (Some(l), Some(v)) if !github::is_newer(l, v));
    if !changed && !reached {
      return Err("installer_failed:version".into());
    }
  }
  view(&app, &def)
}

/// Désinstallation. `delete_data == false` : `/S` + `keepDataArg` (données conservées), puis
/// suppression des raccourcis laissés par ce mode. `delete_data == true` : `/S` seul, puis
/// suppression de `%APPDATA%\<identifier>` et `%LOCALAPPDATA%\<identifier>` s'ils existent encore.
#[tauri::command]
pub async fn app_uninstall(app: AppHandle, id: String, delete_data: bool) -> Result<AppView, String> {
  let def = find(&app, &id)?;
  {
    let ops = app.state::<Ops>();
    let _guard = OpGuard::acquire(&ops, &def.id, "uninstall", crate::selfupdate::in_progress).ok_or_else(|| "busy".to_string())?;
    let inst = detect::detect(&def).ok_or_else(|| "not_installed".to_string())?;
    let uninstaller = inst.uninstaller().ok_or_else(|| "uninstall_failed".to_string())?;

    emit(&app, &def.id, "closing", 0, None);
    if !process::close_gracefully(&inst.exe, CLOSE_TIMEOUT).await.is_empty() {
      return Err("still_running".into());
    }

    emit(&app, &def.id, "running", 0, None);
    let mut args = vec!["/S".to_string()];
    if !delete_data && let Some(arg) = &def.keep_data_arg {
      args.push(arg.clone());
    }
    // Répertoire courant hors du dossier d'installation (qui doit pouvoir être supprimé).
    let started = Instant::now();
    let mut child = process::spawn(&uninstaller, &args, Some(&std::env::temp_dir())).map_err(|_| "uninstall_failed:spawn".to_string())?;
    // Le désinstallateur NSIS se recopie dans %TEMP% et rend la main aussitôt : seule la
    // disparition de la clé de registre indique la fin.
    if let Some(code) = process::wait_child(&mut child, UNINSTALL_TIMEOUT).await
      && code != 0
    {
      return Err(format!("uninstall_failed:{code}"));
    }
    while detect::key_exists(&def) {
      if started.elapsed() >= UNINSTALL_TIMEOUT {
        return Err("uninstall_failed:timeout".into());
      }
      tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let paths = app.path();
    if delete_data {
      for base in [paths.data_dir(), paths.local_data_dir()].into_iter().flatten() {
        remove_data_dir(base.join(&def.identifier)).await;
      }
    } else {
      detect::remove_leftovers(&def, paths.data_dir().ok().as_deref(), paths.desktop_dir().ok().as_deref(), &inst.exe);
    }
  }
  view(&app, &def)
}

/// Lance l'exe installé (détaché, répertoire courant = dossier d'installation).
#[tauri::command]
pub async fn app_launch(app: AppHandle, id: String) -> Result<(), String> {
  let def = find(&app, &id)?;
  if app.state::<Ops>().busy(&def.id).is_some() {
    return Err("busy".into());
  }
  let inst = detect::detect(&def).ok_or_else(|| "not_installed".to_string())?;
  process::spawn(&inst.exe, &[], Some(&inst.dir)).map(drop).map_err(|_| "launch_failed".to_string())
}

/// Annule le téléchargement en cours de cette app (sans effet une fois l'installateur lancé).
#[tauri::command]
pub async fn app_cancel(app: AppHandle, id: String) -> Result<(), String> {
  let def = find(&app, &id)?;
  app.state::<Ops>().cancel(&def.id);
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn une_operation_a_la_fois_par_app() {
    let ops = Ops::default();
    let first = OpGuard::acquire(&ops, "demo", "install", || false).expect("première prise");
    assert!(OpGuard::acquire(&ops, "demo", "update", || false).is_none());
    assert_eq!(ops.busy("demo"), Some("install"));
    let other = OpGuard::acquire(&ops, "autre", "uninstall", || false).expect("autre app libre");
    assert!(ops.any());
    drop(first);
    assert_eq!(ops.busy("demo"), None);
    drop(other);
    assert!(!ops.any());
    assert!(OpGuard::acquire(&ops, "demo", "install", || true).is_none(), "Lanceur en cours de mise à jour");
  }

  #[test]
  fn annulation_par_app() {
    let ops = Ops::default();
    let guard = OpGuard::acquire(&ops, "demo", "install", || false).unwrap();
    ops.cancel("autre");
    assert!(!guard.cancel.is_cancelled());
    ops.cancel("demo");
    assert!(guard.cancel.is_cancelled());
  }

  #[test]
  fn url_des_fichiers_de_release() {
    let def = Registry::new().find("lucaia").expect("Luca-ia activée dans apps.json");
    assert_eq!(release_asset_url(&def, &def.setup), "https://github.com/Pierre-ML/Luca-ia/releases/latest/download/Luca-ia-Setup.exe");
    assert!(github::allowed_host(&reqwest::Url::parse(&release_asset_url(&def, &def.updater)).unwrap()));
  }
}
