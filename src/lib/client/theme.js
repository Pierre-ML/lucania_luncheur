// Thème unique : « lucania » (attribut data-theme sur <html>).
export const THEMES = [{ id: 'lucania' }];

const DEFAULT_THEME = 'lucania';

/** @param {unknown} id */
export const isTheme = (id) => id === DEFAULT_THEME;

export const getTheme = () => DEFAULT_THEME;

/** Applique le thème (toujours « lucania »). */
export function applyTheme() {
  document.documentElement.dataset.theme = DEFAULT_THEME;
}

export function initTheme() {
  applyTheme();
}
