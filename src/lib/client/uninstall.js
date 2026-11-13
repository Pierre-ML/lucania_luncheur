// Bouton « Désinstaller le lanceur » (application de bureau uniquement) :
// confirmation, puis commande `self_uninstall` (lance le désinstalleur NSIS et ferme l'app).
import { call, inTauri } from './tauri.js';
import { confirmDialog } from './dialog.js';
import { t } from './i18n.js';
import { toast } from './toast.js';

export function initSelfUninstall() {
  const btn = /** @type {HTMLButtonElement|null} */ (document.querySelector('[data-self-uninstall]'));
  if (!btn) return;
  // Hors Tauri (navigateur, démo) : le bouton reste masqué et inoffensif.
  if (!inTauri()) {
    btn.hidden = true;
    return;
  }
  btn.hidden = false;
  let busy = false;

  btn.addEventListener('click', async () => {
    if (busy) return;
    busy = true;
    try {
      const { ok } = await confirmDialog({
        title: t('common.selfUninstall.confirmTitle'),
        message: t('common.selfUninstall.confirmMessage'),
        confirmLabel: t('common.selfUninstall.confirmAction'),
        danger: true,
      });
      if (!ok) return;
      btn.disabled = true;
      try {
        await call('self_uninstall');
        // Succès : le désinstalleur s'affiche et l'app se ferme d'elle-même.
        return;
      } catch {
        btn.disabled = false;
        toast('error', t('common.selfUninstall.failed'));
      }
    } finally {
      busy = false;
    }
  });
}
