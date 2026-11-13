// Erreurs du backend : chaîne « code » ou « code:détail », traduite par common.errors.<code>.
import { hasKey, t } from './i18n.js';

/** @param {unknown} err */
function rawError(err) {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}

/** Code de l'erreur (partie avant « : »). @param {unknown} err */
export function errorCode(err) {
  return rawError(err).split(':', 1)[0].trim();
}

/** Message lisible pour l'utilisateur. @param {unknown} err */
export function errorMessage(err) {
  const raw = rawError(err);
  const index = raw.indexOf(':');
  const code = (index === -1 ? raw : raw.slice(0, index)).trim();
  const detail = index === -1 ? '' : raw.slice(index + 1).trim();
  const key = `common.errors.${code}`;
  if (/^[a-z_]+$/.test(code) && code !== 'generic' && hasKey(key)) {
    return detail ? `${t(key)} (${detail})` : t(key);
  }
  // Code inconnu : message générique + texte brut
  return `${t('common.errors.generic')} : ${raw}`;
}
