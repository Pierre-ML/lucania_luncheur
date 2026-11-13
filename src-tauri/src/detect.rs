//! Détection des installations dans le registre Windows (installateurs NSIS de Tauri, mode
//! `currentUser`) : clé `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\<productName>`.
//!
//! - L'app est « installée » si la clé existe ET si `InstallLocation` (écrit entre guillemets par le
//!   template) est un chemin absolu dont `<InstallLocation>\<exe du registre des apps>` existe.
//! - Le désinstallateur exécuté n'est jamais la chaîne `UninstallString` brute : on en extrait le
//!   chemin, qui doit être exactement `<InstallLocation>\uninstall.exe` et exister ; c'est alors le
//!   chemin CONSTRUIT qui est lancé.
//! - Nettoyage après une désinstallation avec `/UPDATE` (qui laisse raccourcis et démarrage auto) :
//!   raccourcis `<productName>.lnk` du Menu Démarrer et du Bureau (seulement s'ils visent l'exe de
//!   l'app) et valeur `HKCU\…\Run\<productName>` (seulement si elle cite l'exe installé).

use std::path::{Component, Path, PathBuf};

use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
  HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_REG_SZ, RegCloseKey, RegDeleteKeyValueW, RegGetValueW, RegOpenKeyExW,
};

use crate::registry::AppDef;

const UNINSTALL_ROOT: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// Nom du désinstallateur écrit par le template (`WriteUninstaller "$INSTDIR\uninstall.exe"`).
const UNINSTALLER: &str = "uninstall.exe";
const MAX_VERSION_LEN: usize = 64;

/// Installation détectée.
#[derive(Debug, Clone)]
pub struct Installed {
  pub dir: PathBuf,
  pub exe: PathBuf,
  pub version: Option<String>,
  uninstall_string: Option<String>,
}

fn wide(s: &str) -> Vec<u16> {
  s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn uninstall_key(product_name: &str) -> String {
  format!(r"{UNINSTALL_ROOT}\{product_name}")
}

/// Lit une valeur texte de `HKCU\<subkey>` (REG_EXPAND_SZ développée) ; `None` si absente.
fn reg_string(subkey: &str, value: &str) -> Option<String> {
  let (subkey, value) = (wide(subkey), wide(value));
  let mut buf: Vec<u16> = vec![0; 256];
  for _ in 0..4 {
    let mut bytes = (buf.len() * 2) as u32;
    // SAFETY : chaînes terminées par NUL, tampon de `bytes` octets.
    let status = unsafe {
      RegGetValueW(
        HKEY_CURRENT_USER,
        subkey.as_ptr(),
        value.as_ptr(),
        RRF_RT_REG_SZ,
        std::ptr::null_mut(),
        buf.as_mut_ptr().cast(),
        &mut bytes,
      )
    };
    if status == ERROR_MORE_DATA {
      buf = vec![0; (bytes as usize).div_ceil(2) + 1];
      continue;
    }
    if status != ERROR_SUCCESS {
      return None;
    }
    let len = (bytes as usize / 2).min(buf.len());
    let s = String::from_utf16_lossy(&buf[..len]);
    return Some(s.trim_end_matches('\0').to_string());
  }
  None
}

/// Vrai si la clé `HKCU\<subkey>` existe.
fn reg_key_exists(subkey: &str) -> bool {
  let subkey = wide(subkey);
  let mut key: HKEY = std::ptr::null_mut();
  // SAFETY : chaîne terminée par NUL ; la clé ouverte est refermée aussitôt.
  unsafe {
    if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut key) == ERROR_SUCCESS {
      RegCloseKey(key);
      true
    } else {
      false
    }
  }
}

/// Vrai tant que la clé de désinstallation de l'app existe.
pub fn key_exists(app: &AppDef) -> bool {
  reg_key_exists(&uninstall_key(&app.product_name))
}

/// Retire espaces et guillemets autour d'une valeur du registre.
fn unquote(raw: &str) -> &str {
  let s = raw.trim();
  s.strip_prefix('"').and_then(|r| r.strip_suffix('"')).unwrap_or(s).trim()
}

