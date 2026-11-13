// Accès à Tauri : invoke / listen, avec repli propre hors de l'application (navigateur, npm run dev).
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** Vrai dans l'application de bureau Tauri. */
export const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Appelle une commande du backend.
 * @param {string} cmd @param {Record<string, unknown>} [args]
 * @returns {Promise<any>}
 */
export const call = (cmd, args) => invoke(cmd, args);

/**
 * Écoute un événement du backend ; renvoie la fonction d'arrêt (sans effet hors Tauri).
 * @param {string} name @param {(payload: any) => void} handler
 * @returns {Promise<() => void>}
 */
export async function onEvent(name, handler) {
  if (!inTauri()) return () => {};
  return listen(name, (event) => handler(event.payload));
}
