#!/usr/bin/env python3
"""Apply DailyOS WordPress theme overrides to a synced chrome.js.

Reads chrome.js from stdin, writes the patched version to stdout.

Patches applied:
  1. brandMark()     — read from window.dailyosBrandMark when present
  2. inject()        — merge window.dailyosChrome into body.dataset before injection
  3. inject() tail   — strip dev-only buildReferenceControls() + wireOnboardingNav()
  4. buildNav()      — items honor data-nav-items-json on body (custom website nav)
  5. buildNav()      — per-item href; flat render when items have no `group`
  6. buildNav()      — home button href/label/id from body data attrs
  7. buildNav()      — group render falls back to flat when items lack `group`
  8. buildFolio()    — folio home link href from body.dataset.folioHomeHref
  9a. inject()       — return early when chrome is already present
"""
import sys

src = sys.stdin.read()


def apply(label, before, after):
    if before not in src:
        sys.stderr.write(f"FAIL: patch '{label}' anchor not found in chrome.js\n")
        sys.exit(2)
    return src.replace(before, after, 1)


# Patch 1: brandMark override hook
src = apply(
    "brandMark override",
    "  function brandMark(size, className) {\n    const s = svgEl({",
    "  function brandMark(size, className) {\n"
    "    if (typeof window !== 'undefined' && typeof window.dailyosBrandMark === 'function') {\n"
    "      const node = window.dailyosBrandMark(size, className);\n"
    "      if (node) return node;\n"
    "    }\n"
    "    const s = svgEl({",
)

# Patch 2: inject() — merge window.dailyosChrome into body.dataset
src = apply(
    "inject merge",
    "  function inject() {\n"
    "    const body = document.body;\n"
    "    body.dataset.chrome = body.dataset.chrome || 'on';",
    "  function inject() {\n"
    "    const body = document.body;\n"
    "    body.dataset.chrome = body.dataset.chrome || 'on';\n"
    "\n"
    "    // [WP override] Merge window.dailyosChrome (set by PHP) into body.dataset\n"
    "    if (typeof window !== 'undefined' && window.dailyosChrome) {\n"
    "      for (const [k, v] of Object.entries(window.dailyosChrome)) {\n"
    "        if (v == null) continue;\n"
    "        const camel = k.replace(/^data-/, '').replace(/-([a-z])/g, (_, c) => c.toUpperCase());\n"
    "        body.dataset[camel] = String(v);\n"
    "      }\n"
    "    }",
)

# Patch 3: strip dev-only reference controls + onboarding nav
src = apply(
    "strip dev controls",
    "    // Reference controls are intentionally outside the copied app chrome.\n"
    "    body.append(buildReferenceControls());\n"
    "\n"
    "    wireOnboardingNav(body);",
    "    // [WP override] Reference controls + onboarding nav stripped at sync.",
)

