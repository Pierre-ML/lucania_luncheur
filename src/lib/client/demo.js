// Mode démo, réservé au développement : `npm run dev` puis `/?demo` dans un navigateur (hors Tauri).
// Charge une liste factice construite depuis apps.json pour voir les différents états de l'interface.
// Tout ce code est éliminé du build de production (import.meta.env.DEV est faux).
import { inTauri } from './tauri.js';

/** Vrai en développement, dans un navigateur, quand l'URL contient `?demo`. */
export const isDemo = () => import.meta.env.DEV && !inTauri() && new URLSearchParams(location.search).has('demo');

/** États d'exemple attribués dans l'ordre aux applications activées. */
const SAMPLES = [
  { installed: true, installedVersion: '1.4.0', latestVersion: '1.4.0', updateAvailable: false, running: false, busy: null },
  { installed: true, installedVersion: '0.9.2', latestVersion: '1.0.0', updateAvailable: true, running: false, busy: null },
  { installed: false, installedVersion: null, latestVersion: '0.3.1', updateAvailable: false, running: false, busy: null },
  { installed: false, installedVersion: null, latestVersion: '2.1.0', updateAvailable: false, running: false, busy: 'install' },
  { installed: true, installedVersion: '1.0.0', latestVersion: '1.0.0', updateAvailable: false, running: true, busy: null },
  { installed: true, installedVersion: '0.5.0', latestVersion: '0.6.0', updateAvailable: true, running: true, busy: null },
];

const DEMO_TOTAL_BYTES = 131 * 1024 * 1024;

/** Liste factice (vide hors développement). @returns {Promise<import('./apps.js').AppView[]>} */
export async function demoApps() {
  if (!import.meta.env.DEV) return [];
  const { default: catalog } = await import('../../../apps.json');
  return catalog.apps
    .filter((/** @type {{ enabled?: boolean }} */ a) => a.enabled === true)
    .map((/** @type {{ id: string, name: string, description: { fr: string, en: string }, icon: string, repo: string }} */ a, /** @type {number} */ i) => ({
      id: a.id,
      name: a.name,
      description: a.description,
      icon: a.icon,
      repo: a.repo,
      releaseUrl: null,
      ...SAMPLES[i % SAMPLES.length],
    }));
}

/** Progression factice : téléchargement à 37 % de l'application « en cours d'installation ». */
export const demoProgress = () => ({ phase: /** @type {const} */ ('download'), downloaded: Math.round(DEMO_TOTAL_BYTES * 0.37), total: DEMO_TOTAL_BYTES });
