// Barre d'outils : onglets de filtre avec compteurs, recherche, « Tout mettre à jour (n) » et compteur du pied de page.
import { getApps, render } from './apps.js';
import { FILTERS, getFilter, getQuery, inFilter, setFilter, setQuery } from './filters.js';
import { getLang, t } from './i18n.js';

/** Met à jour compteurs, onglet actif, bouton « Tout mettre à jour » et pied de page. */
function sync() {
  const apps = getApps();
  const filter = getFilter();

  for (const f of FILTERS) {
    const count = apps.filter((a) => inFilter(a, f)).length;
    const btn = document.querySelector(`[data-filter="${f}"]`);
    const counter = document.querySelector(`[data-count="${f}"]`);
    if (counter) {
      counter.textContent = String(count);
      /** @type {HTMLElement} */ (counter).dataset.warn = String(f === 'updates' && count > 0);
    }
    btn?.setAttribute('aria-pressed', String(f === filter));
  }

  const updates = apps.filter((a) => inFilter(a, 'updates')).length;
  const allBtn = /** @type {HTMLButtonElement|null} */ (document.querySelector('[data-update-all]'));
  const allCount = document.querySelector('[data-update-count]');
  if (allBtn) allBtn.hidden = updates === 0;
  if (allCount) allCount.textContent = `(${updates})`;

  const footer = document.querySelector('[data-app-count]');
  if (footer) {
    const one = new Intl.PluralRules(getLang()).select(apps.length) === 'one';
    footer.textContent = t(one ? 'common.footer.appsOne' : 'common.footer.appsOther', { count: apps.length });
  }
}

export function initToolbar() {
  const search = /** @type {HTMLInputElement|null} */ (document.querySelector('[data-search]'));

  for (const btn of document.querySelectorAll('[data-filter]')) {
    btn.addEventListener('click', () => {
      setFilter(/** @type {import('./filters.js').Filter} */ (/** @type {HTMLElement} */ (btn).dataset.filter));
      render();
    });
  }

  search?.addEventListener('input', () => {
    setQuery(search.value);
    render();
  });

  document.querySelector('[data-reset-filters]')?.addEventListener('click', () => {
    setFilter('all');
    setQuery('');
    if (search) {
      search.value = '';
      search.focus();
    }
    render();
  });

  // Chaque rendu de la liste (et chaque changement de langue, qui relance un rendu) rafraîchit la barre
  document.addEventListener('lanceur:apps', sync);
  if (search) search.value = getQuery();
  sync();
}