# Patch 4: nav items override
src = apply(
    "nav items override",
    "    const accountsItem = { id: 'accounts', label: 'Accounts', icon: 'building', group: 'entity' };\n"
    "    const projectsItem = { id: 'projects', label: 'Projects', icon: 'folder',   group: 'entity' };\n"
    "    const entityPair = entityMode === 'project' ? [projectsItem, accountsItem] : [accountsItem, projectsItem];\n"
    "\n"
    "    const items = [\n"
    "      { id: 'week',     label: 'This Week', icon: 'calendar',    group: 'main' },\n"
    "      { id: 'emails',   label: 'Mail',      icon: 'mail',        group: 'work' },\n"
    "      { id: 'actions',  label: 'Actions',   icon: 'checksquare', group: 'work' },\n"
    "      { id: 'me',       label: 'Me',        icon: 'usercircle',  group: 'entity' },\n"
    "      { id: 'people',   label: 'People',    icon: 'users',       group: 'entity' },\n"
    "      ...entityPair,\n"
    "      { id: 'dropbox',  label: 'Inbox',     icon: 'inbox',       group: 'admin' },\n"
    "      { id: 'settings', label: 'Settings',  icon: 'settings',    group: 'admin' },\n"
    "    ];",
    "    let items;\n"
    "    const navItemsJson = body.dataset.navItemsJson;\n"
    "    if (navItemsJson) {\n"
    "      try {\n"
    "        items = JSON.parse(navItemsJson);\n"
    "      } catch (e) {\n"
    "        if (typeof console !== 'undefined') console.warn('[dailyos chrome] invalid data-nav-items-json', e);\n"
    "        items = [];\n"
    "      }\n"
    "    } else {\n"
    "      const accountsItem = { id: 'accounts', label: 'Accounts', icon: 'building', group: 'entity' };\n"
    "      const projectsItem = { id: 'projects', label: 'Projects', icon: 'folder',   group: 'entity' };\n"
    "      const entityPair = entityMode === 'project' ? [projectsItem, accountsItem] : [accountsItem, projectsItem];\n"
    "\n"
    "      items = [\n"
    "        { id: 'week',     label: 'This Week', icon: 'calendar',    group: 'main' },\n"
    "        { id: 'emails',   label: 'Mail',      icon: 'mail',        group: 'work' },\n"
    "        { id: 'actions',  label: 'Actions',   icon: 'checksquare', group: 'work' },\n"
    "        { id: 'me',       label: 'Me',        icon: 'usercircle',  group: 'entity' },\n"
    "        { id: 'people',   label: 'People',    icon: 'users',       group: 'entity' },\n"
    "        ...entityPair,\n"
    "        { id: 'dropbox',  label: 'Inbox',     icon: 'inbox',       group: 'admin' },\n"
    "        { id: 'settings', label: 'Settings',  icon: 'settings',    group: 'admin' },\n"
    "      ];\n"
    "    }",
)

# Patch 5: per-item href + tag selection
src = apply(
    "per-item href",
    "    const isActive = (id) => id === active;\n"
    "    const activeClass = N('active' + capitalize(tint));\n"
    "    const navTag = navBase ? 'a' : 'button';\n"
    "\n"
    "    function navAttrs(item) {\n"
    "      const attrs = {\n"
    "        class: N('navIslandItem') + (isActive(item.id) ? ' ' + activeClass : ''),\n"
    "        title: item.label,\n"
    "        'data-label': item.label,\n"
    "        'aria-label': item.label,\n"
    "      };\n"
    "      if (navBase) attrs.href = navBase + '/' + (item.id === 'dropbox' ? 'inbox' : item.id) + '.html';\n"
    "      else attrs.type = 'button';\n"
    "      return attrs;\n"
    "    }\n"
    "\n"
    "    function renderItem(item) {\n"
    "      const node = el(navTag, navAttrs(item));",
    "    const isActive = (id) => id === active;\n"
    "    const activeClass = N('active' + capitalize(tint));\n"
    "    const navTag = navBase ? 'a' : 'button';\n"
    "\n"
    "    // [WP override] Per-item href when navBase is absent (website mode)\n"
    "    function tagFor(item) { return item.href ? 'a' : navTag; }\n"
    "\n"
    "    function navAttrs(item) {\n"
    "      const attrs = {\n"
    "        class: N('navIslandItem') + (isActive(item.id) ? ' ' + activeClass : ''),\n"
    "        title: item.label,\n"
    "        'data-label': item.label,\n"
    "        'aria-label': item.label,\n"
    "      };\n"
    "      if (item.href) attrs.href = item.href;\n"
    "      else if (navBase) attrs.href = navBase + '/' + (item.id === 'dropbox' ? 'inbox' : item.id) + '.html';\n"
    "      else attrs.type = 'button';\n"
    "      return attrs;\n"
    "    }\n"
    "\n"
    "    function renderItem(item) {\n"
    "      const node = el(tagFor(item), navAttrs(item));",
)

