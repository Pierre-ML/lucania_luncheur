// Thèmes daisyUI : attribut data-theme sur <html>, cache local pour éviter le flash au démarrage.
// La source durable est la config de l'app (config_get / config_set), gérée par prefs.js.
export const THEMES = [
  { id: 'lucania', label: { fr: 'Lucania', en: 'Lucania' }, dark: true },
  { id: 'system', label: { fr: 'Système', en: 'System' } },
  { id: 'light', label: { fr: 'Clair', en: 'Light' }, dark: false },
  { id: 'dark', label: { fr: 'Sombre', en: 'Dark' }, dark: true },
  { id: 'dim', label: { fr: 'Dim', en: 'Dim' }, dark: true },
  { id: 'night', label: { fr: 'Nuit', en: 'Night' }, dark: true },
  { id: 'nord', label: { fr: 'Nord', en: 'Nord' }, dark: false },
  { id: 'dracula', label: { fr: 'Dracula', en: 'Dracula' }, dark: true },
  { id: 'cupcake', label: { fr: 'Cupcake', en: 'Cupcake' }, dark: false },
  { id: 'business', label: { fr: 'Business', en: 'Business' }, dark: true },
  { id: 'coffee', label: { fr: 'Café', en: 'Coffee' }, dark: true },
  { id: 'sunset', label: { fr: 'Coucher de soleil', en: 'Sunset' }, dark: true },
  { id: 'emerald', label: { fr: 'Émeraude', en: 'Emerald' }, dark: false },
];

const DEFAULT_THEME = 'lucania';
const CACHE_KEY = 'lanceur.theme';

/** @param {unknown} id */
export const isTheme = (id) => THEMES.some((t) => t.id === id);

let current = DEFAULT_THEME;
/** @type {MediaQueryList|null} */
let systemQuery = null;

/** Thème réellement appliqué : « system » devient « light » ou « dark ». */
function resolveTheme(/** @type {string} */ id) {
  if (id === 'system') return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  return id;
}

const onSystemChange = () => {
  document.documentElement.dataset.theme = resolveTheme('system');
};

export const getTheme = () => current;

/**
 * Applique un thème (valeur inconnue : thème par défaut) et le garde en cache local.
 * @param {string|null|undefined} id
 */
export function applyTheme(id) {
  const next = isTheme(id) ? /** @type {string} */ (id) : DEFAULT_THEME;
  current = next;
  try {
    localStorage.setItem(CACHE_KEY, next);
  } catch {
    /* stockage indisponible */
  }
  systemQuery ??= window.matchMedia('(prefers-color-scheme: light)');
  systemQuery.removeEventListener('change', onSystemChange);
  if (next === 'system') systemQuery.addEventListener('change', onSystemChange);
  document.documentElement.dataset.theme = resolveTheme(next);
}

/** Relit le cache local au chargement (le script inline de Layout a déjà posé data-theme). */
export function initTheme() {
  let cached = null;
  try {
    cached = localStorage.getItem(CACHE_KEY);
  } catch {
    /* stockage indisponible */
  }
  applyTheme(cached);
}
