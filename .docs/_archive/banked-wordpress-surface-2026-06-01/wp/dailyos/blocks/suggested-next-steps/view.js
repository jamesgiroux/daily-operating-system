/**
 * Suggested Next Steps feedback affordance view script.
 *
 * W3-A ships the client-side affordance category with explicit disabled
 * guards. W4-A flips FEEDBACK_ENABLED and wires the POST path.
 */
( function () {
	'use strict';

	var FEEDBACK_ENABLED = false;
	var ENGAGEMENT_SIGNALS_ENABLED = false;
	var REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)';

	function prefersReducedMotion() {
		return (
			window.matchMedia &&
			window.matchMedia( REDUCED_MOTION_QUERY ).matches
		);
	}

	function closest( node, selector ) {
		if ( ! node || ! node.closest ) {
			return null;
		}
		return node.closest( selector );
	}

	function toggleMoreFeedback( trigger ) {
		var controls = trigger.getAttribute( 'aria-controls' );
		if ( ! controls ) {
			return;
		}
		var panel = document.getElementById( controls );
		if ( ! panel ) {
			return;
		}

		var isOpen = trigger.getAttribute( 'aria-expanded' ) === 'true';
		var next = ! isOpen;
		trigger.setAttribute( 'aria-expanded', next ? 'true' : 'false' );
		panel.setAttribute( 'aria-hidden', next ? 'false' : 'true' );
	}

	function emitEngagementSignal( container, signal, detail ) {
		if ( ! ENGAGEMENT_SIGNALS_ENABLED ) {
			return;
		}
		window.dispatchEvent(
			new CustomEvent( 'dailyos:suggested-next-steps:engagement', {
				detail: {
					container: container,
					signal: signal,
					payload: detail || {},
				},
			} )
		);
	}

	function postFeedback( row, kind ) {
		if ( ! FEEDBACK_ENABLED ) {
			return Promise.resolve( { ok: false, disabled: true } );
		}

		row.classList.add( 'suggested-next-steps_row--inFlight' );

		var execute =
			window.wp &&
			window.wp.abilities &&
			window.wp.abilities.executeAbility;
		if ( typeof execute !== 'function' ) {
			return Promise.reject( new Error( 'ability_transport_unavailable' ) );
		}

		return execute( 'submit_recommendation_feedback', {
			schemaVersion: 1,
			claimId: row.getAttribute( 'data-claim-id' ) || '',
			feedbackKind: kind,
		} );
	}

	function collapseRow( row ) {
		row.classList.remove( 'suggested-next-steps_row--inFlight' );
		row.classList.add( 'suggested-next-steps_row--collapsing' );
		if ( prefersReducedMotion() ) {
			row.style.maxHeight = '0';
			row.style.opacity = '0';
		}
	}

	function handleAffordanceClick( container, button ) {
		var kind = button.getAttribute( 'data-feedback-kind' );
		var row = closest( button, '.suggested-next-steps_row' );
		if ( ! kind || ! row ) {
			return;
		}

		emitEngagementSignal( container, 'Clicked', {
			claimId: row.getAttribute( 'data-claim-id' ) || '',
			feedbackKind: kind,
		} );

		if ( ! FEEDBACK_ENABLED ) {
			return;
		}

		postFeedback( row, kind )
			.then( function ( response ) {
				if ( response && response.ok ) {
					collapseRow( row );
					return;
				}
				row.classList.remove( 'suggested-next-steps_row--inFlight' );
				row.classList.add( 'suggested-next-steps_row--error' );
			} )
			.catch( function () {
				row.classList.remove( 'suggested-next-steps_row--inFlight' );
				row.classList.add( 'suggested-next-steps_row--error' );
			} );
	}

	function bindContainer( container ) {
		container.addEventListener( 'click', function ( event ) {
			var target = event.target;
			var expand = closest( target, '.suggested-next-steps_buttonExpand' );
			if ( expand && container.contains( expand ) ) {
				event.preventDefault();
				toggleMoreFeedback( expand );
				return;
			}

			var affordance = closest( target, '[data-feedback-kind]' );
			if ( affordance && container.contains( affordance ) ) {
				event.preventDefault();
				handleAffordanceClick( container, affordance );
			}
		} );

		emitEngagementSignal( container, 'Rendered', {
			surface: container.getAttribute( 'data-surface' ) || '',
		} );
	}

	function init() {
		var containers = document.querySelectorAll( '.SuggestedNextSteps_section' );
		for ( var i = 0; i < containers.length; i++ ) {
			bindContainer( containers[ i ] );
		}
	}

	if ( document.readyState === 'loading' ) {
		document.addEventListener( 'DOMContentLoaded', init );
	} else {
		init();
	}
} )();