# Patch 6: home button href/label/id
src = apply(
    "home button override",
    "    const homeAttrs = {\n"
    "      class: N('navIslandMark') + (active === 'today' ? ' ' + N('navIslandMarkActive') : ''),\n"
    "      title: 'Today',\n"
    "      'data-label': 'Today',\n"
    "      'aria-label': 'Today',\n"
    "    };\n"
    "    if (navBase) {\n"
    "      homeAttrs.href = navBase + '/briefing.html';\n"
    "      globalPill.append(el('a', homeAttrs, brandMark(16)));\n"
    "    } else {\n"
    "      homeAttrs.type = 'button';\n"
    "      globalPill.append(el('button', homeAttrs, brandMark(16)));\n"
    "    }",
    "    // [WP override] Home button href/label/id from body data attrs\n"
    "    const homeId = body.dataset.navHomeId || 'today';\n"
    "    const homeLabel = body.dataset.navHomeLabel || 'Today';\n"
    "    const homeHref = body.dataset.navHomeHref || (navBase ? navBase + '/briefing.html' : null);\n"
    "    const homeAttrs = {\n"
    "      class: N('navIslandMark') + (active === homeId ? ' ' + N('navIslandMarkActive') : ''),\n"
    "      title: homeLabel,\n"
    "      'data-label': homeLabel,\n"
    "      'aria-label': homeLabel,\n"
    "    };\n"
    "    if (homeHref) {\n"
    "      homeAttrs.href = homeHref;\n"
    "      globalPill.append(el('a', homeAttrs, brandMark(16)));\n"
    "    } else {\n"
    "      homeAttrs.type = 'button';\n"
    "      globalPill.append(el('button', homeAttrs, brandMark(16)));\n"
    "    }",
)

# Patch 7: group render — fall back to flat when items lack `group`
src = apply(
    "flat render fallback",
    "    items.filter(i => i.group === 'main').forEach(i => globalPill.append(renderItem(i)));\n"
    "    globalPill.append(el('div', { class: N('navIslandDivider'), 'aria-hidden': 'true' }));\n"
    "    items.filter(i => i.group === 'work').forEach(i => globalPill.append(renderItem(i)));\n"
    "    globalPill.append(el('div', { class: N('navIslandDivider'), 'aria-hidden': 'true' }));\n"
    "    items.filter(i => i.group === 'entity').forEach(i => globalPill.append(renderItem(i)));\n"
    "    globalPill.append(el('div', { class: N('navIslandDivider'), 'aria-hidden': 'true' }));\n"
    "    items.filter(i => i.group === 'admin').forEach(i => globalPill.append(renderItem(i)));",
    "    // [WP override] Flat render when items have no group; grouped render preserves dividers\n"
    "    const _groupOrder = ['main', 'work', 'entity', 'admin'];\n"
    "    const _hasGroups = items.some(i => i.group);\n"
    "    if (_hasGroups) {\n"
    "      let _firstGroup = true;\n"
    "      for (const _g of _groupOrder) {\n"
    "        const _groupItems = items.filter(i => i.group === _g);\n"
    "        if (_groupItems.length === 0) continue;\n"
    "        if (!_firstGroup) globalPill.append(el('div', { class: N('navIslandDivider'), 'aria-hidden': 'true' }));\n"
    "        _groupItems.forEach(i => globalPill.append(renderItem(i)));\n"
    "        _firstGroup = false;\n"
    "      }\n"
    "    } else {\n"
    "      items.forEach(i => globalPill.append(renderItem(i)));\n"
    "    }",
)

# Patch 8: folioHomeHref override
src = apply(
    "folio home href",
    "    const homeLink = el('a', { class: F('folioHomeLink'), href: '#' });",
    "    const homeLink = el('a', { class: F('folioHomeLink'), href: body.dataset.folioHomeHref || '#' });",
)

# Patch 9a: inject() idempotency guard
src = apply(
    "inject idempotency guard",
    "  function inject() {\n"
    "    const body = document.body;",
    "  function inject() {\n"
    "    const body = document.body;\n"
    "    if (body.querySelector('.FolioBar_folio, .FloatingNavIsland_navIslandContainer, .AtmosphereLayer_atmosphere')) return;",
)

header = (
    "/* DailyOS WP theme: this file is synced from .docs/design/reference/_shared/chrome.js\n"
    "   and post-processed by tools/patch-chrome-js.py. Do not edit by hand —\n"
    "   re-run tools/sync-chrome.sh to update. */\n"
)
sys.stdout.write(header + src)
