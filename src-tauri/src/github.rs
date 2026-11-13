//! Accès à GitHub : client HTTP unique, dernière release de chaque dépôt (cache), registre distant.
//!
//! - Client repris de l'updater de Lucania : HTTPS uniquement, hôtes autorisés `github.com`,
//!   `api.github.com` et `*.githubusercontent.com`, vérifiés à CHAQUE redirection (10 au plus).
//! - `releases/latest` : cache de 5 min par dépôt (ignoré si `force`), requêtes en parallèle.
//!   Une 404 signifie « aucune release » (mise en cache, pas une erreur). Limite de l'API atteinte
//!   (403/429 avec `x-ratelimit-remaining: 0` ou `retry-after`) : plus aucune requête avant
//!   `x-ratelimit-reset`, les valeurs en cache restent affichées, avertissement `rate_limited`.
//! - Codes d'erreur stables (traduits par l'interface) : `offline`, `rate_limited`, `no_release`,
//!   `http:<statut>`, `too_large`.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use reqwest::header::HeaderMap;
use reqwest::{Client, Response, StatusCode, Url, redirect::Policy};
use serde::Deserialize;

/// Durée de validité de la dernière release d'un dépôt.
const RELEASE_TTL: Duration = Duration::from_secs(5 * 60);
/// Attente par défaut quand GitHub signale la limite sans dire jusqu'à quand.
const DEFAULT_BLOCK: Duration = Duration::from_secs(60);
const MAX_TAG_LEN: usize = 64;

/// Hôtes autorisés : l'API GitHub, le site et le stockage des fichiers de release (redirections).
pub fn allowed_host(url: &Url) -> bool {
  url.scheme() == "https"
    && url.host_str().is_some_and(|h| {
      let h = h.to_ascii_lowercase();
      h == "github.com" || h == "api.github.com" || h.ends_with(".githubusercontent.com")
    })
}

fn build_client() -> Result<Client, String> {
  Client::builder()
    .user_agent(concat!("Lanceur/", env!("CARGO_PKG_VERSION")))
    .https_only(true)
    .connect_timeout(Duration::from_secs(15))
    .read_timeout(Duration::from_secs(60))
    .redirect(Policy::custom(|attempt| {
      if attempt.previous().len() >= 10 {
        attempt.error("too_many_redirects")
      } else if !allowed_host(attempt.url()) {
        attempt.error("forbidden_host")
      } else {
        attempt.follow()
      }
    }))
    .build()
    .map_err(|_| "offline".to_string())
}

/// Client partagé (le chargement des certificats Windows n'est fait qu'une fois).
pub fn client() -> Result<Client, String> {
  static CLIENT: Mutex<Option<Client>> = Mutex::new(None);
  let mut slot = CLIENT.lock().unwrap();
  if slot.is_none() {
    *slot = Some(build_client()?);
  }
  Ok(slot.clone().expect("client initialisé"))
}

/// Version « X.Y.Z » (préfixe `v` toléré, suffixe `-beta` ignoré) ; `None` si illisible.
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
  let core = s.trim().trim_start_matches(['v', 'V']).split(['-', '+']).next()?;
  let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
  let v = (parts.next()??, parts.next()??, parts.next().unwrap_or(Some(0))?);
  parts.next().is_none().then_some(v)
}

pub fn is_newer(latest: &str, current: &str) -> bool {
  matches!((parse_version(latest), parse_version(current)), (Some(l), Some(c)) if l > c)
}

/// Erreur d'envoi → `offline` (connexion, délai) ou `http:redirect` (redirection refusée).
pub fn send_error(e: &reqwest::Error) -> String {
  if e.is_redirect() { "http:redirect".into() } else { "offline".into() }
}

fn header_u64(headers: &HeaderMap, name: &str) -> Option<u64> {
  headers.get(name)?.to_str().ok()?.trim().parse().ok()
}

/// Limite de l'API atteinte ? Renvoie alors l'instant jusqu'auquel il ne faut plus l'interroger.
fn rate_limit_until(status: StatusCode, headers: &HeaderMap, now: SystemTime) -> Option<SystemTime> {
  if status != StatusCode::FORBIDDEN && status != StatusCode::TOO_MANY_REQUESTS {
    return None;
  }
  let exhausted = headers.get("x-ratelimit-remaining").and_then(|v| v.to_str().ok()).is_some_and(|v| v.trim() == "0");
  let retry_after = header_u64(headers, "retry-after");
  if !exhausted && retry_after.is_none() {
    return None;
  }
  let reset = header_u64(headers, "x-ratelimit-reset").map(|s| UNIX_EPOCH + Duration::from_secs(s));
  let until = match (exhausted, reset, retry_after) {
    (true, Some(reset), _) => reset,
    (_, _, Some(secs)) => now + Duration::from_secs(secs),
    _ => now + DEFAULT_BLOCK,
  };
  Some(until.max(now + Duration::from_secs(1)))
}

