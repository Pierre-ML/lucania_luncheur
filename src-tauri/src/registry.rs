//! Registre des applications : liste embarquée (`apps.json` à la racine du dépôt, `include_str!`)
//! remplacée, si elle est valide, par la version distante publiée sur la branche `main`.
//!
//! - Chaque entrée est validée à la main (aucune crate regex) ; une entrée invalide est ignorée.
//!   Tout ce qui sert à construire une URL, un chemin, une clé de registre ou un argument
//!   d'installateur passe par ces validations : rien d'autre n'est jamais exécuté.
//! - Registre distant (`REMOTE_URL`, 256 Ko maximum, cache mémoire 10 min) : une entrée distante
//!   n'est acceptée que si le propriétaire de son dépôt figure parmi ceux du registre EMBARQUÉ
//!   (un `apps.json` distant ne peut pas pointer vers un dépôt tiers). Il remplace la liste
//!   embarquée s'il contient au moins une entrée acceptée ; sinon (ou en cas d'échec) on garde
//!   le dernier registre valide connu, à défaut le registre embarqué.
//! - Seules les apps `enabled` sont exposées à l'interface.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Registre embarqué dans l'exe (contrat avec le dépôt).
const EMBEDDED: &str = include_str!("../../apps.json");
/// Registre distant : même format, lu sur la branche `main` du dépôt du Lanceur.
pub const REMOTE_URL: &str = "https://raw.githubusercontent.com/Pierre-ML/lucania_luncheur/main/apps.json";
/// Taille maximale du registre distant.
pub const REMOTE_MAX_BYTES: u64 = 256 * 1024;
/// Durée de validité du registre distant en mémoire.
const REMOTE_TTL: Duration = Duration::from_secs(10 * 60);

const SCHEMA: u32 = 1;
const DEFAULT_MAX_MB: u64 = 500;
const MAX_MB_CAP: u64 = 2048;
const MAX_ARGS: usize = 4;
const MAX_NAME_CHARS: usize = 64;
const MAX_DESCRIPTION_CHARS: usize = 300;
const MAX_ICON_URL_LEN: usize = 300;
/// Identifiant du Lanceur lui-même : jamais accepté pour une app (ses données seraient effacées).
const OWN_IDENTIFIER: &str = "com.lanceur.desktop";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Description {
  pub fr: String,
  pub en: String,
}

/// Entrée VALIDÉE du registre.
#[derive(Debug, Clone, PartialEq)]
pub struct AppDef {
  pub id: String,
  pub enabled: bool,
  pub name: String,
  pub description: Description,
  pub icon: String,
  /// `owner/nom` du dépôt GitHub.
  pub repo: String,
  /// Nom du fichier de première installation publié dans la release.
  pub setup: String,
  /// Nom du fichier de mise à jour publié dans la release.
  pub updater: String,
  /// Identifiant Tauri : nom des dossiers de données `%APPDATA%` / `%LOCALAPPDATA%`.
  pub identifier: String,
  /// Nom de la clé `HKCU\…\Uninstall\<productName>` et des raccourcis.
  pub product_name: String,
  /// Nom de l'exe principal, dans le dossier d'installation.
  pub exe: String,
  pub wait_pid_arg: Option<String>,
  pub keep_data_arg: Option<String>,
  pub install_args: Vec<String>,
  pub update_args: Vec<String>,
  /// Taille maximale des téléchargements, en octets.
  pub max_bytes: u64,
}

impl AppDef {
  /// Propriétaire du dépôt (partie avant le `/`).
  pub fn owner(&self) -> &str {
    self.repo.split('/').next().unwrap_or("")
  }
}

