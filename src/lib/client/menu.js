// Menu « … » des tuiles : ouverture au clic ou au clavier, navigation aux flèches,
// fermeture sur Échap, clic extérieur, Tab ou après le choix d'une action.

/** @param {Element} wrap */
const parts = (wrap) => ({
  btn: /** @type {HTMLButtonElement|null} */ (wrap.querySelector('[data-menu-btn]')),
  menu: /** @type {HTMLElement|null} */ (wrap.querySelector('[data-menu]')),
});

/** Entrées actives du menu. @param {HTMLElement} menu */
const items = (menu) => /** @type {HTMLButtonElement[]} */ ([...menu.querySelectorAll('[role="menuitem"]')]).filter((i) => !i.hidden && !i.disabled);

/** Ferme le menu d'une tuile. @param {Element} wrap @param {boolean} [restoreFocus] */
function close(wrap, restoreFocus = false) {
  const { btn, menu } = parts(wrap);
  if (!btn || !menu || menu.hidden) return;
  menu.hidden = true;
  btn.setAttribute('aria-expanded', 'false');
  if (restoreFocus) btn.focus();
}

/** Ferme tous les menus ouverts (ou seulement ceux d'une tuile). @param {ParentNode} [root] */
export function closeMenus(root = document) {
  for (const wrap of root.querySelectorAll('[data-menu-wrap]')) close(wrap);
}

/** @param {Element} wrap @param {'first'|'last'|null} [focus] */
function open(wrap, focus = null) {
  const { btn, menu } = parts(wrap);
  if (!btn || !menu || btn.disabled) return;
  closeMenus();
  const list = items(menu);
  if (list.length === 0) return;
  menu.hidden = false;
  btn.setAttribute('aria-expanded', 'true');
  if (focus) list[focus === 'last' ? list.length - 1 : 0].focus();
}

export function initMenus() {
  document.addEventListener('click', (event) => {
    const target = /** @type {Element} */ (event.target);
    const btn = target.closest('[data-menu-btn]');
    if (btn) {
      const wrap = /** @type {Element} */ (btn.closest('[data-menu-wrap]'));
      const { menu } = parts(wrap);
      if (menu?.hidden === false) close(wrap);
      else open(wrap, event.detail === 0 ? 'first' : null); // détail 0 : activé au clavier
      return;
    }
    const item = target.closest('[role="menuitem"]');
    if (item) {
      close(/** @type {Element} */ (item.closest('[data-menu-wrap]')), true);
      return;
    }
    closeMenus();
  });

  document.addEventListener('keydown', (event) => {
    const target = /** @type {Element} */ (event.target);
    const wrap = target.closest('[data-menu-wrap]');
    if (!wrap) return;
    const { btn, menu } = parts(wrap);
    if (!btn || !menu) return;

    if (event.key === 'Escape' && !menu.hidden) {
      event.preventDefault();
      close(wrap, true);
    } else if (event.key === 'Tab' && !menu.hidden) {
      close(wrap);
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (menu.hidden) {
        open(wrap, event.key === 'ArrowUp' ? 'last' : 'first');
        return;
      }
      const list = items(menu);
      if (list.length === 0) return;
      const at = list.indexOf(/** @type {HTMLButtonElement} */ (document.activeElement));
      const next = event.key === 'ArrowDown' ? (at + 1) % list.length : (at <= 0 ? list.length - 1 : at - 1);
      list[next].focus();
    } else if ((event.key === 'Home' || event.key === 'End') && !menu.hidden) {
      event.preventDefault();
      const list = items(menu);
      list[event.key === 'Home' ? 0 : list.length - 1]?.focus();
    }
  });
}
