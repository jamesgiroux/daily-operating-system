/* =============================================================================
   chrome.js — injects FolioBar + AtmosphereLayer + FloatingNavIsland.
   Mirrors src/components/layout/{FolioBar,FloatingNavIsland,AtmosphereLayer}.tsx
   1:1. Class names are scoped (e.g. FolioBar_folioMark) to match what
   scope-modules.py produces, eliminating cross-module collisions.

   Body data attributes:
     data-folio-label    "Account" | "Daily Briefing" | "Weekly Forecast" | etc.
     data-folio-crumbs   ">"-separated breadcrumb chain
     data-folio-status   italic mono status text on right
     data-folio-date     mono center text (e.g. "WEEK 17 · APR 20-24, 2026")
     data-folio-readiness CSV "label,color" pairs ("3 ready,sage|2 building,terracotta")
     data-folio-actions   csv: refresh, reports, tools  (search is always present)
     data-folio-mark      "pulsing"  → animates the brand mark (saffron pulse)
     data-active-page     today|week|emails|actions|me|people|accounts|projects|dropbox|settings
     data-tint            turmeric|larkspur|terracotta|eucalyptus|olive
     data-nav-base        when set, nav items render as <a href="<base>/<id>.html">
     data-chrome          on|off
============================================================================= */

