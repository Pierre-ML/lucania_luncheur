//! Téléchargement en flux d'un installateur vers `%TEMP%\Lanceur\<sous-dossier>\<nom>.part`,
//! renommé en `<nom>` seulement une fois complet et contrôlé.
//!
//! - URL vérifiée avant l'envoi (HTTPS, hôte GitHub autorisé ; redirections vérifiées par le client).
//! - Taille maximale appliquée sur `Content-Length` ET sur les octets reçus (`too_large`).
//! - Contrôle final : au moins `MIN_BYTES` octets et en-tête « MZ » d'un exe Windows
//!   (`invalid_file`) : écarte une page d'erreur servie à la place.
//! - Progression : au plus ~10 appels par seconde, puis un dernier à 100 %.
//! - Annulation (`Cancel`) : prise en compte immédiatement, même si le flux est bloqué ; l'opération
//!   échoue avec `cancelled`. Dans tous les cas d'échec, le fichier partiel est supprimé.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::Url;
use tokio::io::AsyncWriteExt;
use tokio::sync::Notify;

use crate::github;

const PROGRESS_EVERY: Duration = Duration::from_millis(100);
/// Taille minimale d'un installateur plausible.
const MIN_BYTES: u64 = 64 * 1024;

/// Jeton d'annulation d'un téléchargement.
#[derive(Default)]
pub struct Cancel {
  flag: AtomicBool,
  notify: Notify,
}

impl Cancel {
  pub fn cancel(&self) {
    self.flag.store(true, Ordering::SeqCst);
    // `notify_one` garde un « permis » si personne n'attend encore : aucune annulation perdue.
    self.notify.notify_one();
  }

  pub fn is_cancelled(&self) -> bool {
    self.flag.load(Ordering::SeqCst)
  }
}

/// Dossier de travail : `%TEMP%\Lanceur\<sub>` (`sub` = identifiant validé ou `self`).
pub fn work_dir(sub: &str) -> PathBuf {
  std::env::temp_dir().join("Lanceur").join(sub)
}

pub fn io_error(e: &std::io::Error) -> String {
  format!("io:{:?}", e.kind())
}

/// Fichier partiel supprimé au `Drop`, sauf après `commit`.
struct PartFile {
  path: PathBuf,
  keep: bool,
}

impl Drop for PartFile {
  fn drop(&mut self) {
    if !self.keep {
      let _ = std::fs::remove_file(&self.path);
    }
  }
}

/// Télécharge `url` vers `<dir>\<name>` et renvoie ce chemin.
pub async fn download(
  url: &str,
  dir: &Path,
  name: &str,
  max_bytes: u64,
  cancel: Option<&Cancel>,
  mut progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf, String> {
  let cancelled = || cancel.is_some_and(Cancel::is_cancelled);
  let url = Url::parse(url).ok().filter(github::allowed_host).ok_or_else(|| "http:host".to_string())?;
  tokio::fs::create_dir_all(dir).await.map_err(|e| io_error(&e))?;
  let final_path = dir.join(name);
  let mut part = PartFile { path: dir.join(format!("{name}.part")), keep: false };
  if cancelled() {
    return Err("cancelled".into());
  }

  let client = github::client()?;
  let resp = until_cancelled(cancel, client.get(url).send())
    .await?
    .map_err(|e| github::send_error(&e))?;
  if !resp.status().is_success() {
    return Err(github::status_error(&resp));
  }
  let total = resp.content_length().filter(|n| *n > 0);
  if total.is_some_and(|n| n > max_bytes) {
    return Err("too_large".into());
  }

  let mut file = tokio::fs::File::create(&part.path).await.map_err(|e| io_error(&e))?;
  let mut stream = resp.bytes_stream();
  let mut received: u64 = 0;
  let mut head: Vec<u8> = Vec::with_capacity(2);
  let mut last_emit = Instant::now() - PROGRESS_EVERY;
  progress(0, total);
  while let Some(chunk) = until_cancelled(cancel, stream.next()).await? {
    let chunk = chunk.map_err(|_| "offline".to_string())?;
    received += chunk.len() as u64;
    if received > max_bytes {
      return Err("too_large".into());
    }
    if head.len() < 2 {
      head.extend(chunk.iter().take(2 - head.len()));
    }
    file.write_all(&chunk).await.map_err(|e| io_error(&e))?;
    if last_emit.elapsed() >= PROGRESS_EVERY {
      last_emit = Instant::now();
      progress(received, total);
    }
  }
  file.flush().await.map_err(|e| io_error(&e))?;
  drop(file);

  // Un exe Windows commence toujours par « MZ ».
  if received < MIN_BYTES || head != b"MZ" || total.is_some_and(|n| n != received) {
    return Err("invalid_file".into());
  }
  tokio::fs::rename(&part.path, &final_path).await.map_err(|e| io_error(&e))?;
  part.keep = true;
  progress(received, Some(received));
  Ok(final_path)
}

/// Exécute `fut` sauf annulation : `Err("cancelled")` dès que `cancel` est déclenché.
async fn until_cancelled<T>(cancel: Option<&Cancel>, fut: impl Future<Output = T>) -> Result<T, String> {
  match cancel {
    None => Ok(fut.await),
    Some(c) => {
      if c.is_cancelled() {
        return Err("cancelled".into());
      }
      tokio::select! {
        v = fut => Ok(v),
        _ = c.notify.notified() => {
          // Permis consommé : on le remet pour les attentes suivantes de la même opération.
          c.notify.notify_one();
          Err("cancelled".into())
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dossier_de_travail() {
    let dir = work_dir("lucania");
    assert!(dir.ends_with(Path::new("Lanceur").join("lucania")));
    assert!(dir.starts_with(std::env::temp_dir()));
  }

  #[test]
  fn annulation_memorisee() {
    let c = Cancel::default();
    assert!(!c.is_cancelled());
    c.cancel();
    assert!(c.is_cancelled());
  }
}