/// Entrée brute telle qu'écrite dans `apps.json` (champs inconnus tolérés).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawApp {
  id: String,
  #[serde(default = "enabled_by_default")]
  enabled: bool,
  name: String,
  description: Description,
  icon: String,
  repo: String,
  setup: String,
  updater: String,
  identifier: String,
  product_name: String,
  exe: String,
  wait_pid_arg: Option<String>,
  keep_data_arg: Option<String>,
  install_args: Option<Vec<String>>,
  update_args: Option<Vec<String>>,
  max_size_mb: Option<u64>,
}

fn enabled_by_default() -> bool {
  true
}

#[derive(Deserialize)]
struct RawFile {
  schema: u32,
  apps: Vec<serde_json::Value>,
}

// ---------------------------------------------------------------------------------------------
// Validations (équivalents écrits à la main des expressions du contrat)

fn is_lower_alnum(c: char) -> bool {
  c.is_ascii_lowercase() || c.is_ascii_digit()
}

/// `^[a-z0-9][a-z0-9-]{0,31}$`
fn is_id(s: &str) -> bool {
  let mut chars = s.chars();
  matches!(chars.next(), Some(c) if is_lower_alnum(c))
    && s.len() <= 32
    && chars.all(|c| is_lower_alnum(c) || c == '-')
}

/// `^[A-Za-z0-9][A-Za-z0-9-]{0,38}/[A-Za-z0-9._-]{1,100}$`, et nom différent de `.` / `..`
/// (une URL `…/owner/../releases` serait normalisée vers un autre chemin).
fn is_repo(s: &str) -> bool {
  let Some((owner, name)) = s.split_once('/') else { return false };
  let mut oc = owner.chars();
  matches!(oc.next(), Some(c) if c.is_ascii_alphanumeric())
    && owner.len() <= 39
    && oc.all(|c| c.is_ascii_alphanumeric() || c == '-')
    && (1..=100).contains(&name.len())
    && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    && name != "."
    && name != ".."
}

/// `^[A-Za-z0-9][A-Za-z0-9 ._-]{0,63}\.exe$` : simple nom de fichier, aucun séparateur de chemin.
fn is_exe_name(s: &str) -> bool {
  let Some(stem) = s.strip_suffix(".exe") else { return false };
  let mut chars = stem.chars();
  matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
    && stem.len() <= 64
    && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-'))
}

/// `^[A-Za-z0-9][A-Za-z0-9.-]{0,127}$`, plus (il sert de nom de dossier de données à effacer) :
/// forme de domaine inversé (au moins un point, aucun segment vide) et différent du Lanceur.
fn is_identifier(s: &str) -> bool {
  let mut chars = s.chars();
  matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
    && s.len() <= 128
    && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
    && s.contains('.')
    && s.split('.').all(|seg| !seg.is_empty())
    && !s.eq_ignore_ascii_case(OWN_IDENTIFIER)
}

/// `^[A-Za-z0-9][A-Za-z0-9 ._-]{0,63}$`, sans espace ni point final (Windows les retire des noms
/// de fichiers : le raccourci `<productName>.lnk` ne correspondrait plus).
fn is_product_name(s: &str) -> bool {
  let mut chars = s.chars();
  matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
    && s.len() <= 64
    && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-'))
    && !s.ends_with([' ', '.'])
}

