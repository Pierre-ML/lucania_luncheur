// i18n côté client (site statique) : langue = config sauvegardée, sinon cache local, sinon langue du navigateur.
// Les dictionnaires sont embarqués au build ; ce module sert aussi au rendu Astro (langue par défaut : fr).
const modules = import.meta.glob('../../i18n/*/*.json', { eager: true, import: 'default' });

export const LANGS = ['fr', 'en'];
const DEFAULT_LANG = 'fr';
const CACHE_KEY = 'lanceur.lang';

// dictionaries[lang][namespace]
/** @type {Record<string, Record<string, any>>} */
const dictionaries = {};
for (const [path, content] of Object.entries(modules)) {
  const m = /\/([^/]+)\/([^/]+)\.json$/.exec(path);
  if (!m) continue;
  (dictionaries[m[1]] ??= {})[m[2]] = content;
}

let lang = DEFAULT_LANG;

/** @param {unknown} value */
const isLang = (value) => typeof value === 'string' && LANGS.includes(value);

export const getLang = () => lang;

/** Langue du navigateur : fr si « fr* », sinon en. */
function browserLang() {
  return (navigator.language || '').toLowerCase().startsWith('fr') ? 'fr' : 'en';
}

/** @param {string} l @param {string} key */
function lookup(l, key) {
  const [ns, ...path] = key.split('.');
  /** @type {any} */
  let node = dictionaries[l]?.[ns];
  for (const part of path) {
    if (node === undefined || typeof node === 'string') return undefined;
    node = node[part];
  }
  return typeof node === 'string' ? node : undefined;
}

/**
 * Traduit une clé (« common.xxx ») avec repli sur le français, puis sur la clé elle-même.
 * @param {string} key @param {Record<string, string|number>} [params]
 */
export function t(key, params) {
  for (const l of lang === DEFAULT_LANG ? [lang] : [lang, DEFAULT_LANG]) {
    const text = lookup(l, key);
    if (text === undefined) continue;
    return text.replace(/\{(\w+)\}/g, (all, name) => (params && name in params ? String(params[name]) : all));
  }
  return key;
}

/** Vrai si la clé existe dans la langue courante ou en français. */
export const hasKey = (/** @type {string} */ key) => lookup(lang, key) !== undefined || lookup(DEFAULT_LANG, key) !== undefined;

/** Remplit les éléments marqués data-i18n (texte), data-i18n-title, data-i18n-label (aria-label) et data-i18n-placeholder. */
export function applyI18n(/** @type {ParentNode} */ root = document) {
  for (const el of root.querySelectorAll('[data-i18n]')) el.textContent = t(/** @type {HTMLElement} */ (el).dataset.i18n ?? '');
  for (const el of root.querySelectorAll('[data-i18n-title]')) {
    el.setAttribute('title', t(/** @type {HTMLElement} */ (el).dataset.i18nTitle ?? ''));
  }
  for (const el of root.querySelectorAll('[data-i18n-label]')) {
    el.setAttribute('aria-label', t(/** @type {HTMLElement} */ (el).dataset.i18nLabel ?? ''));
  }
  for (const el of root.querySelectorAll('[data-i18n-placeholder]')) {
    el.setAttribute('placeholder', t(/** @type {HTMLElement} */ (el).dataset.i18nPlaceholder ?? ''));
  }
}

/**
 * Initialise la langue : valeur préférée (config), sinon cache local, sinon navigateur.
 * @param {string|null} [preferred]
 */
export function initLang(preferred) {
  let cached = null;
  try {
    cached = localStorage.getItem(CACHE_KEY);
  } catch {
    /* stockage indisponible */
  }
  lang = isLang(preferred) ? /** @type {string} */ (preferred) : isLang(cached) ? /** @type {string} */ (cached) : browserLang();
  document.documentElement.lang = lang;
}

/**
 * Change la langue : met à jour le cache, les textes et prévient les modules (événement lanceur:lang).
 * @param {string} next
 */
export function setLang(next) {
  if (!isLang(next) || next === lang) return;
  lang = next;
  try {
    localStorage.setItem(CACHE_KEY, next);
  } catch {
    /* stockage indisponible */
  }
  document.documentElement.lang = next;
  applyI18n();
  document.dispatchEvent(new CustomEvent('lanceur:lang'));
}
