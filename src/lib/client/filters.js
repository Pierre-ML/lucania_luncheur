// Filtre (onglets) et recherche : état partagé entre la barre d'outils et le rendu de la grille.
import { getLang } from './i18n.js';

/** @typedef {'all'|'installed'|'updates'|'notInstalled'} Filter */

export const FILTERS = /** @type {const} */ (['all', 'installed', 'updates', 'notInstalled']);
const CACHE_KEY = 'lanceur.filter';

/** @param {unknown} value @returns {value is Filter} */
const isFilter = (value) => typeof value === 'string' && FILTERS.includes(/** @type {Filter} */ (value));

/** @type {Filter} */
let filter = 'all';
let query = '';

try {
  const cached = localStorage.getItem(CACHE_KEY);
  if (isFilter(cached)) filter = cached;
} catch {
  /* stockage indisponible */
}

export const getFilter = () => filter;
export const getQuery = () => query;
/** Vrai si un filtre ou une recherche est actif. */
export const isFiltering = () => filter !== 'all' || query.trim() !== '';

/** @param {Filter} next */
export function setFilter(next) {
  filter = isFilter(next) ? next : 'all';
  try {
    localStorage.setItem(CACHE_KEY, filter);
  } catch {
    /* stockage indisponible */
  }
}

/** @param {string} next */
export function setQuery(next) {
  query = next;
}

/** Minuscules sans accents, pour une recherche tolérante. @param {string} text */
export const normalize = (text) => text.normalize('NFD').replace(/\p{M}+/gu, '').toLowerCase();

/** Applications qui correspondent à un onglet. @param {import('./apps.js').AppView} app @param {Filter} f */
export function inFilter(app, f) {
  if (f === 'installed') return app.installed;
  if (f === 'updates') return app.installed && app.updateAvailable;
  if (f === 'notInstalled') return !app.installed;
  return true;
}

/** Filtre et recherche combinés. @param {import('./apps.js').AppView} app */
export function matches(app) {
  if (!inFilter(app, filter)) return false;
  const q = normalize(query.trim());
  if (!q) return true;
  const description = app.description?.[/** @type {'fr'|'en'} */ (getLang())] || app.description?.en || app.description?.fr || '';
  return normalize(`${app.name} ${description}`).includes(q);
}
