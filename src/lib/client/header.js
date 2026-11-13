// En-tête : bouton « Actualiser » ; bouton « Tout mettre à jour » (visibilité et compteur : toolbar.js).
import { refresh, updateAll } from './actions.js';
import { isDemo } from './demo.js';
import { inTauri } from './tauri.js';

export function initHeader() {
  const refreshBtn = /** @type {HTMLButtonElement|null} */ (document.querySelector('[data-refresh]'));
  const spinner = document.querySelector('[data-refresh-spinner]');
  const icon = document.querySelector('[data-refresh-icon]');
  const allBtn = /** @type {HTMLButtonElement|null} */ (document.querySelector('[data-update-all]'));
  if (!refreshBtn || !spinner || !icon || !allBtn) return;

  // Hors application de bureau (et hors mode démo) : rien à actualiser
  if (!inTauri() && !isDemo()) {
    refreshBtn.disabled = true;
    return;
  }

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
