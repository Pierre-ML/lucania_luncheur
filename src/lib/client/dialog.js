// Boîte de confirmation (<dialog> daisyUI) : remplace confirm(), avec case à cocher facultative.
import { t } from './i18n.js';

/** @type {Promise<{ ok: boolean, checked: boolean }>|null} */
let pending = null;

/**
 * @param {{ title: string, message: string, confirmLabel?: string, danger?: boolean, checkboxLabel?: string }} opts
 *   checkboxLabel : affiche une case décochée par défaut ; sa valeur est renvoyée dans `checked`.
 * @returns {Promise<{ ok: boolean, checked: boolean }>}
 */
export function confirmDialog({ title, message, confirmLabel = t('common.close'), danger = false, checkboxLabel }) {
  // Une seule boîte à la fois : les demandes suivantes attendent la fermeture de la précédente.
  if (pending) return pending.then(() => confirmDialog({ title, message, confirmLabel, danger, checkboxLabel }));
  const dlg = /** @type {HTMLDialogElement|null} */ (document.getElementById('confirm-dialog'));
  const titleEl = dlg?.querySelector('[data-confirm-title]');
  const msgEl = dlg?.querySelector('[data-confirm-message]');
  const okBtn = /** @type {HTMLButtonElement|null} */ (dlg?.querySelector('[data-confirm-ok]'));
  const cancelBtn = /** @type {HTMLButtonElement|null} */ (dlg?.querySelector('[data-confirm-cancel]'));
  const option = /** @type {HTMLElement|null} */ (dlg?.querySelector('[data-confirm-option]'));
  const check = /** @type {HTMLInputElement|null} */ (dlg?.querySelector('[data-confirm-check]'));
  const checkText = dlg?.querySelector('[data-confirm-check-label]');
  if (!dlg || !titleEl || !msgEl || !okBtn || !cancelBtn || !option || !check || !checkText) {
    return Promise.resolve({ ok: false, checked: false });
  }

  titleEl.textContent = title;
  msgEl.textContent = message;
  okBtn.textContent = confirmLabel;
  cancelBtn.textContent = t('common.cancel');
  okBtn.dataset.danger = danger ? 'true' : 'false';
  option.hidden = !checkboxLabel;
  checkText.textContent = checkboxLabel ?? '';
  check.checked = false;
  const previous = /** @type {HTMLElement|null} */ (document.activeElement);

  pending = new Promise((resolve) => {
    const onOk = () => dlg.close('ok');
    const onClose = () => {
      okBtn.removeEventListener('click', onOk);
      dlg.removeEventListener('close', onClose);
      pending = null;
      if (previous?.isConnected) previous.focus();
      // Échap, Annuler et clic sur le fond fermeront sans la valeur « ok ».
      const ok = dlg.returnValue === 'ok';
      resolve({ ok, checked: ok && check.checked });
    };
    okBtn.addEventListener('click', onOk);
    dlg.addEventListener('close', onClose);
    dlg.returnValue = '';
    dlg.showModal();
    cancelBtn.focus();
  });
  return pending;
}