/// `InstallLocation` → dossier absolu, sans `\` final ni composant `..` ; `None` sinon.
fn parse_install_location(raw: &str) -> Option<PathBuf> {
  let s = unquote(raw).trim_end_matches(['\\', '/']);
  if s.is_empty() || s.contains('"') {
    return None;
  }
  let path = PathBuf::from(s);
  (path.is_absolute() && !path.components().any(|c| matches!(c, Component::ParentDir))).then_some(path)
}

/// Chemin contenu dans `UninstallString` : la partie entre guillemets s'il y en a, sinon la chaîne
/// entière (des arguments non séparés par des guillemets la rendent alors invalide).
fn extract_uninstaller(raw: &str) -> Option<&str> {
  let s = raw.trim();
  let path = match s.strip_prefix('"') {
    Some(rest) => &rest[..rest.find('"')?],
    None => s,
  };
  (!path.is_empty()).then_some(path)
}

/// Comparaison de chemins Windows : casse, `/` et `\` final indifférents.
pub fn same_path(a: &Path, b: &Path) -> bool {
  let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
  norm(a) == norm(b)
}

/// Désinstallateur à exécuter : `<dossier>\uninstall.exe`, à condition que `UninstallString` le
/// désigne exactement et qu'il existe.
fn checked_uninstaller(dir: &Path, uninstall_string: &str) -> Option<PathBuf> {
  let expected = dir.join(UNINSTALLER);
  let declared = extract_uninstaller(uninstall_string)?;
  (same_path(Path::new(declared), &expected) && expected.is_file()).then_some(expected)
}

impl Installed {
  /// Désinstallateur vérifié (voir `checked_uninstaller`).
  pub fn uninstaller(&self) -> Option<PathBuf> {
    checked_uninstaller(&self.dir, self.uninstall_string.as_deref()?)
  }
}

/// État d'installation d'une app ; `None` si elle n'est pas (correctement) installée.
pub fn detect(app: &AppDef) -> Option<Installed> {
  let key = uninstall_key(&app.product_name);
  if !reg_key_exists(&key) {
    return None;
  }
  let dir = parse_install_location(&reg_string(&key, "InstallLocation")?)?;
  let exe = dir.join(&app.exe);
  if !exe.is_file() {
    return None;
  }
  let version = reg_string(&key, "DisplayVersion")
    .map(|v| v.trim().to_string())
    .filter(|v| !v.is_empty() && v.len() <= MAX_VERSION_LEN && !v.chars().any(char::is_control));
  Some(Installed { dir, exe, version, uninstall_string: reg_string(&key, "UninstallString") })
}

/// Vrai si le contenu d'un `.lnk` ressemble à un raccourci vers un fichier nommé `exe_name`
/// (en-tête Shell Link, nom présent en ASCII ou en UTF-16, casse indifférente). Sans COM : sert
/// seulement à ne pas supprimer un raccourci homonyme qui viserait autre chose.
fn lnk_targets(bytes: &[u8], exe_name: &str) -> bool {
  const HEADER: [u8; 8] = [0x4C, 0, 0, 0, 0x01, 0x14, 0x02, 0x00];
  if !bytes.starts_with(&HEADER) {
    return false;
  }
  let lower: Vec<u8> = bytes.iter().map(u8::to_ascii_lowercase).collect();
  let name = exe_name.to_ascii_lowercase();
  let ascii = name.as_bytes();
  let utf16: Vec<u8> = name.bytes().flat_map(|b| [b, 0]).collect();
  let contains = |needle: &[u8]| lower.windows(needle.len()).any(|w| w == needle);
  contains(ascii) || contains(&utf16)
}

/// Supprime `<dir>\<productName>.lnk` s'il vise l'exe de l'app.
fn remove_shortcut(dir: &Path, app: &AppDef) {
  let lnk = dir.join(format!("{}.lnk", app.product_name));
  let Ok(meta) = std::fs::metadata(&lnk) else { return };
  if meta.len() > 1024 * 1024 {
    return;
  }
  if std::fs::read(&lnk).is_ok_and(|b| lnk_targets(&b, &app.exe)) {
    let _ = std::fs::remove_file(&lnk);
  }
}

