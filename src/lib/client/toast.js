// Toasts daisyUI (zone aria-live) : succès brefs, avertissements, erreurs fermables.
import { t } from './i18n.js';

/** Classes complètes par type (jamais construites dynamiquement). */
const KIND_CLASS = {
  success: 'alert-success',
  info: 'alert-info',
  warning: 'alert-warning',
  error: 'alert-error',
};
/** Durée d'affichage avant fermeture automatique. */
const DURATION_MS = { success: 4000, info: 4000, warning: 8000, error: 12000 };
const MAX_TOASTS = 4;

/**
 * @param {'success'|'info'|'warning'|'error'} kind
 * @param {string} text
 */
export function toast(kind, text) {
  const zone = document.getElementById('toasts');
  const tpl = /** @type {HTMLTemplateElement|null} */ (document.getElementById('tpl-toast'));
  const node = /** @type {HTMLElement|null} */ (tpl?.content.firstElementChild?.cloneNode(true));
  if (!zone || !node) return;

  node.classList.add(KIND_CLASS[kind]);
  node.setAttribute('role', kind === 'error' ? 'alert' : 'status');
  const textEl = node.querySelector('[data-toast-text]');
  if (textEl) textEl.textContent = text;

  const close = /** @type {HTMLElement|null} */ (node.querySelector('[data-toast-close]'));
  if (close) {
    close.setAttribute('aria-label', t('common.close'));
    close.hidden = kind !== 'error';
    close.addEventListener('click', () => node.remove());
  }

  zone.append(node);
  while (zone.children.length > MAX_TOASTS) zone.firstElementChild?.remove();
  setTimeout(() => node.remove(), DURATION_MS[kind]);
}
