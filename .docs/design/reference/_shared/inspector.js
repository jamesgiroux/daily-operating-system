/**
 * DailyOS design system inspector.
 *
 * Opt-in overlay for reference renders. Press `?` (or Shift+/) to toggle.
 * Hovering any element with data-ds-* attributes shows a label with tier,
 * name, variant, and spec path. Click a labeled element to open its spec.
 *
 * Convention (see SYSTEM-MAP.md → Inspectability):
 *   data-ds-tier="token | primitive | pattern | surface"
 *   data-ds-name="VitalsStrip"
 *   data-ds-spec="primitives/VitalsStrip.md"
 *   data-ds-variant="default"            (optional)
 *   data-ds-state="loading | error | empty"   (optional)
 */
(function () {
  let enabled = false;
  let label = null;
  let labelTier = null;
  let labelName = null;
  let labelVariant = null;
  let labelState = null;
  let labelSpec = null;

  function pill(className) {
    const el = document.createElement('span');
    el.className = className;
    return el;
  }

  function init() {
    label = document.createElement('div');
    label.className = 'ds-inspector-label';
    label.setAttribute('role', 'tooltip');
    label.style.display = 'none';

    labelTier = pill('ds-inspector-tier');
    labelName = pill('ds-inspector-name');
    labelVariant = pill('ds-inspector-variant');
    labelState = pill('ds-inspector-state');
    labelSpec = pill('ds-inspector-spec');

    label.appendChild(labelTier);
    label.appendChild(labelName);
    label.appendChild(labelVariant);
    label.appendChild(labelState);
    label.appendChild(labelSpec);

    document.body.appendChild(label);

    document.addEventListener('keydown', onKey);
    document.addEventListener('mousemove', onMove);
    document.addEventListener('click', onClick, true);
  }

  function onKey(e) {
    const isToggle = e.key === '?' || (e.key === '/' && e.shiftKey);
    if (isToggle) {
      enabled = !enabled;
      document.body.classList.toggle('ds-inspect-on', enabled);
      if (!enabled) hideLabel();
      e.preventDefault();
    } else if (e.key === 'Escape' && enabled) {
      enabled = false;
      document.body.classList.remove('ds-inspect-on');
      hideLabel();
    }
  }

  function onMove(e) {
    if (!enabled) return;
    const el = e.target.closest('[data-ds-name]');
    if (!el) {
      hideLabel();
      return;
    }
    showLabel(el, e.clientX, e.clientY);
  }

  function onClick(e) {
    if (!enabled) return;
    const el = e.target.closest('[data-ds-name]');
    if (!el || !el.dataset.dsSpec) return;
    e.preventDefault();
    e.stopPropagation();
    // Reference renders live at .../reference/<page>.html, specs live at
    // .../<tier>/<Name>.md — go up one level then resolve the spec path.
    window.open('../' + el.dataset.dsSpec, '_blank');
  }

  function setPill(el, text, tierClass) {
    if (text) {
      el.textContent = text;
      el.style.display = '';
    } else {
      el.textContent = '';
      el.style.display = 'none';
    }
    if (tierClass !== undefined) {
      // Reset tier color classes; only one applies at a time.
      el.classList.remove(
        'ds-inspector-tier-token',
        'ds-inspector-tier-primitive',
        'ds-inspector-tier-pattern',
        'ds-inspector-tier-surface',
        'ds-inspector-tier-unknown'
      );
      if (tierClass) el.classList.add(tierClass);
    }
  }

  function showLabel(el, x, y) {
    const tier = el.dataset.dsTier || 'unknown';
    const name = el.dataset.dsName;
    const variant = el.dataset.dsVariant;
    const state = el.dataset.dsState;
    const spec = el.dataset.dsSpec;

    setPill(labelTier, tier, 'ds-inspector-tier-' + tier);
    setPill(labelName, name);
    setPill(labelVariant, variant);
    setPill(labelState, state ? 'state: ' + state : '');
    setPill(labelSpec, spec);

    label.style.display = 'flex';

    // Clamp inside viewport.
    const labelWidth = 320;
    const labelHeight = 100;
    const left = Math.min(x + 12, window.innerWidth - labelWidth - 8);
    const top = Math.min(y + 12, window.innerHeight - labelHeight - 8);
    label.style.left = left + 'px';
    label.style.top = top + 'px';
  }

  function hideLabel() {
    if (label) label.style.display = 'none';
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
