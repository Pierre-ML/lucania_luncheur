// En-tête : boutons « Actualiser » et « Tout mettre à jour ».
import { refresh, updateAll } from './actions.js';
import { getApps } from './apps.js';
import { inTauri } from './tauri.js';

export function initHeader() {
  const refreshBtn = /** @type {HTMLButtonElement|null} */ (document.querySelector('[data-refresh]'));
  const spinner = document.querySelector('[data-refresh-spinner]');
  const icon = document.querySelector('[data-refresh-icon]');
  const allBtn = /** @type {HTMLButtonElement|null} */ (document.querySelector('[data-update-all]'));
  if (!refreshBtn || !spinner || !icon || !allBtn) return;

  if (!inTauri()) {
    refreshBtn.disabled = true;
    return;
  }

  // Le bouton « Tout mettre à jour » n'apparaît que s'il y a au moins une mise à jour
  const syncAll = () => {
    allBtn.hidden = !getApps().some((a) => a.installed && a.updateAvailable);
  };
  document.addEventListener('lanceur:apps', syncAll);

  refreshBtn.addEventListener('click', async () => {
    refreshBtn.disabled = true;
    spinner.removeAttribute('hidden');
    icon.setAttribute('hidden', '');
    try {
      await refresh({ force: true });
    } finally {
      refreshBtn.disabled = false;
      spinner.setAttribute('hidden', '');
      icon.removeAttribute('hidden');
    }
  });

  allBtn.addEventListener('click', async () => {
    allBtn.disabled = true;
    try {
      await updateAll();
    } finally {
      allBtn.disabled = false;
    }
  });
}
