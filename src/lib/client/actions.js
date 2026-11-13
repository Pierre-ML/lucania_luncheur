// Actions : actualisation de la liste, installation, mise à jour, désinstallation, lancement, annulation.
import { call } from './tauri.js';
import { currentOp, findApp, getApps, setApps, setLocalOp } from './apps.js';
import { confirmDialog } from './dialog.js';
import { errorCode, errorMessage } from './errors.js';
import { t } from './i18n.js';
import { toast } from './toast.js';

/** @type {Promise<void>|null} */
let refreshing = null;
let updatingAll = false;

/** Erreur en toast (l'annulation volontaire reste silencieuse). @param {unknown} err */
function reportError(err) {
  if (errorCode(err) === 'cancelled') return;
  toast('error', errorMessage(err));
}

/** Liste locale, sans réseau (affichage immédiat et suivi de « en cours d'exécution »). Erreurs silencieuses sauf `notify`. */
export async function pollList(notify = false) {
  try {
    setApps(await call('apps_list'));
  } catch (err) {
    if (notify) reportError(err);
  }
}

/**
 * Actualise la liste avec les dernières versions (réseau). Un seul avertissement par actualisation.
 * @param {{ force?: boolean, notify?: boolean }} [opts] notify : afficher avertissement et erreur
 */
export function refresh({ force = false, notify = true } = {}) {
  refreshing ??= (async () => {
    try {
      /** @type {{ apps: import('./apps.js').AppView[], warning: string|null }} */
      const res = await call('apps_refresh', { force });
      setApps(res.apps);
      if (notify && res.warning) {
        const key = `common.warnings.${res.warning}`;
        toast('warning', t(key) === key ? res.warning : t(key));
      }
    } catch (err) {
      if (notify) reportError(err);
    } finally {
      refreshing = null;
    }
  })();
  return refreshing;
}

/**
 * Exécute une opération sur une application, avec état local et rafraîchissement ensuite.
 * @param {import('./apps.js').AppView} app
 * @param {import('./apps.js').Busy} op
 * @param {string} cmd
 * @param {Record<string, unknown>} args
 * @param {boolean} [silent] ne pas afficher l'erreur (résumé fait par l'appelant)
 * @returns {Promise<{ ok: boolean, code: string }>}
 */
async function runOp(app, op, cmd, args, silent = false) {
  setLocalOp(app.id, op);
  /** @type {{ ok: boolean, code: string }} */
  let result = { ok: true, code: '' };
  try {
    await call(cmd, args);
  } catch (err) {
    result = { ok: false, code: errorCode(err) };
    if (!silent) reportError(err);
  } finally {
    setLocalOp(app.id, null);
  }
  await pollList();
  void refresh({ force: false, notify: false });
  return result;
}

/** Liste de noms lisible. @param {import('./apps.js').AppView[]} list */
const names = (list) => list.map((a) => a.name).join(', ');

/** Met à jour toutes les applications qui en ont besoin, une par une. */
export async function updateAll() {
  if (updatingAll) return;
  const targets = getApps().filter((a) => a.installed && a.updateAvailable && !currentOp(a));
  if (targets.length === 0) return;
  const running = targets.filter((a) => a.running);
  if (running.length > 0) {
    const { ok } = await confirmDialog({
      title: t('common.updateAllConfirm.title'),
      message: t('common.updateAllConfirm.message', { names: names(running) }),
      confirmLabel: t('common.updateAllConfirm.confirm'),
    });
    if (!ok) return;
  }
  updatingAll = true;
  try {
    const failed = [];
    for (const app of targets) {
      // On continue après un échec ou une annulation
      const res = await runOp(app, 'update', 'app_update', { id: app.id }, true);
      if (!res.ok) failed.push(app);
    }
    if (failed.length === 0) toast('success', t('common.toast.updateAllOk'));
    else {
      toast('error', t('common.toast.updateAllPartial', { ok: targets.length - failed.length, total: targets.length, names: names(failed) }));
    }
  } finally {
    updatingAll = false;
  }
}

/** Actions des boutons d'une carte. @param {string} action @param {string} id */
export async function handleAction(action, id) {
  const app = findApp(id);
  if (!app) return;

  if (action === 'cancel') {
    try {
      await call('app_cancel', { id });
    } catch (err) {
      reportError(err);
    }
    return;
  }
  if (currentOp(app)) return;

  if (action === 'install') {
    const res = await runOp(app, 'install', 'app_install', { id });
    if (res.ok) toast('success', t('common.toast.installed', { name: app.name }));
  } else if (action === 'update') {
    if (app.running) {
      const { ok } = await confirmDialog({
        title: t('common.updateConfirm.title', { name: app.name }),
        message: t('common.updateConfirm.message', { name: app.name }),
        confirmLabel: t('common.updateConfirm.confirm'),
      });
      if (!ok) return;
    }
    const res = await runOp(app, 'update', 'app_update', { id });
    if (res.ok) toast('success', t('common.toast.updated', { name: app.name }));
  } else if (action === 'uninstall') {
    const { ok, checked } = await confirmDialog({
      title: t('common.uninstall.title', { name: app.name }),
      message: t('common.uninstall.message'),
      confirmLabel: t('common.uninstall.confirm'),
      danger: true,
      checkboxLabel: t('common.uninstall.deleteData'),
    });
    if (!ok) return;
    const res = await runOp(app, 'uninstall', 'app_uninstall', { id, deleteData: checked });
    if (res.ok) toast('success', t('common.toast.uninstalled', { name: app.name }));
  } else if (action === 'launch') {
    setLocalOp(id, 'launch');
    try {
      await call('app_launch', { id });
    } catch (err) {
      reportError(err);
    } finally {
      setLocalOp(id, null);
    }
    await pollList();
  }
}
