//! Lanceur : installe, met à jour, désinstalle et lance les applications listées dans `apps.json`.
//!
//! Modules :
//! - `registry` : registre des apps (embarqué + distant) et validation stricte des entrées ;
//! - `detect` : installations détectées dans le registre Windows (clé Uninstall) ;
//! - `github` : client HTTP (hôtes autorisés), dernières releases (cache), registre distant ;
//! - `download` : téléchargement en flux annulable vers `%TEMP%\Lanceur` ;
//! - `process` : processus en cours, fermeture propre (`WM_CLOSE`), lancement détaché ;
//! - `actions` : commandes install / update / uninstall / launch / cancel ;
//! - `selfupdate` : mise à jour du Lanceur lui-même ;
//! - `selfuninstall` : désinstallation du Lanceur lui-même ;
//! - `config` : préférences de l'interface (langue, thème).

#[cfg(not(windows))]
compile_error!("Le Lanceur ne cible que Windows (registre, processus et installateurs NSIS).");

use tauri::Manager;

mod actions;
mod config;
mod detect;
mod download;
mod github;
mod process;
mod registry;
mod selfuninstall;
mod selfupdate;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let builder = tauri::Builder::default();

  // Instance unique : doit être le PREMIER plugin enregistré. Une 2e instance transmet ses
  // arguments à la première (qui revient au premier plan) puis se termine avant setup.
  #[cfg(desktop)]
  let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
    if let Some(w) = app.get_webview_window("main") {
      let _ = w.unminimize();
      let _ = w.show();
      let _ = w.set_focus();
    }
  }));

  builder
    .manage(registry::Registry::new())
    .manage(github::Releases::default())
    .manage(actions::Ops::default())
    .invoke_handler(tauri::generate_handler![
      actions::apps_list,
      actions::apps_refresh,
      actions::app_install,
      actions::app_update,
      actions::app_uninstall,
      actions::app_launch,
      actions::app_cancel,
      selfupdate::self_check_update,
      selfupdate::self_install_update,
      selfuninstall::self_uninstall,
      config::config_get,
      config::config_set,
    ])
    .setup(|_app| {
      #[cfg(debug_assertions)]
      if let Some(w) = _app.get_webview_window("main") {
        w.open_devtools();
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