/// Statut non 2xx → code d'erreur stable.
pub fn status_error(resp: &Response) -> String {
  let status = resp.status();
  if rate_limit_until(status, resp.headers(), SystemTime::now()).is_some() {
    "rate_limited".into()
  } else if status == StatusCode::NOT_FOUND {
    "no_release".into()
  } else {
    format!("http:{}", status.as_u16())
  }
}

/// Lit un petit corps texte (registre distant), refusé au-delà de `max` octets.
async fn read_small_text(resp: Response, max: u64) -> Result<String, String> {
  if resp.content_length().is_some_and(|n| n > max) {
    return Err("too_large".into());
  }
  let mut body = Vec::new();
  let mut stream = resp.bytes_stream();
  while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|_| "offline".to_string())?;
    if body.len() as u64 + chunk.len() as u64 > max {
      return Err("too_large".into());
    }
    body.extend_from_slice(&chunk);
  }
  String::from_utf8(body).map_err(|_| "invalid_file".to_string())
}

/// Télécharge le texte d'une URL GitHub autorisée (registre distant).
pub async fn fetch_text(url: &str, max: u64) -> Result<String, String> {
  let url = Url::parse(url).ok().filter(allowed_host).ok_or_else(|| "http:host".to_string())?;
  let resp = client()?.get(url).send().await.map_err(|e| send_error(&e))?;
  if !resp.status().is_success() {
    return Err(status_error(&resp));
  }
  read_small_text(resp, max).await
}

/// Dernière release publiée d'un dépôt.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseInfo {
  /// Version sans préfixe `v`.
  pub version: String,
  /// Page de la release (toujours sur `https://github.com/`).
  pub url: Option<String>,
}

#[derive(Deserialize)]
struct Release {
  tag_name: String,
  #[serde(default)]
  html_url: String,
}

/// Échec d'une requête `releases/latest`.
enum FetchError {
  Offline,
  RateLimited(SystemTime),
  Other,
}

/// Interroge `releases/latest` ; `Ok(None)` = aucune release (404).
async fn fetch_latest(client: &Client, repo: &str) -> Result<Option<ReleaseInfo>, FetchError> {
  let url = format!("https://api.github.com/repos/{repo}/releases/latest");
  let resp = client
    .get(url)
    .header("Accept", "application/vnd.github+json")
    .send()
    .await
    .map_err(|_| FetchError::Offline)?;
  let status = resp.status();
  if let Some(until) = rate_limit_until(status, resp.headers(), SystemTime::now()) {
    return Err(FetchError::RateLimited(until));
  }
  if status == StatusCode::NOT_FOUND {
    return Ok(None);
  }
  if !status.is_success() {
    return Err(FetchError::Other);
  }
  let release: Release = resp.json().await.map_err(|_| FetchError::Other)?;
  Ok(release_info(release))
}

/// Normalise une release : tag lisible (sinon « aucune release »), URL limitée à github.com.
fn release_info(release: Release) -> Option<ReleaseInfo> {
  let version = release.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
  if version.is_empty() || version.len() > MAX_TAG_LEN || version.chars().any(|c| c.is_control() || c.is_whitespace()) {
    return None;
  }
  let url = Some(release.html_url).filter(|u| u.starts_with("https://github.com/") && u.len() <= 512 && !u.contains(['"', '<', '>', ' ']));
  Some(ReleaseInfo { version, url })
}

/// Vérifie la dernière release d'un dépôt pour le Lanceur lui-même (sans cache) ; erreurs codées.
pub async fn latest_release(repo: &str) -> Result<ReleaseInfo, String> {
  let url = format!("https://api.github.com/repos/{repo}/releases/latest");
  let resp = client()?
    .get(url)
    .header("Accept", "application/vnd.github+json")
    .send()
    .await
    .map_err(|e| send_error(&e))?;
  if !resp.status().is_success() {
    return Err(status_error(&resp));
  }
  let release: Release = resp.json().await.map_err(|_| "http:invalid_json".to_string())?;
  release_info(release).ok_or_else(|| "no_release".to_string())
}

#[derive(Default)]
struct ReleasesInner {
  /// Par dépôt : instant de la dernière réponse et release (`None` = aucune).
  cache: HashMap<String, (Instant, Option<ReleaseInfo>)>,
  /// Limite de l'API atteinte : aucune requête avant cet instant.
  blocked_until: Option<SystemTime>,
}

/// Cache des dernières releases (état Tauri).
#[derive(Default)]
pub struct Releases(Mutex<ReleasesInner>);

impl Releases {
  /// Dernière release connue (même périmée) ; aucun réseau.
  pub fn cached(&self, repo: &str) -> Option<ReleaseInfo> {
    self.0.lock().unwrap().cache.get(repo).and_then(|(_, info)| info.clone())
  }

