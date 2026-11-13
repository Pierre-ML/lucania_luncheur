fn main() {
  // Commandes de l'app appelables depuis la page : chacune génère une permission `allow-<nom>`
  // à déclarer dans capabilities/default.json.
  tauri_build::try_build(
    tauri_build::Attributes::new()
      .app_manifest(tauri_build::AppManifest::new().commands(&[
        "apps_list",
        "apps_refresh",
        "app_install",
        "app_update",
        "app_uninstall",
        "app_launch",
        "app_cancel",
        "self_check_update",
        "self_install_update",
        "self_uninstall",
        "config_get",
        "config_set",
      ])),
  )
  .unwrap()
}
