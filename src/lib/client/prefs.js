// Préférences (langue) : bascule de l'en-tête. Source durable : config_get / config_set.
import { call, inTauri } from './tauri.js';
import { LANGS, applyI18n, getLang, setLang } from './i18n.js';
import { getTheme } from './theme.js';

/** Enregistre langue et thème dans la config de l'app (hors Tauri : cache local seulement). */
function save() {
  if (!inTauri()) return;
  call('config_set', { config: { lang: getLang(), theme: getTheme() } }).catch(() => {
    /* le cache local suffit si l'écriture échoue */
  });
}

/** Aligne les contrôles sur la langue et le thème courants. */
function syncControls() {
  for (const btn of document.querySelectorAll('[data-lang-btn]')) {
    btn.setAttribute('aria-pressed', String(/** @type {HTMLElement} */ (btn).dataset.lang === getLang()));
  }
}

/** Remplit le sélecteur de thème et branche les contrôles. */
export function initPrefs() {
  for (const btn of document.querySelectorAll('[data-lang-btn]')) {
    btn.addEventListener('click', () => {
      const lang = /** @type {HTMLElement} */ (btn).dataset.lang ?? '';
      if (!LANGS.includes(lang) || lang === getLang()) return;
      setLang(lang);
      save();
    });
  }
  document.addEventListener('lanceur:lang', syncControls);
  applyI18n();
  syncControls();
}

/** Charge la config sauvegardée et l'applique si elle diffère du cache local. */
export async function loadPrefs() {
  if (!inTauri()) return;
  try {
    /** @type {{ lang: string|null, theme: string|null }} */
    const config = await call('config_get');
    if (config?.lang && LANGS.includes(config.lang)) setLang(config.lang);
    syncControls();
    // Config vide : on y inscrit les valeurs détectées (cache local ou navigateur)
    if (!config?.lang || !config?.theme) save();
  } catch {
    /* config illisible : valeurs locales conservées */
  }
}
