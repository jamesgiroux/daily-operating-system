/* =============================================================================
   chrome.js — injects FolioBar + AtmosphereLayer + FloatingNavIsland
   Reads <body data-...> attributes for per-page config.

   Body attributes:
     data-folio-label   "Account" | "Briefing" | "Meeting" | etc.
     data-folio-crumbs  "Accounts > Meridian Harbor Holdings"  (>-separated)
     data-folio-status  "Saving..."  (optional italic mono on right)
     data-folio-actions "refresh,reports,tools,search"  (csv)
     data-active-page   "accounts" | "briefing" | "actions" | ...
     data-tint          "turmeric"|"larkspur"|"terracotta"|"eucalyptus"|"olive"|"sage"
     data-chrome        "on" | "off"
============================================================================= */

(function () {
  function el(tag, attrs, ...children) {
    const e = document.createElement(tag);
    if (attrs) for (const [k, v] of Object.entries(attrs)) {
      if (k === 'class') e.className = v;
      else if (k === 'html') e.innerHTML = v;
      else if (v !== null && v !== undefined) e.setAttribute(k, v);
    }
    for (const c of children) {
      if (c == null) continue;
      e.append(typeof c === 'string' ? document.createTextNode(c) : c);
    }
    return e;
  }

  function lucide(name, opts = {}) {
    // Inline SVG copies of Lucide icons used in DailyOS.
    const ICONS = {
      calendar:    'M8 2v4M16 2v4M3 10h18M5 4h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z',
      mail:        'M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z|M22 6l-10 7L2 6',
      checksquare: 'm9 11 3 3L22 4|M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11',
      usercircle:  'M18 20a6 6 0 0 0-12 0|M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4z|M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z',
      users:       'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2|M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z|M22 21v-2a4 4 0 0 0-3-3.87|M16 3.13a4 4 0 0 1 0 7.75',
      building:    'M6 22V4a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v18Z|M6 12H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2|M18 9h2a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-2|M10 6h4|M10 10h4|M10 14h4|M10 18h4',
      folder:      'M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2z|M2 10h20',
      inbox:       'M22 12h-6l-2 3h-4l-2-3H2|M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z',
      settings:    'M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z|M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
      search:      'M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z|m21 21-4.3-4.3',
      refresh:     'M3 12a9 9 0 0 1 15.5-6.36L21 8|M21 3v5h-5|M21 12a9 9 0 0 1-15.5 6.36L3 16|M3 21v-5h5',
      filetext:    'M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z|M14 2v6h6|M16 13H8|M16 17H8|M10 9H8',
      tools:       'm14.7 6.3-1 1a2.83 2.83 0 0 0 0 4l1.4 1.4a2.83 2.83 0 0 0 4 0l1-1c2 2 0 6-2 8s-6 4-8 2L4.4 14a4 4 0 0 1 0-5.65l5.65-5.66a4 4 0 0 1 5.66 0l-1 1z',
      chev:        'm6 9 6 6 6-6',
    };
    const path = ICONS[name];
    if (!path) return el('span', { class: 'icon-missing' }, '?');
    const ns = 'http://www.w3.org/2000/svg';
    const svg = document.createElementNS(ns, 'svg');
    svg.setAttribute('viewBox', '0 0 24 24');
    svg.setAttribute('fill', 'none');
    svg.setAttribute('stroke', 'currentColor');
    svg.setAttribute('stroke-width', opts.weight || '1.8');
    svg.setAttribute('stroke-linecap', 'round');
    svg.setAttribute('stroke-linejoin', 'round');
    if (opts.size) { svg.setAttribute('width', opts.size); svg.setAttribute('height', opts.size); }
    for (const d of path.split('|')) {
      const p = document.createElementNS(ns, 'path');
      p.setAttribute('d', d);
      svg.append(p);
    }
    return svg;
  }

  function buildFolio(body) {
    const label   = body.dataset.folioLabel   || 'Daily Briefing';
    const crumbs  = (body.dataset.folioCrumbs || '').split('>').map(s => s.trim()).filter(Boolean);
    const status  = body.dataset.folioStatus  || '';
    const actions = (body.dataset.folioActions || 'refresh,search').split(',').map(s => s.trim()).filter(Boolean);
    const pulsing = body.dataset.folioMark === 'pulsing';

    const left = el('div', { class: 'folio-left' },
      el('span', { class: 'folio-mark' + (pulsing ? ' pulsing' : '') }, '\u002A'),
      el('span', { class: 'folio-label' }, label.toUpperCase())
    );
    if (crumbs.length) {
      const crumbWrap = el('div', { class: 'folio-crumbs' });
      crumbs.forEach((c, i) => {
        if (i > 0) crumbWrap.append(el('span', { class: 'sep' }, '/'));
        crumbWrap.append(el('a', { href: '#' }, c));
      });
      left.append(crumbWrap);
    }

    const right = el('div', { class: 'folio-right' });
    if (status) right.append(el('span', { class: 'folio-status-text' }, status));
    for (const a of actions) {
      if (a === 'refresh') {
        const b = el('button', { class: 'folio-action' }); b.append(lucide('refresh', { size: 11, weight: 1.6 }), 'Refresh'); right.append(b);
      } else if (a === 'reports') {
        const b = el('button', { class: 'folio-action accent' }); b.append('Reports ', lucide('chev', { size: 10, weight: 2 })); right.append(b);
      } else if (a === 'tools') {
        const b = el('button', { class: 'folio-action' }); b.append('Tools ', lucide('chev', { size: 10, weight: 2 })); right.append(b);
      } else if (a === 'search') {
        const b = el('span', { class: 'folio-search' }); b.append(lucide('search', { size: 11, weight: 1.6 }), 'Search', el('span', { style: 'opacity:0.5; margin-left:6px' }, '\u2318K')); right.append(b);
      } else if (a === 'status-dot') {
        right.append(el('span', { class: 'folio-status-dot', title: 'Live' }));
      }
    }
    return el('header', { class: 'folio' }, left, right);
  }

  function buildNav(body) {
    const active = body.dataset.activePage || '';
    const tint = body.dataset.tint || 'turmeric';
    // navBase: when set (e.g. "../surfaces"), nav items render as anchors
    // linking to <navBase>/<id>.html. When absent, render as buttons (mockup
    // default — no navigation, just visual chrome).
    const navBase = body.dataset.navBase;
    const items = [
      { id: 'briefing', label: 'Briefing',  icon: 'inbox',     tint: 'turmeric' },
      { id: 'week',     label: 'Week',      icon: 'calendar',  tint: 'larkspur' },
      { id: 'inbox',    label: 'Inbox',     icon: 'mail',      tint: 'larkspur' },
      { id: 'actions',  label: 'Actions',   icon: 'checksquare', tint: 'terracotta' },
      { rule: true },
      { id: 'accounts', label: 'Accounts',  icon: 'building',  tint: 'turmeric' },
      { id: 'people',   label: 'People',    icon: 'users',     tint: 'larkspur' },
      { id: 'projects', label: 'Projects',  icon: 'folder',    tint: 'olive' },
      { rule: true },
      { id: 'me',       label: 'Me',        icon: 'usercircle', tint: 'eucalyptus' },
      { id: 'settings', label: 'Settings',  icon: 'settings',  tint: 'turmeric' },
    ];
    const island = el('nav', { class: 'nav-island', 'aria-label': 'Primary' });
    for (const it of items) {
      if (it.rule) { island.append(el('span', { class: 'nav-island-rule' })); continue; }
      const tag = navBase ? 'a' : 'button';
      const attrs = {
        class: 'nav-island-item',
        title: it.label,
        'data-tint': it.tint,
        'aria-current': it.id === active ? 'page' : null,
      };
      if (navBase) attrs.href = navBase + '/' + it.id + '.html';
      const node = el(tag, attrs);
      node.append(lucide(it.icon, { size: 18, weight: 1.8 }));
      island.append(node);
    }
    return island;
  }

  function buildAtmosphere(body) {
    return el('div', { class: 'atmosphere', 'data-tint': body.dataset.tint || 'turmeric' });
  }

  function buildChromeToggle() {
    const t = el('button', { class: 'chrome-toggle' });
    t.append(el('span', { class: 'dot' }), 'Chrome on');
    t.addEventListener('click', () => {
      const off = document.body.dataset.chrome === 'off';
      document.body.dataset.chrome = off ? 'on' : 'off';
      t.classList.toggle('off', !off);
      t.lastChild.textContent = off ? 'Chrome on' : 'Chrome off';
    });
    return t;
  }

  function inject() {
    const body = document.body;
    body.dataset.chrome = body.dataset.chrome || 'on';
    // Atmosphere first (z=0)
    body.prepend(buildAtmosphere(body));
    body.prepend(buildFolio(body));
    body.append(buildNav(body));
    body.append(buildChromeToggle());
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', inject);
  else inject();
})();
