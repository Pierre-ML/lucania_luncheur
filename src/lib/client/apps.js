// Liste des applications : état local et rendu des cartes (clonées depuis #tpl-app, mises à jour en place).
import { applyI18n, getLang, t } from './i18n.js';
import { downloadProgress } from './format.js';
import { inTauri } from './tauri.js';
import { matches } from './filters.js';
import { closeMenus } from './menu.js';

/**
 * @typedef {'install'|'update'|'uninstall'} Busy
 * @typedef {{
 *   id: string, name: string, description: { fr: string, en: string }, icon: string, repo: string,
 *   installed: boolean, installedVersion: string|null, latestVersion: string|null,
 *   updateAvailable: boolean, running: boolean, busy: Busy|null, releaseUrl: string|null
 * }} AppView
 * @typedef {{ id: string, phase: 'download'|'closing'|'running', downloaded: number, total: number|null }} AppProgress
 */

const DEFAULT_ICON = '/app-icons/default.svg';
/** Nombre de couleurs de pastille (voir global.css). */
const TONES = 8;

/** @type {AppView[]} */
let apps = [];
let loaded = false;
/** Opérations en cours lancées depuis cette fenêtre (« launch » : lancement, sans barre de progression). */
const localOps = /** @type {Map<string, Busy|'launch'>} */ (new Map());
const progress = /** @type {Map<string, AppProgress>} */ (new Map());

/** @param {string} selector */
const $ = (selector) => document.querySelector(selector);

export const getApps = () => apps;
/** @param {string} id */
export const findApp = (id) => apps.find((a) => a.id === id);

/** Opération en cours sur une application (backend, sinon locale). @param {AppView} app */
export const currentOp = (app) => app.busy ?? localOps.get(app.id) ?? null;

/** Nouvelle liste venue du backend. @param {AppView[]} list */
export function setApps(list) {
  apps = Array.isArray(list) ? list : [];
  loaded = true;
  for (const id of [...progress.keys()]) {
    const app = findApp(id);
    if (!app || !currentOp(app)) progress.delete(id);
  }
  render();
}

/** Marque la fin du premier chargement (liste vide possible). */
export function markLoaded() {
  loaded = true;
  render();
}

/** @param {string} id @param {Busy|'launch'|null} op */
export function setLocalOp(id, op) {
  if (op) localOps.set(id, op);
  else localOps.delete(id);
  refreshCard(id);
}

/** @param {string} id @param {AppProgress|null} p */
export function setProgress(id, p) {
  const app = findApp(id);
  if (!p || !app || !currentOp(app)) progress.delete(id);
  else progress.set(id, p);
  refreshCard(id);
}

/** URL de l'icône : nom de fichier local ou URL raw.githubusercontent.com. @param {string} icon */
function iconUrl(icon) {
  if (icon.startsWith('https://')) return icon;
  if (!icon || icon.includes('/') || icon.includes('\\') || icon.includes('..')) return DEFAULT_ICON;
  return `/app-icons/${icon}`;
}

/** Texte et valeur de la barre de progression. @param {Busy} op @param {AppProgress|undefined} p */
function progressView(op, p) {
  if (op === 'uninstall') return { text: t('common.progress.uninstalling'), value: null };
  if (!p) return { text: t('common.progress.starting'), value: null };
  if (p.phase === 'download') {
    const d = downloadProgress(p.downloaded, p.total, { withTotal: 'common.progress.download', noTotal: 'common.progress.downloadNoTotal' });
    return { text: t(d.key, d.params), value: d.value };
  }
  return { text: t(p.phase === 'closing' ? 'common.progress.closing' : 'common.progress.running'), value: null };
}

/** Texte de la ligne de version. @param {AppView} app */
function versionText(app) {
  const latest = app.latestVersion ?? '–';
  if (!app.installed) return t('common.card.version', { version: latest });
  const installed = app.installedVersion ?? '–';
  if (app.updateAvailable) return t('common.card.updateFromTo', { installed, latest });
  return t('common.card.installedOk', { version: installed });
}

