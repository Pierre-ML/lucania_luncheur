// Point d'entrée : langue et thème, cycle de vie de la liste d'applications.
import { handleAction, pollList, refresh } from './actions.js';
import { getApps, initGrid, markLoaded, render, setApps, setProgress } from './apps.js';
import { demoApps, demoProgress, isDemo } from './demo.js';
import { initHeader } from './header.js';
import { applyI18n, initLang } from './i18n.js';
import { initMenus } from './menu.js';
import { initPrefs, loadPrefs } from './prefs.js';
import { call, inTauri, onEvent } from './tauri.js';
import { initTheme } from './theme.js';
import { initToolbar } from './toolbar.js';
import { initSelfUpdate } from './update.js';
import { initSelfUninstall } from './uninstall.js';
import { errorMessage } from './errors.js';
import { toast } from './toast.js';

const LIST_EVERY_MS = 5000;
const REFRESH_EVERY_MS = 30 * 60 * 1000;

async function start() {
  // Langue et thème : valeurs en cache tout de suite, config sauvegardée ensuite
  initLang(null);
  initTheme();
  applyI18n();
  initPrefs();
  initGrid(handleAction);
  initHeader();
  initToolbar();
  initMenus();
  render();

  // Hors Tauri (navigateur) : état vide et message discret, aucun appel au backend.
  // Développement seulement, avec ?demo : liste factice pour voir le design.
  if (!inTauri()) {
    if (isDemo()) {
      setApps(await demoApps());
      const busy = getApps().find((a) => a.busy);
      if (busy) setProgress(busy.id, { id: busy.id, ...demoProgress() });
    } else {
      markLoaded();
    }
    return;
  }

  void loadPrefs();
  initSelfUpdate();
  initSelfUninstall();
  await onEvent('app-progress', (p) => setProgress(p.id, p));

  // Affichage immédiat (sans réseau), puis actualisation
  try {
    setApps(await call('apps_list'));
  } catch (err) {
    markLoaded();
    toast('error', errorMessage(err));
  }
  void refresh({ force: false });

  setInterval(() => void pollList(), LIST_EVERY_MS);
  setInterval(() => void refresh({ force: false }), REFRESH_EVERY_MS);
}

void start();