(function () {
  function el(tag, attrs, ...children) {
    const e = document.createElement(tag);
    if (attrs) for (const [k, v] of Object.entries(attrs)) {
      if (k === 'class') e.className = v;
      else if (v !== null && v !== undefined) e.setAttribute(k, v);
    }
    for (const c of children) {
      if (c == null) continue;
      e.append(typeof c === 'string' ? document.createTextNode(c) : c);
    }
    return e;
  }

  function svgEl(attrs) {
    const ns = 'http://www.w3.org/2000/svg';
    const s = document.createElementNS(ns, 'svg');
    for (const [k, v] of Object.entries(attrs || {})) s.setAttribute(k, v);
    return s;
  }

  function svgPath(d) {
    const ns = 'http://www.w3.org/2000/svg';
    const p = document.createElementNS(ns, 'path');
    p.setAttribute('d', d);
    return p;
  }

  // ── BrandMark — direct lift of src/components/ui/BrandMark.tsx ──────────
  function brandMark(size, className) {
    const s = svgEl({
      xmlns: 'http://www.w3.org/2000/svg',
      viewBox: '0 0 433 407',
      width: size || 18,
      height: size || 18,
      'aria-hidden': 'true',
    });
    if (className) s.setAttribute('class', className);
    const p = svgPath('M159 407 161 292 57 355 0 259 102 204 0 148 57 52 161 115 159 0H273L271 115L375 52L433 148L331 204L433 259L375 355L271 292L273 407Z');
    p.setAttribute('fill', 'currentColor');
    s.append(p);
    return s;
  }

  // ── Lucide icon paths used in DailyOS nav ──────────────────────────────
  const ICONS = {
    calendar:    'M8 2v4M16 2v4M3 10h18M5 4h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z',
    mail:        'M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z|M22 6l-10 7L2 6',
    inbox:       'M22 12h-6l-2 3h-4l-2-3H2|M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z',
    checksquare: 'm9 11 3 3L22 4|M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11',
    usercircle:  'M18 20a6 6 0 0 0-12 0|M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4z|M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z',
    users:       'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2|M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z|M22 21v-2a4 4 0 0 0-3-3.87|M16 3.13a4 4 0 0 1 0 7.75',
    building:    'M6 22V4a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v18Z|M6 12H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2|M18 9h2a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-2|M10 6h4|M10 10h4|M10 14h4|M10 18h4',
    folder:      'M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2z|M2 10h20',
    settings:    'M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z|M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',

    // Chapter icons — for the local nav island chapter pill
    user:          'M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2|M16 7a4 4 0 1 1 -8 0a4 4 0 0 1 8 0',
    link2:         'M9 17H7A5 5 0 0 1 7 7h2|M15 7h2a5 5 0 1 1 0 10h-2|M8 12L16 12',
    shield:        'M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z',
    monitor:       'M2 5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2h-16a2 2 0 0 1-2-2z|M8 21L16 21|M12 17L12 21',
    wrench:        'M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.106-3.105c.32-.322.863-.22.983.218a6 6 0 0 1-8.259 7.057l-7.91 7.91a1 1 0 0 1-2.999-3l7.91-7.91a6 6 0 0 1 7.057-8.259c.438.12.54.662.219.984z',
    target:        'M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0|M6 12a6 6 0 1 0 12 0a6 6 0 1 0 -12 0|M10 12a2 2 0 1 0 4 0a2 2 0 1 0 -4 0',
    filetext:      'M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z|M14 2v6h6|M16 13H8|M16 17H8|M10 9H8',
    paperclip:     'm16 6-8.414 8.586a2 2 0 0 0 2.829 2.829l8.414-8.586a4 4 0 1 0-5.657-5.657l-8.379 8.551a6 6 0 1 0 8.485 8.485l8.379-8.551',
    alignleft:     'M21 5H3|M15 12H3|M17 19H3',
    crosshair:     'M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0|M22 12h-4|M6 12H2|M12 6V2|M12 22v-4',
    alerttriangle: 'M21.73 18l-8-14a2 2 0 0 0-3.46 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3z|M12 9v4|M12 17h.01',
    network:       'M16 17a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1z|M2 17a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1z|M9 3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1z|M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3|M12 12V8',
    eye:           'M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7z|M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
    activity:      'M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2',
    barchart2:     'M18 20V10|M12 20V4|M6 20v-6',
    star:          'm12 2 3.09 6.26 6.91 1-5 4.87 1.18 6.88L12 17.77l-6.18 3.24L7 14.13 2 9.27l6.91-1L12 2z',
    arrowright:    'M5 12h14|m12 5 7 7-7 7',
    compass:       'M16.24 7.76l-2.12 6.36-6.36 2.12 2.12-6.36 6.36-2.12z|M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0',
    layoutgrid:    'M4 3h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z|M15 3h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z|M15 14h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z|M4 14h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z',
    lightbulb:     'M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5|M9 18h6|M10 22h4',
    trendingup:    'M16 7h6v6|m22 7-8.5 8.5-5-5L2 17',
    trendingdown:  'M16 17h6v-6|M22 17 13.5 8.5 8.5 13.5 2 7',
    briefcase:     'M16 20V4a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16|M2 8a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2h-16a2 2 0 0 1-2-2z',
    notebookpen:   'M13.4 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-7.4|M2 6h4|M2 10h4|M2 14h4|M2 18h4|M21.378 5.626a1 1 0 1 0-3.004-3.004l-5.01 5.012a2 2 0 0 0-.506.854l-.837 2.87a.5.5 0 0 0 .62.62l2.87-.837a2 2 0 0 0 .854-.506z',
    hearthandshake:'M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.29 1.51 4.04 3 5.5l7 7|m12 6 8 8|M16.5 17.5 18 19',
    bookopen:      'M12 7v14|M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z',
    hand:          'M18 11V6a2 2 0 0 0-4 0|M14 10V4a2 2 0 0 0-4 0v2|M10 10.5V6a2 2 0 0 0-4 0v8|M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15',
  };

  function lucide(name, opts) {
    opts = opts || {};
    const path = ICONS[name];
    if (!path) return el('span', { class: 'icon-missing' }, '?');
    const svg = svgEl({
      viewBox: '0 0 24 24',
      fill: 'none',
      stroke: 'currentColor',
      'stroke-width': opts.weight || '1.8',
      'stroke-linecap': 'round',
      'stroke-linejoin': 'round',
    });
    if (opts.size) { svg.setAttribute('width', opts.size); svg.setAttribute('height', opts.size); }
    for (const d of path.split('|')) svg.append(svgPath(d));
    return svg;
  }

  function capitalize(s) { return s ? s[0].toUpperCase() + s.slice(1) : ''; }

  // Helper — class names from FolioBar.module.css are prefixed with FolioBar_
  const F = (...names) => names.map(n => 'FolioBar_' + n).join(' ');
  // Helper — class names from FloatingNavIsland.module.css
  const N = (...names) => names.map(n => 'FloatingNavIsland_' + n).join(' ');
  // Helper — class names from AtmosphereLayer.module.css
  const A = (...names) => names.map(n => 'AtmosphereLayer_' + n).join(' ');

  // ── FolioBar — mirrors src/components/layout/FolioBar.tsx output ───────
  function buildFolio(body) {
    const label   = body.dataset.folioLabel   || 'Daily Briefing';
    const crumbs  = (body.dataset.folioCrumbs || '').split('>').map(s => s.trim()).filter(Boolean);
    const status  = body.dataset.folioStatus  || '';
    const dateText = body.dataset.folioDate   || '';
    const readiness = body.dataset.folioReadiness || '';
    const actions = (body.dataset.folioActions || '').split(',').map(s => s.trim()).filter(Boolean);
    const pulsing = body.dataset.folioMark === 'pulsing';

    const left = el('div', { class: F('folioLeft') });

    // BrandMark — receives className directly (no wrapping span). Matches FolioBar.tsx.
    const homeLink = el('a', { class: F('folioHomeLink'), href: '#' });
    const markClass = pulsing ? F('folioMark', 'folioMarkPulsing') : F('folioMark');
    homeLink.append(brandMark(18, markClass));
    left.append(homeLink);

    if (crumbs.length) {
      const crumbWrap = el('nav', { class: F('folioBreadcrumbs'), 'aria-label': 'Breadcrumb' });
      crumbs.forEach((c, i) => {
        if (i > 0) crumbWrap.append(el('span', { class: F('folioBreadcrumbSeparator') }, '/'));
        const isLast = i === crumbs.length - 1;
        if (isLast) {
          crumbWrap.append(el('span', { class: F('folioBreadcrumbCurrent') }, c));
        } else {
          crumbWrap.append(el('button', { class: F('folioBreadcrumbButton'), type: 'button' }, c));
        }
      });
      left.append(crumbWrap);
    } else {
      left.append(el('span', { class: F('folioPub') }, label));
    }

    const center = dateText ? el('div', { class: F('folioCenter') }, dateText) : null;

    const right = el('div', { class: F('folioRight') });

    if (readiness) {
      const rWrap = el('div', { class: F('folioReadiness') });
      const stats = readiness.indexOf('|') >= 0
        ? readiness.split('|').map(p => { const [label, color] = p.split(','); return { label: label.trim(), color: (color || 'sage').trim() }; })
        : readiness.split(',').map((label, i) => ({ label: label.trim(), color: i === 0 ? 'sage' : 'terracotta' }));
      stats.forEach(s => {
        const stat = el('span', { class: F('folioStat', 'folioStat' + capitalize(s.color)) });
        stat.append(el('span', { class: F('folioDot', 'folioDot' + capitalize(s.color)) }), s.label);
        rWrap.append(stat);
      });
      right.append(rWrap);
    }

    if (status) right.append(el('span', { class: F('folioStatus') }, status));

    // Actions slot — folioActions container wraps page-specific buttons.
    // Refresh button mirrors src/components/ui/folio-refresh-button.tsx exactly:
    // title varies per page (Refresh briefings on week, Refresh on others).
    if (actions.length) {
      const actWrap = el('div', { class: F('folioActions') });
      const refreshTitle = body.dataset.folioRefreshTitle || 'Refresh';
      for (const a of actions) {
        if (a === 'refresh') {
          actWrap.append(el('button', {
            type: 'button',
            title: refreshTitle,
            style: "font-family:var(--font-mono); font-size:11px; font-weight:600; letter-spacing:0.06em; text-transform:uppercase; color:var(--color-text-tertiary); background:none; border:1px solid var(--color-rule-heavy); border-radius:4px; padding:2px 10px; cursor:pointer; transition: color 150ms, border-color 150ms;",
          }, 'Refresh'));
        } else if (a === 'regenerate') {
          actWrap.append(el('button', {
            type: 'button',
            class: 'report-slides_folioAction',
            style: '--report-accent:' + (body.dataset.folioActionAccent || 'var(--color-garden-sage)'),
          }, body.dataset.folioRegenerateLabel || 'Regenerate'));
        }
      }
      right.append(actWrap);
    }

    // Search ⌘K — always present, text-only button. Matches FolioBar.tsx exactly.
    right.append(el('button', {
      class: F('folioSearch'),
      type: 'button',
      'aria-label': 'Open search (⌘K)',
      title: 'Open search',
    }, '⌘K'));

    const folio = el('header', { class: F('folio') }, left);
    if (center) folio.append(center);
    folio.append(right);
    return folio;
  }

  // ── FloatingNavIsland — mirrors src/components/layout/FloatingNavIsland.tsx ──
  function buildNav(body) {
    const active = body.dataset.activePage || '';
    const tint = body.dataset.tint || 'turmeric';
    const navBase = body.dataset.navBase;
    const entityMode = body.dataset.entityMode || 'account';

    const accountsItem = { id: 'accounts', label: 'Accounts', icon: 'building', group: 'entity' };
    const projectsItem = { id: 'projects', label: 'Projects', icon: 'folder',   group: 'entity' };
    const entityPair = entityMode === 'project' ? [projectsItem, accountsItem] : [accountsItem, projectsItem];

    const items = [
      { id: 'week',     label: 'This Week', icon: 'calendar',    group: 'main' },
      { id: 'emails',   label: 'Mail',      icon: 'mail',        group: 'work' },
      { id: 'actions',  label: 'Actions',   icon: 'checksquare', group: 'work' },
      { id: 'me',       label: 'Me',        icon: 'usercircle',  group: 'entity' },
      { id: 'people',   label: 'People',    icon: 'users',       group: 'entity' },
      ...entityPair,
      { id: 'dropbox',  label: 'Inbox',     icon: 'inbox',       group: 'admin' },
      { id: 'settings', label: 'Settings',  icon: 'settings',    group: 'admin' },
    ];

    const isActive = (id) => id === active;
    const activeClass = N('active' + capitalize(tint));
    const navTag = navBase ? 'a' : 'button';

    function navAttrs(item) {
      const attrs = {
        class: N('navIslandItem') + (isActive(item.id) ? ' ' + activeClass : ''),
        title: item.label,
        'data-label': item.label,
        'aria-label': item.label,
      };
      if (navBase) attrs.href = navBase + '/' + (item.id === 'dropbox' ? 'inbox' : item.id) + '.html';
      else attrs.type = 'button';
      return attrs;
    }

    function renderItem(item) {
      const node = el(navTag, navAttrs(item));
      node.append(lucide(item.icon, { size: 18, weight: 1.8 }));
      if (item.id === 'me' && body.dataset.meNeedsContent === 'true') {
        node.append(el('span', { class: N('meContentDot'), 'aria-hidden': 'true' }));
      }
      return node;
    }

    // Parse chapters early so we know whether to apply navIslandGlobalMerged
    // (flattens the global pill's left corners when the local pill attaches).
    const chaptersRaw = body.dataset.chapters || '';
    const chapters = chaptersRaw.split('|').map(s => s.trim()).filter(Boolean).map(spec => {
      const parts = spec.split(':');
      const id = parts[0].trim();
      const icon = (parts[1] || '').trim();
      const label = parts.slice(2).join(':').trim() ||
        id.split('-').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
      return { id, icon, label };
    });
    const hasChapters = chapters.length > 0;

    const globalPill = el('nav', {
      class: N('navIslandGlobal', 'color' + capitalize(tint)) +
             (hasChapters ? ' ' + N('navIslandGlobalMerged') : ''),
      'aria-label': 'App navigation',
    });

    const homeAttrs = {
      class: N('navIslandMark') + (active === 'today' ? ' ' + N('navIslandMarkActive') : ''),
      title: 'Today',
      'data-label': 'Today',
      'aria-label': 'Today',
    };
    if (navBase) {
      homeAttrs.href = navBase + '/briefing.html';
      globalPill.append(el('a', homeAttrs, brandMark(16)));
    } else {
      homeAttrs.type = 'button';
      globalPill.append(el('button', homeAttrs, brandMark(16)));
    }

    items.filter(i => i.group === 'main').forEach(i => globalPill.append(renderItem(i)));
    globalPill.append(el('div', { class: N('navIslandDivider'), 'aria-hidden': 'true' }));
    items.filter(i => i.group === 'work').forEach(i => globalPill.append(renderItem(i)));
    globalPill.append(el('div', { class: N('navIslandDivider'), 'aria-hidden': 'true' }));
    items.filter(i => i.group === 'entity').forEach(i => globalPill.append(renderItem(i)));
    globalPill.append(el('div', { class: N('navIslandDivider'), 'aria-hidden': 'true' }));
    items.filter(i => i.group === 'admin').forEach(i => globalPill.append(renderItem(i)));

    // Local pill — chapter navigation. `chapters` was parsed at the top so
    // we knew whether to apply navIslandGlobalMerged. Mirrors
    // FloatingNavIsland.tsx:264-288 — hidden via navIslandLocalHidden
    // when no chapters are declared.
    const localPill = el('nav', {
      class: N('navIslandLocal') + ' ' + N('color' + capitalize(tint)) +
             (hasChapters ? '' : ' ' + N('navIslandLocalHidden')),
      'aria-label': 'Section navigation',
    });

    chapters.forEach((c, idx) => {
      const node = el('a', {
        class: N('navIslandLocalItem') + (idx === 0 ? ' ' + activeClass : ''),
        href: '#' + c.id,
        title: c.label,
        'data-label': c.label,
        'aria-label': c.label,
      });
      node.append(lucide(c.icon, { size: 18, weight: 1.5 }));
      localPill.append(node);
    });

    // Container — local pill goes first (left), global pill goes second (right).
    return el('div', { class: N('navIslandContainer') }, localPill, globalPill);
  }

  // ── AtmosphereLayer — mirrors src/components/layout/AtmosphereLayer.tsx ──
  function buildAtmosphere(body) {
    const tint = body.dataset.tint || 'turmeric';
    const wrap = el('div', { class: A('atmosphere', tint) });
    const watermark = el('div', { class: A('watermark', 'watermark' + capitalize(tint)) });
    watermark.append(brandMark('100%'));
    wrap.append(watermark);
    return wrap;
  }

  function buildChromeToggle() {
    const t = el('button', { class: 'chrome-toggle', type: 'button' });
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

    // MagazinePageLayout.tsx structure: atmosphere + folio + nav island all
    // live INSIDE the magazinePage div so the cream background doesn't cover
    // the atmosphere gradient. If the surface HTML provides the wrapper, inject
    // there; otherwise fall back to body-level (legacy).
    const wrapper = body.querySelector('.MagazinePageLayout_magazinePage');
    const target = wrapper || body;

    target.prepend(buildAtmosphere(body));
    // Folio bar + nav island are position:fixed so DOM placement is mostly
    // about stacking context. Putting them inside magazinePage matches TSX.
    target.prepend(buildFolio(body));
    target.append(buildNav(body));

    // Chrome toggle is a reference-only control, body-level is fine.
    body.append(buildChromeToggle());
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', inject);
  else inject();
})();