/// Ce que le désinstallateur laisse avec `/UPDATE` (voir `Section Uninstall` du template) :
/// raccourcis Menu Démarrer (`$SMPROGRAMS\<P>.lnk` et `$SMPROGRAMS\<P>\<P>.lnk`), Bureau
/// (`$DESKTOP\<P>.lnk`) et démarrage automatique (`HKCU\…\Run`, valeur `<P>`).
/// `roaming` = `%APPDATA%`, `desktop` = Bureau de l'utilisateur.
pub fn remove_leftovers(app: &AppDef, roaming: Option<&Path>, desktop: Option<&Path>, exe: &Path) {
  if let Some(roaming) = roaming {
    let programs = roaming.join(r"Microsoft\Windows\Start Menu\Programs");
    remove_shortcut(&programs, app);
    let folder = programs.join(&app.product_name);
    remove_shortcut(&folder, app);
    let _ = std::fs::remove_dir(&folder); // seulement s'il est vide
  }
  if let Some(desktop) = desktop {
    remove_shortcut(desktop, app);
  }
  let exe_text = exe.to_string_lossy().to_lowercase();
  if reg_string(RUN_KEY, &app.product_name).is_some_and(|v| v.to_lowercase().contains(&exe_text)) {
    let (key, value) = (wide(RUN_KEY), wide(&app.product_name));
    // SAFETY : chaînes terminées par NUL.
    unsafe {
      RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr());
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dossier_d_installation() {
    let p = parse_install_location("\"C:\\Programmes\\Demo\"").unwrap();
    assert_eq!(p, PathBuf::from(r"C:\Programmes\Demo"));
    assert_eq!(parse_install_location(r"C:\Programmes\Demo\").unwrap(), PathBuf::from(r"C:\Programmes\Demo"));
    for bad in ["", "\"\"", r"Demo", r"\Demo", r"C:Demo", r"C:\a\..\b", "\"C:\\a\"b\""] {
      assert!(parse_install_location(bad).is_none(), "accepté : {bad}");
    }
  }

  #[test]
  fn extraction_du_desinstallateur() {
    assert_eq!(extract_uninstaller("\"C:\\A B\\uninstall.exe\""), Some(r"C:\A B\uninstall.exe"));
    assert_eq!(extract_uninstaller("\"C:\\A B\\uninstall.exe\" /S"), Some(r"C:\A B\uninstall.exe"));
    assert_eq!(extract_uninstaller(r"C:\A\uninstall.exe"), Some(r"C:\A\uninstall.exe"));
    assert_eq!(extract_uninstaller("\"C:\\A\\uninstall.exe"), None);
    assert_eq!(extract_uninstaller("  "), None);
  }

  #[test]
  fn desinstallateur_impose() {
    let dir = std::env::temp_dir().join(format!("lanceur-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let real = dir.join(UNINSTALLER);
    std::fs::write(&real, b"MZ").unwrap();
    let quoted = format!("\"{}\"", real.display());
    assert_eq!(checked_uninstaller(&dir, &quoted), Some(real.clone()));
    let upper = format!("\"{}\"", real.display().to_string().to_uppercase());
    assert_eq!(checked_uninstaller(&dir, &upper), Some(real.clone()), "casse indifférente");
    // Autre exe, arguments collés, chemin hors du dossier : refusés.
    assert!(checked_uninstaller(&dir, &format!("\"{}\"", dir.join("evil.exe").display())).is_none());
    assert!(checked_uninstaller(&dir, &format!("{} /S", real.display())).is_none());
    assert!(checked_uninstaller(&dir, r"C:\Windows\System32\cmd.exe").is_none());
    std::fs::remove_file(&real).unwrap();
    assert!(checked_uninstaller(&dir, &quoted).is_none(), "fichier absent");
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn comparaison_de_chemins() {
    assert!(same_path(Path::new(r"C:\A\b.exe"), Path::new("c:/a/B.EXE")));
    assert!(same_path(Path::new(r"C:\A\"), Path::new(r"C:\A")));
    assert!(!same_path(Path::new(r"C:\A\b.exe"), Path::new(r"C:\A\b.exe.bak")));
  }

  #[test]
  fn cible_d_un_raccourci() {
    let mut lnk = vec![0x4C, 0, 0, 0, 0x01, 0x14, 0x02, 0x00, 1, 2, 3];
    lnk.extend("C:\\X\\Demo.EXE".encode_utf16().flat_map(|u| u.to_le_bytes()));
    assert!(lnk_targets(&lnk, "demo.exe"));
    assert!(!lnk_targets(&lnk, "autre.exe"));
    assert!(!lnk_targets(b"demo.exe", "demo.exe"), "pas un raccourci");
  }
}