  /// Rafraîchit les dépôts dont le cache est périmé (tous si `force`), en parallèle.
  /// Renvoie l'avertissement éventuel : `rate_limited` ou `offline`.
  pub async fn refresh(&self, repos: &[String], force: bool) -> Option<String> {
    let todo: Vec<String> = {
      let inner = self.0.lock().unwrap();
      if inner.blocked_until.is_some_and(|t| SystemTime::now() < t) {
        return Some("rate_limited".into());
      }
      repos
        .iter()
        .filter(|r| force || inner.cache.get(*r).is_none_or(|(t, _)| t.elapsed() >= RELEASE_TTL))
        .cloned()
        .collect()
    };
    if todo.is_empty() {
      return None;
    }
    let client = match client() {
      Ok(c) => c,
      Err(e) => return Some(e),
    };
    let results = futures_util::future::join_all(todo.iter().map(|repo| fetch_latest(&client, repo))).await;

    let mut inner = self.0.lock().unwrap();
    let mut warning = None;
    for (repo, result) in todo.into_iter().zip(results) {
      match result {
        Ok(info) => {
          inner.cache.insert(repo, (Instant::now(), info));
        }
        Err(FetchError::RateLimited(until)) => {
          inner.blocked_until = Some(inner.blocked_until.map_or(until, |t| t.max(until)));
          warning = Some("rate_limited".to_string());
        }
        Err(FetchError::Offline) => {
          warning.get_or_insert_with(|| "offline".to_string());
        }
        Err(FetchError::Other) => {}
      }
    }
    warning
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use reqwest::header::HeaderValue;

  #[test]
  fn lecture_de_version() {
    assert_eq!(parse_version("v0.2.0"), Some((0, 2, 0)));
    assert_eq!(parse_version("1.10.3-beta"), Some((1, 10, 3)));
    assert_eq!(parse_version("2.1"), Some((2, 1, 0)));
    assert_eq!(parse_version("abc"), None);
    assert_eq!(parse_version("1.2.3.4"), None);
  }

  #[test]
  fn comparaison_de_version() {
    assert!(is_newer("0.2.1", "0.2.0"));
    assert!(is_newer("v0.10.0", "0.9.9"));
    assert!(!is_newer("0.2.0", "0.2.0"));
    assert!(!is_newer("0.1.16", "0.2.0"));
    assert!(!is_newer("illisible", "0.2.0"));
  }

  #[test]
  fn hotes_autorises() {
    assert!(allowed_host(&Url::parse("https://objects.githubusercontent.com/x").unwrap()));
    assert!(allowed_host(&Url::parse("https://raw.githubusercontent.com/Pierre-ML/lucania_luncheur/main/apps.json").unwrap()));
    assert!(allowed_host(&Url::parse("https://github.com/Pierre-ML/Lucania").unwrap()));
    assert!(allowed_host(&Url::parse("https://api.github.com/repos/x/y").unwrap()));
    assert!(!allowed_host(&Url::parse("http://github.com/").unwrap()));
    assert!(!allowed_host(&Url::parse("https://evil.example/").unwrap()));
    assert!(!allowed_host(&Url::parse("https://githubusercontent.com.evil.example/").unwrap()));
    assert!(!allowed_host(&Url::parse("https://evilgithubusercontent.com/").unwrap()));
    assert!(!allowed_host(&Url::parse("https://github.com.evil.example/").unwrap()));
  }

  #[test]
  fn limite_de_l_api() {
    let now = UNIX_EPOCH + Duration::from_secs(1_000);
    let mut h = HeaderMap::new();
    assert!(rate_limit_until(StatusCode::FORBIDDEN, &h, now).is_none(), "403 sans en-tête : pas une limite");
    h.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
    h.insert("x-ratelimit-reset", HeaderValue::from_static("1600"));
    assert_eq!(rate_limit_until(StatusCode::FORBIDDEN, &h, now), Some(UNIX_EPOCH + Duration::from_secs(1_600)));
    assert!(rate_limit_until(StatusCode::NOT_FOUND, &h, now).is_none());
    let mut r = HeaderMap::new();
    r.insert("retry-after", HeaderValue::from_static("30"));
    assert_eq!(rate_limit_until(StatusCode::TOO_MANY_REQUESTS, &r, now), Some(now + Duration::from_secs(30)));
  }

  #[test]
  fn normalisation_d_une_release() {
    let r = |tag: &str, url: &str| release_info(Release { tag_name: tag.into(), html_url: url.into() });
    let info = r("v1.2.3", "https://github.com/Pierre-ML/Lucania/releases/tag/v1.2.3").unwrap();
    assert_eq!(info.version, "1.2.3");
    assert!(info.url.is_some());
    assert!(r("v1.0.0", "javascript:alert(1)").unwrap().url.is_none());
    assert!(r("  ", "").is_none());
    assert!(r("1.0 beta", "").is_none());
  }
}
