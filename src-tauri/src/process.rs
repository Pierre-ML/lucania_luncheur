//! Processus des apps installées : détection, fermeture propre, lancement.
//!
//! - Un processus appartient à une app si le chemin COMPLET de son image
//!   (`QueryFullProcessImageNameW`) est exactement l'exe installé ; le nom du fichier ne sert
//!   qu'à présélectionner les candidats de la liste des processus (ToolHelp).
//! - Fermeture propre : `WM_CLOSE` posté aux fenêtres visibles de premier niveau (sans
//!   propriétaire) de ces processus, puis attente de leur fin. Jamais de `TerminateProcess` ni de
//!   « kill » par nom : si l'app refuse de se fermer, l'appelant décide (`still_running`).
//! - Lancement : `std::process::Command` direct (jamais de `cmd` ni de shell), arguments un à un,
//!   E/S standard nulles, `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` comme l'updater de Lucania.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
  CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
  OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
  EnumWindows, GW_OWNER, GetWindow, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE,
};

use crate::detect::same_path;

/// Intervalle de vérification pendant les attentes.
const POLL: Duration = Duration::from_millis(250);

/// Liste des processus : (pid, nom de l'image en minuscules).
fn snapshot() -> Vec<(u32, String)> {
  let mut out = Vec::new();
  // SAFETY : instantané refermé en fin de fonction ; `dwSize` renseigné comme l'exige l'API.
  unsafe {
    let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snap == INVALID_HANDLE_VALUE {
      return out;
    }
    let mut entry: PROCESSENTRY32W = std::mem::zeroed();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut ok = Process32FirstW(snap, &mut entry) != 0;
    while ok {
      let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
      out.push((entry.th32ProcessID, String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase()));
      ok = Process32NextW(snap, &mut entry) != 0;
    }
    CloseHandle(snap);
  }
  out
}

/// Chemin complet de l'image d'un processus ; `None` si inaccessible.
fn image_path(pid: u32) -> Option<String> {
  // SAFETY : handle refermé avant de sortir ; tampon de `len` caractères.
  unsafe {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if handle.is_null() {
      return None;
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) != 0;
    CloseHandle(handle);
    ok.then(|| String::from_utf16_lossy(&buf[..len as usize]))
  }
}

/// Pour chaque exe, les pid des processus dont l'image est exactement ce fichier (un seul
/// instantané pour tous).
pub fn pids_of_many(exes: &[&Path]) -> Vec<Vec<u32>> {
  let procs = snapshot();
  let me = std::process::id();
  exes
    .iter()
    .map(|exe| {
      let Some(name) = exe.file_name().map(|n| n.to_string_lossy().to_lowercase()) else { return Vec::new() };
      procs
        .iter()
        .filter(|(pid, n)| *pid != me && *n == name)
        .filter(|(pid, _)| image_path(*pid).is_some_and(|p| same_path(Path::new(&p), exe)))
        .map(|(pid, _)| *pid)
        .collect()
    })
    .collect()
}

pub fn pids_of(exe: &Path) -> Vec<u32> {
  pids_of_many(&[exe]).pop().unwrap_or_default()
}

struct WindowSearch<'a> {
  pids: &'a [u32],
  found: Vec<HWND>,
}

unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> i32 {
  // SAFETY : `lparam` pointe sur la `WindowSearch` de `post_close`, vivante pendant `EnumWindows`.
  let search = unsafe { &mut *(lparam as *mut WindowSearch) };
  let mut pid = 0u32;
  unsafe {
    GetWindowThreadProcessId(hwnd, &mut pid);
    if search.pids.contains(&pid) && IsWindowVisible(hwnd) != 0 && GetWindow(hwnd, GW_OWNER).is_null() {
      search.found.push(hwnd);
    }
  }
  1 // continuer l'énumération
}

/// Poste `WM_CLOSE` aux fenêtres visibles de premier niveau des processus `pids`.
fn post_close(pids: &[u32]) {
  let mut search = WindowSearch { pids, found: Vec::new() };
  // SAFETY : rappel synchrone ; `search` survit à l'appel.
  unsafe {
    EnumWindows(Some(collect_window), &mut search as *mut WindowSearch as LPARAM);
    for hwnd in search.found {
      PostMessageW(hwnd, WM_CLOSE, 0, 0);
    }
  }
}

/// Demande la fermeture de l'app (`WM_CLOSE`) et attend au plus `timeout` ; renvoie les pid
/// encore vivants (vide = app fermée ou non lancée).
pub async fn close_gracefully(exe: &Path, timeout: Duration) -> Vec<u32> {
  let pids = pids_of(exe);
  if pids.is_empty() {
    return pids;
  }
  post_close(&pids);
  let deadline = Instant::now() + timeout;
  loop {
    tokio::time::sleep(POLL).await;
    let alive = pids_of(exe);
    if alive.is_empty() || Instant::now() >= deadline {
      return alive;
    }
  }
}

/// Lance un exe directement (jamais via un shell), détaché, arguments passés un à un.
pub fn spawn(exe: &Path, args: &[String], cwd: Option<&Path>) -> std::io::Result<Child> {
  use std::os::windows::process::CommandExt;
  let mut cmd = Command::new(exe);
  cmd.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
  if let Some(dir) = cwd {
    cmd.current_dir(dir);
  }
  // Exe graphique : aucune console. DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP.
  cmd.creation_flags(0x0000_0008 | 0x0000_0200);
  cmd.spawn()
}

/// Attend la fin d'un processus lancé (au plus `timeout`) : `Some(code)` s'il s'est terminé.
pub async fn wait_child(child: &mut Child, timeout: Duration) -> Option<i32> {
  let deadline = Instant::now() + timeout;
  loop {
    match child.try_wait() {
      Ok(Some(status)) => return Some(status.code().unwrap_or(-1)),
      Ok(None) if Instant::now() < deadline => tokio::time::sleep(POLL).await,
      _ => return None,
    }
  }
}