/// `^/[A-Z0-9_]{1,40}=$`
fn is_wait_pid_arg(s: &str) -> bool {
  s.strip_prefix('/')
    .and_then(|r| r.strip_suffix('='))
    .is_some_and(|m| (1..=40).contains(&m.len()) && m.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
}

/// `^/[A-Za-z0-9_]{1,40}$` : un seul argument, sans espace ni guillemet.
fn is_switch(s: &str) -> bool {
  s.strip_prefix('/')
    .is_some_and(|m| (1..=40).contains(&m.len()) && m.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
}

/// Texte affiché : non vide (si `required`), borné, sans caractère de contrôle.
fn is_text(s: &str, max_chars: usize, required: bool) -> bool {
  let n = s.chars().count();
  (!required || !s.trim().is_empty()) && n <= max_chars && !s.chars().any(char::is_control)
}

/// Icône : nom de fichier `^[a-z0-9-]+\.(svg|png)$` (servi par le frontend depuis `/app-icons/`)
/// OU URL `https://raw.githubusercontent.com/…` (autorisée par la CSP `img-src`).
fn is_icon(s: &str) -> bool {
  let file_ok = s
    .strip_suffix(".svg")
    .or_else(|| s.strip_suffix(".png"))
    .is_some_and(|stem| (1..=64).contains(&stem.len()) && stem.chars().all(|c| is_lower_alnum(c) || c == '-'));
  file_ok || is_icon_url(s)
}

fn is_icon_url(s: &str) -> bool {
  const PREFIX: &str = "https://raw.githubusercontent.com/";
  s.len() <= MAX_ICON_URL_LEN
    && s.strip_prefix(PREFIX).is_some_and(|path| !path.is_empty())
    && s.chars().all(|c| c.is_ascii_alphanumeric() || "/:._-~%".contains(c))
    && !s.contains("..")
    && reqwest::Url::parse(s).is_ok_and(|u| {
      u.host_str() == Some("raw.githubusercontent.com") && u.port().is_none() && u.username().is_empty() && u.password().is_none()
    })
}

/// Arguments d'installateur : `default` si absent, au plus `MAX_ARGS`, chacun validé.
fn valid_args(args: Option<Vec<String>>) -> Option<Vec<String>> {
  let args = args.unwrap_or_else(|| vec!["/S".to_string()]);
  (args.len() <= MAX_ARGS && args.iter().all(|a| is_switch(a))).then_some(args)
}

/// Valide une entrée brute ; `None` = entrée ignorée.
fn validate(raw: RawApp) -> Option<AppDef> {
  let ok = is_id(&raw.id)
    && is_text(&raw.name, MAX_NAME_CHARS, true)
    && is_text(&raw.description.fr, MAX_DESCRIPTION_CHARS, false)
    && is_text(&raw.description.en, MAX_DESCRIPTION_CHARS, false)
    && is_icon(&raw.icon)
    && is_repo(&raw.repo)
    && is_exe_name(&raw.setup)
    && is_exe_name(&raw.updater)
    && is_exe_name(&raw.exe)
    && is_identifier(&raw.identifier)
    && is_product_name(&raw.product_name)
    && raw.wait_pid_arg.as_deref().is_none_or(is_wait_pid_arg)
    && raw.keep_data_arg.as_deref().is_none_or(is_switch);
  if !ok {
    return None;
  }
  let max_mb = raw.max_size_mb.unwrap_or(DEFAULT_MAX_MB);
  if max_mb == 0 {
    return None;
  }
  Some(AppDef {
    id: raw.id,
    enabled: raw.enabled,
    name: raw.name.trim().to_string(),
    description: raw.description,
    icon: raw.icon,
    repo: raw.repo,
    setup: raw.setup,
    updater: raw.updater,
    identifier: raw.identifier,
    product_name: raw.product_name,
    exe: raw.exe,
    wait_pid_arg: raw.wait_pid_arg,
    keep_data_arg: raw.keep_data_arg,
    install_args: valid_args(raw.install_args)?,
    update_args: valid_args(raw.update_args)?,
    max_bytes: max_mb.min(MAX_MB_CAP) * 1024 * 1024,
  })
}

/// Lit un `apps.json` : `None` si le fichier lui-même est illisible ou d'un autre schéma.
/// Entrées invalides, en double (même `id`) ou dont le propriétaire n'est pas dans
/// `allowed_owners` (s'il est fourni, comparaison sans casse comme GitHub) : ignorées.
fn parse(json: &str, allowed_owners: Option<&HashSet<String>>) -> Option<Vec<AppDef>> {
  let file: RawFile = serde_json::from_str(json).ok()?;
  if file.schema != SCHEMA {
    return None;
  }
  let mut seen = HashSet::new();
  let apps = file
    .apps
    .into_iter()
    .filter_map(|v| serde_json::from_value::<RawApp>(v).ok())
    .filter_map(validate)
    .filter(|a| allowed_owners.is_none_or(|owners| owners.contains(&a.owner().to_ascii_lowercase())))
    .filter(|a| seen.insert(a.id.clone()))
    .collect();
  Some(apps)
}

/// Registre embarqué validé (vide seulement si `apps.json` est cassé : voir le test).
fn embedded() -> Vec<AppDef> {
  parse(EMBEDDED, None).unwrap_or_default()
}

fn owners_of(apps: &[AppDef]) -> HashSet<String> {
  apps.iter().map(|a| a.owner().to_ascii_lowercase()).collect()
}

/// Registre distant validé contre les propriétaires du registre embarqué ; `None` s'il est
/// illisible ou si aucune entrée n'est acceptée.
fn parse_remote(json: &str, embedded: &[AppDef]) -> Option<Vec<AppDef>> {
  parse(json, Some(&owners_of(embedded))).filter(|apps| !apps.is_empty())
}

// ---------------------------------------------------------------------------------------------
// État partagé

struct Inner {
  embedded: Arc<Vec<AppDef>>,
  current: Arc<Vec<AppDef>>,
  /// Dernier chargement réussi du registre distant.
  fetched: Option<Instant>,
}

/// Registre courant (état Tauri).
pub struct Registry(Mutex<Inner>);

impl Registry {
  pub fn new() -> Self {
    let embedded = Arc::new(embedded());
    Self(Mutex::new(Inner { current: embedded.clone(), embedded, fetched: None }))
  }

  /// Toutes les entrées valides du registre courant (y compris désactivées).
  pub fn all(&self) -> Arc<Vec<AppDef>> {
    self.0.lock().unwrap().current.clone()
  }

  /// Apps exposées à l'interface.
  pub fn enabled(&self) -> Vec<AppDef> {
    self.all().iter().filter(|a| a.enabled).cloned().collect()
  }

  /// App exposée à l'interface, par identifiant.
  pub fn find(&self, id: &str) -> Option<AppDef> {
    self.all().iter().find(|a| a.enabled && a.id == id).cloned()
  }

  /// Vrai s'il faut relire le registre distant.
  pub fn needs_refresh(&self, force: bool) -> bool {
    force || self.0.lock().unwrap().fetched.is_none_or(|t| t.elapsed() >= REMOTE_TTL)
  }

  /// Applique le texte du registre distant ; faux s'il est rejeté (registre courant inchangé).
  pub fn apply_remote(&self, json: &str) -> bool {
    let mut inner = self.0.lock().unwrap();
    match parse_remote(json, &inner.embedded) {
      Some(apps) => {
        inner.current = Arc::new(apps);
        inner.fetched = Some(Instant::now());
        true
      }
      None => false,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample() -> serde_json::Value {
    serde_json::json!({
      "id": "demo",
      "name": "Demo",
      "description": { "fr": "Démo", "en": "Demo" },
      "icon": "demo.svg",
      "repo": "Pierre-ML/Demo",
      "setup": "Demo-Setup.exe",
      "updater": "Demo-Update.exe",
      "identifier": "com.demo.desktop",
      "productName": "Demo",
      "exe": "demo.exe"
    })
  }

  fn with(field: &str, value: serde_json::Value) -> serde_json::Value {
    let mut v = sample();
    v[field] = value;
    v
  }

  fn one(entry: serde_json::Value) -> Option<AppDef> {
    let json = serde_json::json!({ "schema": 1, "apps": [entry] }).to_string();
    parse(&json, None).and_then(|apps| apps.into_iter().next())
  }

  #[test]
  fn registre_embarque_valide() {
    let apps = embedded();
    assert_eq!(apps.len(), 7, "les 7 entrées de apps.json doivent être valides");
    let enabled: Vec<_> = apps.iter().filter(|a| a.enabled).map(|a| a.id.as_str()).collect();
    assert_eq!(enabled, ["lucaia", "lucascrap", "lucamedia", "lucaaudit"]);
    let lucaia = &apps[0];
    assert_eq!((lucaia.product_name.as_str(), lucaia.exe.as_str(), lucaia.identifier.as_str()), ("Luca-ia", "lucania.exe", "com.lucania.desktop"));
    assert_eq!(lucaia.wait_pid_arg.as_deref(), Some("/LUCANIA_WAITPID="));
    assert_eq!(lucaia.max_bytes, 500 * 1024 * 1024);
    // Clé de désinstallation, exe et identifiant : ceux des releases publiées (productName et
    // mainBinaryName des tauri.conf.json aux tags v1.0.0 / v1.0.2 / v1.0.2), pas ceux de main.
    let by_id = |id: &str| apps.iter().find(|a| a.id == id).unwrap();
    let (scrap, media, eco) = (by_id("lucascrap"), by_id("lucamedia"), by_id("lucaaudit"));
    assert_eq!((scrap.product_name.as_str(), scrap.exe.as_str()), ("Lucascrap", "lucascrap.exe"));
    assert_eq!((media.product_name.as_str(), media.exe.as_str(), media.identifier.as_str()), ("Lucamedia", "lucamedia.exe", "com.lucamedia.app"));
    assert_eq!((eco.product_name.as_str(), eco.exe.as_str()), ("Luca-Audit", "luca-audit.exe"));
  }

  #[test]
  fn valeurs_par_defaut() {
    let app = one(sample()).expect("entrée minimale valide");
    assert!(app.enabled);
    assert_eq!(app.install_args, ["/S"]);
    assert_eq!(app.update_args, ["/S"]);
    assert_eq!(app.max_bytes, 500 * 1024 * 1024);
    assert!(app.wait_pid_arg.is_none() && app.keep_data_arg.is_none());
    let capped = one(with("maxSizeMb", 99_999.into())).unwrap();
    assert_eq!(capped.max_bytes, 2048 * 1024 * 1024);
    assert!(one(with("maxSizeMb", 0.into())).is_none());
  }

  #[test]
  fn attaques_sur_les_noms_de_fichiers() {
    for bad in ["..\\evil.exe", "../evil.exe", "dir/app.exe", "C:\\x.exe", "app.EXE", ".exe", " app.exe", "app.exe ", "a\"b.exe"] {
      assert!(one(with("setup", bad.into())).is_none(), "setup accepté : {bad}");
      assert!(one(with("exe", bad.into())).is_none(), "exe accepté : {bad}");
    }
    assert!(one(with("updater", "Mon App-Update_2.exe".into())).is_some());
  }

  #[test]
  fn attaques_sur_le_depot_et_l_identifiant() {
    for bad in ["Pierre-ML/..", "Pierre-ML/.", "Pierre-ML", "Pierre-ML/a/b", "-x/y", "a_b/c", "x/", "x/y z", "x/y?z"] {
      assert!(one(with("repo", bad.into())).is_none(), "dépôt accepté : {bad}");
    }
    for bad in ["Microsoft", "..", "a..b", "com.x.", ".com.x", "com\\x.y", "com/x.y", "com.lanceur.desktop"] {
      assert!(one(with("identifier", bad.into())).is_none(), "identifiant accepté : {bad}");
    }
    for bad in ["", "Demo.", "Demo ", "De\\mo", "De/mo", "..", "A:B"] {
      assert!(one(with("productName", bad.into())).is_none(), "productName accepté : {bad}");
    }
    for bad in ["Demo", "-demo", "a".repeat(33).as_str(), "dé"] {
      assert!(one(with("id", bad.into())).is_none(), "id accepté : {bad}");
    }
  }

  #[test]
  fn attaques_sur_les_arguments() {
    for bad in ["/S /D=C:\\x", "/S\"", "S", "/", "/a-b", "/UPDATE=1", "/S;calc"] {
      assert!(one(with("keepDataArg", bad.into())).is_none(), "keepDataArg accepté : {bad}");
      assert!(one(with("installArgs", serde_json::json!([bad]))).is_none(), "installArgs accepté : {bad}");
    }
    assert!(one(with("updateArgs", serde_json::json!(["/S", "/A", "/B", "/C", "/D"]))).is_none());
    assert!(one(with("updateArgs", serde_json::json!([]))).is_some());
    for bad in ["/lucania_waitpid=", "/X", "/X =", "/=", "LUCANIA_WAITPID="] {
      assert!(one(with("waitPidArg", bad.into())).is_none(), "waitPidArg accepté : {bad}");
    }
    assert!(one(with("waitPidArg", "/APP_2_WAITPID=".into())).is_some());
  }

  #[test]
  fn icones_et_textes() {
    assert!(one(with("icon", "https://raw.githubusercontent.com/Pierre-ML/Demo/main/icon.png".into())).is_some());
    for bad in [
      "Demo.svg",
      "demo.gif",
      "../demo.svg",
      "http://raw.githubusercontent.com/x.png",
      "https://raw.githubusercontent.com.evil.example/x.png",
      "https://raw.githubusercontent.com/a/../b.png",
      "https://raw.githubusercontent.com/x.png\" onerror=\"x",
      "https://user@raw.githubusercontent.com/x.png",
      "javascript:alert(1)",
    ] {
      assert!(one(with("icon", bad.into())).is_none(), "icône acceptée : {bad}");
    }
    assert!(one(with("name", "   ".into())).is_none());
    assert!(one(with("name", "a".repeat(65).into())).is_none());
    assert!(one(with("name", "Nom\u{0007}".into())).is_none());
    assert!(one(with("description", serde_json::json!({ "fr": "x".repeat(301), "en": "" }))).is_none());
    assert!(one(with("description", serde_json::json!({ "fr": "seulement fr" }))).is_none());
  }

  #[test]
  fn fichier_et_doublons() {
    assert!(parse("pas du json", None).is_none());
    assert!(parse(r#"{ "schema": 2, "apps": [] }"#, None).is_none());
    let json = serde_json::json!({ "schema": 1, "apps": [sample(), with("name", "Autre".into()), "texte", with("repo", "x/..".into())] });
    let apps = parse(&json.to_string(), None).unwrap();
    assert_eq!(apps.len(), 1, "doublon, valeur non objet et entrée invalide ignorés");
    assert_eq!(apps[0].name, "Demo");
  }

  #[test]
  fn registre_distant_limite_aux_proprietaires_embarques() {
    let embedded = embedded();
    let foreign = with("repo", "Attaquant/Demo".into());
    let mut other = with("id", "autre".into());
    other["repo"] = "pierre-ml/Autre".into(); // propriétaire GitHub : casse indifférente
    let json = serde_json::json!({ "schema": 1, "apps": [foreign, other] }).to_string();
    let apps = parse_remote(&json, &embedded).expect("une entrée acceptée");
    assert_eq!(apps.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["autre"]);

    let only_foreign = serde_json::json!({ "schema": 1, "apps": [with("repo", "Attaquant/Demo".into())] });
    assert!(parse_remote(&only_foreign.to_string(), &embedded).is_none());

    let registry = Registry::new();
    assert!(!registry.apply_remote(&only_foreign.to_string()));
    assert_eq!(registry.all().len(), 7, "registre embarqué conservé");
    assert!(registry.apply_remote(&json));
    assert!(registry.find("autre").is_some() && registry.find("lucaia").is_none());
    assert!(!registry.needs_refresh(false) && registry.needs_refresh(true));
  }
}
