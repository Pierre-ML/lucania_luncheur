// Formats de nombres selon la langue courante.
import { getLang } from './i18n.js';

/** Octets en Mo avec une décimale. @param {number} bytes */
export const formatMb = (bytes) =>
  new Intl.NumberFormat(getLang(), { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(bytes / (1024 * 1024));

/** Ratio 0..1 en pourcentage. @param {number} ratio */
export const formatPercent = (ratio) =>
  new Intl.NumberFormat(getLang(), { style: 'percent', maximumFractionDigits: 0 }).format(ratio);

/**
 * Texte et valeur d'avancement d'un téléchargement (valeur null : barre indéterminée).
 * @param {number} downloaded @param {number|null} total
 * @param {{ withTotal: string, noTotal: string }} keys clés i18n
 * @returns {{ key: string, params: Record<string, string>, value: number|null }}
 */
export function downloadProgress(downloaded, total, keys) {
  const done = formatMb(downloaded);
  if (total) {
    const ratio = Math.min(1, downloaded / total);
    return { key: keys.withTotal, params: { percent: formatPercent(ratio), done, total: formatMb(total) }, value: ratio };
  }
  return { key: keys.noTotal, params: { done }, value: null };
}
