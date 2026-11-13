// Bouton « mise à jour du Lanceur » (application de bureau uniquement) :
// vérification silencieuse 3 s après le démarrage puis toutes les 6 h, confirmation, progression, erreurs.
import { call, inTauri, onEvent } from './tauri.js';
import { confirmDialog } from './dialog.js';
import { errorMessage } from './errors.js';
import { downloadProgress } from './format.js';
import { t } from './i18n.js';
import { toast } from './toast.js';

/** @typedef {{ current: string, latest: string, available: boolean, url: string }} UpdateInfo */

const FIRST_CHECK_MS = 3000;
const CHECK_EVERY_MS = 6 * 60 * 60 * 1000;

export function initSelfUpdate() {
  if (!inTauri()) return;
  const wrap = /** @type {HTMLElement|null} */ (document.querySelector('[data-self-wrap]'));
  const btn = /** @type {HTMLButtonElement|null} */ (document.querySelector('[data-self-btn]'));
  const dot = /** @type {HTMLElement|null} */ (document.querySelector('[data-self-dot]'));
  const spinner = document.querySelector('[data-self-spinner]');
  const icon = document.querySelector('[data-self-icon]');
  const panel = /** @type {HTMLElement|null} */ (document.querySelector('[data-self-progress]'));
  const panelText = panel?.querySelector('[data-self-progress-text]');
  const panelBar = /** @type {HTMLProgressElement|null} */ (panel?.querySelector('[data-self-progress-bar]'));
  if (!wrap || !btn || !dot || !spinner || !icon || !panel || !panelText || !panelBar) return;
  wrap.hidden = false;

  /** @type {UpdateInfo|null} */
  let known = null;
  let busy = false;
  let installing = false;

  /** @param {string} text @param {number|null} value */
  const setProgress = (text, value) => {
    panelText.textContent = text;
    if (value === null) panelBar.removeAttribute('value');
    else panelBar.value = value;
    panel.hidden = false;
  };

  const renderLabel = () => {
    const label = known ? t('common.selfUpdate.available', { latest: known.latest }) : t('common.selfUpdate.title');
    btn.title = label;
    btn.setAttribute('aria-label', label);
  };
  /** @param {UpdateInfo|null} info */
  const render = (info) => {
    known = info && info.available ? info : null;
    dot.hidden = !known;
    renderLabel();
  };
  document.addEventListener('lanceur:lang', renderLabel);
  renderLabel();

  /** @returns {Promise<UpdateInfo>} */
  const check = () => call('self_check_update');

  const silentCheck = async () => {
    if (busy || installing) return;
    try {
      render(await check());
    } catch {
      /* vérification silencieuse : erreurs ignorées */
    }
  };

  /** @param {UpdateInfo} info */
  const confirmAndInstall = async (info) => {
    const { ok } = await confirmDialog({
      title: t('common.selfUpdate.confirmTitle'),
      message: t('common.selfUpdate.confirmMessage', { latest: info.latest, current: info.current }),
      confirmLabel: t('common.selfUpdate.confirmOk'),
    });
    if (!ok || installing) return;
    installing = true;
    btn.disabled = true;
    setProgress(t('common.selfUpdate.downloading'), null);
    /** @type {(() => void)|undefined} */
    let unlisten;
    try {
      unlisten = await onEvent('self-update-progress', ({ downloaded, total }) => {
        const d = downloadProgress(downloaded, total, { withTotal: 'common.selfUpdate.progress', noTotal: 'common.selfUpdate.progressNoTotal' });
        setProgress(t(d.key, d.params), d.value);
      });
      await call('self_install_update');
      unlisten();
      setProgress(t('common.selfUpdate.launching'), null);
      // Succès : l'app se ferme d'elle-même ; le bouton reste désactivé (aucun second lancement).
    } catch (err) {
      unlisten?.();
      installing = false;
      panel.hidden = true;
      btn.disabled = false;
      toast('error', errorMessage(err));
    }
  };

  btn.addEventListener('click', async () => {
    if (busy || installing) return;
    busy = true;
    try {
      if (known) {
        await confirmAndInstall(known);
        return;
      }
      btn.disabled = true;
      spinner.removeAttribute('hidden');
      icon.setAttribute('hidden', '');
      /** @type {UpdateInfo} */
      let info;
      try {
        info = await check();
      } catch (err) {
        toast('error', errorMessage(err));
        return;
      } finally {
        btn.disabled = false;
        spinner.setAttribute('hidden', '');
        icon.removeAttribute('hidden');
      }
      render(info);
      if (known) await confirmAndInstall(known);
      else toast('success', t('common.selfUpdate.upToDate', { current: info.current }));
    } finally {
      busy = false;
    }
  });

  setTimeout(silentCheck, FIRST_CHECK_MS);
  setInterval(silentCheck, CHECK_EVERY_MS);
}