/** Met à jour une tuile existante, sans la reconstruire. @param {HTMLElement} card @param {AppView} app */
function updateCard(card, app) {
  /** @param {string} sel */
  const q = (sel) => /** @type {HTMLElement} */ (card.querySelector(sel));
  const op = currentOp(app);
  const working = op !== null;
  const p = progress.get(app.id);

  // Position dans la liste complète : numéro et couleur restent stables quel que soit le filtre
  const index = Math.max(0, apps.findIndex((a) => a.id === app.id));
  card.dataset.tone = String(index % TONES);
  q('[data-index]').textContent = String(index + 1).padStart(2, '0');
  q('[data-repo]').textContent = app.repo ?? '';

  const img = /** @type {HTMLImageElement} */ (q('[data-icon]'));
  const src = iconUrl(app.icon);
  if (img.dataset.src !== src) {
    img.dataset.src = src;
    img.src = src;
  }

  q('[data-name]').textContent = app.name;
  q('[data-description]').textContent = app.description?.[getLang()] || app.description?.en || app.description?.fr || '';
  q('[data-version]').textContent = versionText(app);

  const badge = q('[data-badge]');
  const state = !app.installed ? 'none' : app.updateAvailable ? 'update' : 'ok';
  badge.dataset.state = state;
  badge.textContent = t(state === 'none' ? 'common.badge.notInstalled' : state === 'update' ? 'common.badge.updateAvailable' : 'common.badge.upToDate');
  const running = q('[data-running]');
  running.hidden = !(app.installed && app.running);
  running.textContent = t('common.badge.running');

  // Action principale : installer, mettre à jour (plein) ou ouvrir (contour, désactivé si déjà lancée)
  const primary = /** @type {HTMLButtonElement} */ (q('[data-primary]'));
  const label = q('[data-primary-label]');
  if (!app.installed) {
    primary.dataset.action = 'install';
    primary.dataset.variant = 'solid';
    label.textContent = t('common.actions.install');
  } else if (app.updateAvailable) {
    primary.dataset.action = 'update';
    primary.dataset.variant = 'solid';
    label.textContent = t('common.actions.updateTo', { version: app.latestVersion ?? '–' });
  } else {
    primary.dataset.action = 'launch';
    primary.dataset.variant = app.running ? 'muted' : 'outline';
    label.textContent = t(app.running ? 'common.actions.launchRunning' : 'common.actions.launch');
  }
  primary.disabled = working || (primary.dataset.action === 'launch' && app.running);

  // Menu des actions secondaires : masqué s'il est vide
  const launchItem = /** @type {HTMLButtonElement} */ (q('[data-menu-item="launch"]'));
  launchItem.hidden = !(app.installed && app.updateAvailable);
  launchItem.disabled = working || app.running;
  q('[data-menu-launch-label]').textContent = t(app.running ? 'common.actions.launchRunning' : 'common.actions.launch');
  const uninstallItem = /** @type {HTMLButtonElement} */ (q('[data-menu-item="uninstall"]'));
  uninstallItem.hidden = !app.installed;
  uninstallItem.disabled = working;
  const menuEmpty = launchItem.hidden && uninstallItem.hidden;
  q('[data-menu-wrap]').hidden = menuEmpty;
  if (menuEmpty || working) closeMenus(card);

  // Une opération en cours remplace les boutons par la progression
  const showProgress = op !== null && op !== 'launch';
  q('[data-actions]').hidden = showProgress;
  const box = q('[data-progress]');
  box.hidden = !showProgress;
  if (showProgress) {
    const view = progressView(op, p);
    q('[data-progress-text]').textContent = view.text;
    const bar = /** @type {HTMLProgressElement} */ (q('[data-progress-bar]'));
    if (view.value === null) bar.removeAttribute('value');
    else bar.value = view.value;
    // Annuler : actif seulement pendant le téléchargement
    /** @type {HTMLButtonElement} */ (q('[data-action="cancel"]')).disabled = p?.phase !== 'download';
  }
}

/** Crée une carte depuis le <template>. @param {string} id */
function createCard(id) {
  const tpl = /** @type {HTMLTemplateElement} */ ($('#tpl-app'));
  const card = /** @type {HTMLElement} */ (tpl.content.firstElementChild?.cloneNode(true));
  card.dataset.appId = id;
  const img = /** @type {HTMLImageElement} */ (card.querySelector('[data-icon]'));
  // Icône introuvable : icône générique (une seule fois, pour éviter une boucle)
  img.addEventListener('error', () => {
    if (img.dataset.src !== DEFAULT_ICON) {
      img.dataset.src = DEFAULT_ICON;
      img.src = DEFAULT_ICON;
    }
  });
  applyI18n(card);
  return card;
}

/** @param {string} id */
function cardOf(id) {
  const grid = $('[data-grid]');
  return grid ? /** @type {HTMLElement|null} */ ([...grid.children].find((c) => /** @type {HTMLElement} */ (c).dataset.appId === id) ?? null) : null;
}

/** Met à jour une seule carte (progression, opération locale). @param {string} id */
function refreshCard(id) {
  const card = cardOf(id);
  const app = findApp(id);
  if (card && app) updateCard(card, app);
}

/** Rend la liste : squelettes, états vides ou tuiles réconciliées par data-app-id (filtrées par masquage). */
export function render() {
  const grid = $('[data-grid]');
  const skeleton = $('[data-skeleton]');
  const empty = $('[data-empty]');
  const noResults = $('[data-noresults]');
  if (!grid || !skeleton || !empty) return;

  skeleton.hidden = loaded;
  empty.hidden = !(loaded && apps.length === 0);
  empty.textContent = t(inTauri() ? 'common.empty.none' : 'common.empty.noTauri');

  const existing = new Map([...grid.children].map((c) => [/** @type {HTMLElement} */ (c).dataset.appId ?? '', /** @type {HTMLElement} */ (c)]));
  let visible = 0;
  apps.forEach((app, i) => {
    const card = existing.get(app.id) ?? createCard(app.id);
    existing.delete(app.id);
    updateCard(card, app);
    const show = matches(app);
    card.hidden = !show;
    if (show) visible++;
    if (grid.children[i] !== card) grid.insertBefore(card, grid.children[i] ?? null);
  });
  for (const stale of existing.values()) stale.remove();

  // Les filets de la grille ne s'affichent que s'il y a des tuiles
  grid.hidden = visible === 0;
  if (noResults) noResults.hidden = !(loaded && apps.length > 0 && visible === 0);
  document.dispatchEvent(new CustomEvent('lanceur:apps'));
}

/** Clic sur un bouton d'action d'une carte. @param {(action: string, id: string) => void} handler */
export function initGrid(handler) {
  const grid = $('[data-grid]');
  grid?.addEventListener('click', (event) => {
    const btn = /** @type {HTMLButtonElement|null} */ (/** @type {HTMLElement} */ (event.target).closest('[data-action]'));
    const card = /** @type {HTMLElement|null} */ (btn?.closest('[data-app-id]'));
    if (!btn || btn.disabled || !card?.dataset.appId) return;
    handler(btn.dataset.action ?? '', card.dataset.appId);
  });
  // Changement de langue : textes des cartes à refaire
  document.addEventListener('lanceur:lang', render);
}
