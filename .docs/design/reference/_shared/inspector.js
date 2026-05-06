/**
 * DailyOS design system inspector.
 *
 * Opt-in overlay for reference renders. Press `?` (or Shift+/) or use the
 * reference controls to toggle. Hovering any element with data-ds-* attributes
 * shows a label with tier, name, variant, and spec path. Surface pages also
 * infer primitives/patterns from known copied CSS module prefixes.
 *
 * Convention (see SYSTEM-MAP.md → Inspectability):
 *   data-ds-tier="token | primitive | pattern | surface"
 *   data-ds-name="VitalsStrip"
 *   data-ds-spec="patterns/VitalsStrip.md"
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
  let activeElement = null;

  const PRIMITIVES = new Set([
    'AsOfTimestamp',
    'Avatar',
    'ConfidenceScoreChip',
    'DataGapNotice',
    'EditableText',
    'EntityChip',
    'FolioRefreshButton',
    'FreshnessIndicator',
    'GlanceCell',
    'HealthBadge',
    'InlineInput',
    'IntelligenceQualityBadge',
    'MeetingStatusPill',
    'Pill',
    'ProvenanceTag',
    'RemovableChip',
    'Segmented',
    'SourceCoverageLine',
    'StatusDot',
    'Switch',
    'TrustBandBadge',
    'TypeBadge',
    'VerificationStatusFlag',
  ]);

  const PATTERNS = new Set([
    'AboutThisIntelligencePanel',
    'AccountViewSwitcher',
    'ActionRow',
    'ActivityLogSection',
    'AgendaThreadList',
    'AtmosphereLayer',
    'BriefingMeetingCard',
    'ChampionHealthBlock',
    'ChapterHeading',
    'ClaimRow',
    'CommitmentRow',
    'ConsistencyFindingBanner',
    'DailyBriefingAttentionSection',
    'DayChart',
    'DayStrip',
    'DiagnosticsSection',
    'DossierSourceCoveragePanel',
    'EntityListShell',
    'EntityRow',
    'EscalationQuote',
    'FindingsTriad',
    'FinisMarker',
    'FloatingNavIsland',
    'FolioActions',
    'FolioBar',
    'FormRow',
    'GlanceRow',
    'InferredActionSelector',
    'IntelligenceFeedback',
    'Lead',
    'MarginGrid',
    'MeetingCard',
    'MeetingSpineItem',
    'OnboardingFlow',
    'PostMeetingIntelligence',
    'PredictionsVsRealityGrid',
    'ReceiptCallout',
    'RoleTransitionRow',
    'SettingsSections',
    'SignalGrid',
    'StaleReportBanner',
    'SuggestedActionRow',
    'SurfaceMasthead',
    'TalkBalanceBar',
    'TrustBand',
    'VitalsStrip',
    'WorkSurface',
    'YouCard',
  ]);

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
    document.addEventListener('dailyos:toggle-inspector', () => setEnabled(!enabled));
    document.addEventListener('dailyos:set-inspector', (e) => setEnabled(Boolean(e.detail && e.detail.enabled)));
  }

  function setEnabled(next) {
    enabled = next;
    document.body.classList.toggle('ds-inspect-on', enabled);
    document.body.dataset.inspect = enabled ? 'on' : 'off';
    if (!enabled) hideLabel();
    document.dispatchEvent(new CustomEvent('dailyos:inspectorchange', { detail: { enabled } }));
  }

  function onKey(e) {
    const isToggle = e.key === '?' || (e.key === '/' && e.shiftKey);
    if (isToggle) {
      setEnabled(!enabled);
      e.preventDefault();
    } else if (e.key === 'Escape' && enabled) {
      setEnabled(false);
    }
  }

  function onMove(e) {
    if (!enabled) return;
    const target = findInspectableTarget(e.target);
    if (!target) {
      hideLabel();
      return;
    }
    showLabel(target, e.clientX, e.clientY);
  }

  function onClick(e) {
    if (!enabled) return;
    const target = findInspectableTarget(e.target);
    if (!target || !target.spec) return;
    e.preventDefault();
    e.stopPropagation();
    window.open(resolveSpecHref(target.spec), '_blank');
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

  function showLabel(target, x, y) {
    setActiveElement(target);

    setPill(labelTier, target.tier || 'unknown', 'ds-inspector-tier-' + (target.tier || 'unknown'));
    setPill(labelName, target.name);
    setPill(labelVariant, target.variant);
    setPill(labelState, target.state ? 'state: ' + target.state : (target.inferred ? 'class match' : ''));
    setPill(labelSpec, target.spec);

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
    setActiveElement(null);
  }

  function setActiveElement(target) {
    if (activeElement) {
      activeElement.classList.remove(
        'ds-inspector-active',
        'ds-inspector-active-token',
        'ds-inspector-active-primitive',
        'ds-inspector-active-pattern',
        'ds-inspector-active-surface',
        'ds-inspector-active-unknown'
      );
    }
    activeElement = target && target.element ? target.element : null;
    if (activeElement) {
      activeElement.classList.add('ds-inspector-active', 'ds-inspector-active-' + (target.tier || 'unknown'));
    }
  }

  function findInspectableTarget(start) {
    let fallback = null;
    let node = start && start.nodeType === 1 ? start : start.parentElement;

    while (node && node !== document.body) {
      if (node.dataset && node.dataset.dsName) {
        const explicit = {
          element: node,
          tier: node.dataset.dsTier || 'unknown',
          name: node.dataset.dsName,
          variant: node.dataset.dsVariant,
          state: node.dataset.dsState,
          spec: node.dataset.dsSpec,
          inferred: false,
        };
        if (explicit.tier === 'primitive' || explicit.tier === 'pattern') return explicit;
        if (!fallback) fallback = explicit;
      }

      const inferred = inferFromClasses(node);
      if (inferred) return inferred;

      node = node.parentElement;
    }

    return fallback;
  }

  function inferFromClasses(node) {
    if (!node || !node.classList) return null;
    for (const className of node.classList) {
      const prefix = className.split('_')[0];
      if (PRIMITIVES.has(prefix)) {
        return {
          element: node,
          tier: 'primitive',
          name: prefix,
          spec: 'primitives/' + prefix + '.md',
          inferred: true,
        };
      }
      if (PATTERNS.has(prefix)) {
        return {
          element: node,
          tier: 'pattern',
          name: prefix,
          spec: 'patterns/' + prefix + '.md',
          inferred: true,
        };
      }
    }
    return null;
  }

  function resolveSpecHref(spec) {
    const segments = window.location.pathname.split('/').filter(Boolean);
    const referenceIndex = segments.lastIndexOf('reference');
    const depthWithinReference = referenceIndex === -1
      ? Math.max(0, segments.length - 1)
      : Math.max(0, segments.length - referenceIndex - 2);
    const depth = depthWithinReference + 1;
    return '../'.repeat(depth) + spec;
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
