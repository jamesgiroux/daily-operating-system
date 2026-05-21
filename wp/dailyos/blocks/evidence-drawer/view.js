/**
 * EvidenceDrawer primitive view script (DOS-689).
 *
 * Open/close toggle for the closed-state drawer markup. Stays inside the
 * ADR-0130 §3.1 10-channel allowlist: the panel content is server-rendered
 * once per request from the actor-filtered envelope; the client does NOT
 * fetch raw provenance, IDs, or internal note bodies.
 *
 * Future evidence-fetch (lazy via executeAbility('get_claim_evidence', ...))
 * lands when DOS-689 §"L4 integration" wires the per-claim drill-down at
 * Account Detail; until then, the drawer reveals the server-emitted summary.
 */
( function () {
	'use strict';

	function bindDrawer( root ) {
		var toggle = root.querySelector( '.dailyos-evidence-drawer__toggle' );
		var panel  = root.querySelector( '.dailyos-evidence-drawer__panel' );
		if ( ! toggle || ! panel ) {
			return;
		}
		toggle.addEventListener( 'click', function () {
			var isOpen = root.getAttribute( 'data-open' ) === 'true';
			var next   = ! isOpen;
			root.setAttribute( 'data-open', next ? 'true' : 'false' );
			toggle.setAttribute( 'aria-expanded', next ? 'true' : 'false' );
			if ( next ) {
				panel.removeAttribute( 'hidden' );
			} else {
				panel.setAttribute( 'hidden', '' );
			}
		} );
		toggle.addEventListener( 'keydown', function ( event ) {
			if ( event.key === 'Escape' && root.getAttribute( 'data-open' ) === 'true' ) {
				root.setAttribute( 'data-open', 'false' );
				toggle.setAttribute( 'aria-expanded', 'false' );
				panel.setAttribute( 'hidden', '' );
			}
		} );
	}

	function init() {
		var drawers = document.querySelectorAll( '.dailyos-evidence-drawer' );
		for ( var i = 0; i < drawers.length; i++ ) {
			bindDrawer( drawers[ i ] );
		}
	}

	if ( document.readyState === 'loading' ) {
		document.addEventListener( 'DOMContentLoaded', init );
	} else {
		init();
	}
} )();
