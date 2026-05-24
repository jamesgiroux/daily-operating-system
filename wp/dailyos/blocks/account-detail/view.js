/**
 * Account detail view tabs.
 *
 * Keeps the WordPress surface aligned with the Tauri account-detail IA while
 * the page is still a server-rendered dynamic block composition.
 */

( function () {
	const VIEW_CHAPTERS = {
		health: [
			'your-assessment',
			'needs-attention',
			'on-track',
			'outlook',
			'relationship-health',
			'portfolio',
			'about-intelligence',
		],
		context: [
			'thesis',
			'the-room',
			'what-matters',
			'value-commitments',
			'their-voice',
			'commercial-shape',
			'technical-shape',
			'relationship-fabric',
			'about-dossier',
		],
		work: [
			'commitments',
			'suggestions',
			'programs',
			'shared',
			'recently-landed',
			'outputs',
			'the-record',
			'files',
			'linear-issues',
		],
	};
	const SHARED_VISIBLE_CHAPTERS = [ 'headline' ];
	const ALL_CHAPTERS = Array.from(
		new Set( [
			...SHARED_VISIBLE_CHAPTERS,
			...Object.values( VIEW_CHAPTERS ).flat(),
		] )
	);
	const VALID_VIEWS = Object.keys( VIEW_CHAPTERS );
	let currentView = 'health';
	let navObserver = null;

	function activeViewFromUrl() {
		try {
			const view = new URL( window.location.href ).searchParams.get(
				'view'
			);
			return VALID_VIEWS.includes( view ) ? view : null;
		} catch ( error ) {
			return null;
		}
	}

	function resolveInitialView( switcher ) {
		const urlView = activeViewFromUrl();
		if ( urlView ) {
			return urlView;
		}
		const active = switcher.querySelector(
			'.AccountViewSwitcher_tabActive[data-view]'
		);
		const view = active ? active.getAttribute( 'data-view' ) : '';
		return VALID_VIEWS.includes( view ) ? view : 'health';
	}

	function setUrlView( view ) {
		if ( ! window.history || ! window.history.replaceState ) {
			return;
		}
		try {
			const url = new URL( window.location.href );
			url.searchParams.set( 'view', view );
			url.hash = '';
			window.history.replaceState( null, '', url.toString() );
		} catch ( error ) {}
	}

	function updateSwitcher( switcher, view ) {
		switcher.querySelectorAll( '[data-view]' ).forEach( ( tab ) => {
			const active = tab.getAttribute( 'data-view' ) === view;
			tab.classList.toggle( 'AccountViewSwitcher_tabActive', active );
			tab.setAttribute( 'aria-pressed', active ? 'true' : 'false' );
			if ( active ) {
				tab.setAttribute( 'aria-current', 'page' );
			} else {
				tab.removeAttribute( 'aria-current' );
			}
		} );
	}

	function updateChapterVisibility( view ) {
		const visible = new Set( [
			...SHARED_VISIBLE_CHAPTERS,
			...VIEW_CHAPTERS[ view ],
		] );
		ALL_CHAPTERS.forEach( ( id ) => {
			const node = document.getElementById( id );
			if ( node ) {
				const show = visible.has( id );
				node.hidden = ! show;
				node.style.display = show ? '' : 'none';
				node.setAttribute( 'aria-hidden', show ? 'false' : 'true' );
			}
		} );
	}

	function updateNavVisibility( view ) {
		const visible = new Set( VIEW_CHAPTERS[ view ] );
		const items = document.querySelectorAll( '[data-chapter-id]' );
		let firstVisible = null;
		let activeClass = '';
		items.forEach( ( item ) => {
			const itemActive = Array.from( item.classList ).find( ( cls ) =>
				cls.includes( 'FloatingNavIsland_active' )
			);
			if ( itemActive ) {
				activeClass = itemActive;
			}
			const id = item.getAttribute( 'data-chapter-id' );
			const show = Boolean(
				id && visible.has( id ) && document.getElementById( id )
			);
			item.hidden = ! show;
			item.style.display = show ? '' : 'none';
			item.setAttribute( 'aria-hidden', show ? 'false' : 'true' );
			if ( show && ! firstVisible ) {
				firstVisible = item;
			}
		} );
		if ( ! activeClass ) {
			activeClass = 'FloatingNavIsland_activeTurmeric';
		}
		items.forEach( ( item ) => item.classList.remove( activeClass ) );
		if ( firstVisible ) {
			firstVisible.classList.add( activeClass );
		}
	}

	function observeChromeNav() {
		if ( navObserver || ! window.MutationObserver || ! document.body ) {
			return;
		}
		navObserver = new MutationObserver( ( mutations ) => {
			const navChanged = mutations.some( ( mutation ) =>
				Array.from( mutation.addedNodes ).some(
					( node ) =>
						node.nodeType === 1 &&
						( ( node.matches &&
							node.matches( '[data-chapter-id]' ) ) ||
							( node.querySelector &&
								node.querySelector( '[data-chapter-id]' ) ) )
				)
			);
			if ( navChanged ) {
				updateNavVisibility( currentView );
			}
		} );
		navObserver.observe( document.body, {
			childList: true,
			subtree: true,
		} );
	}

	function setView( switcher, view, options ) {
		if ( ! VALID_VIEWS.includes( view ) ) {
			return;
		}
		currentView = view;
		const root = document.querySelector( '.wp-block-dailyos-account-detail' );
		document.body.dataset.accountView = view;
		if ( root ) {
			root.setAttribute( 'data-dailyos-account-view', view );
		}
		updateSwitcher( switcher, view );
		updateChapterVisibility( view );
		updateNavVisibility( view );
		window.setTimeout( () => updateNavVisibility( view ), 0 );
		window.setTimeout( () => updateNavVisibility( view ), 50 );
		if ( window.requestAnimationFrame ) {
			window.requestAnimationFrame( () => updateNavVisibility( view ) );
		}
		if ( options && options.updateUrl ) {
			setUrlView( view );
		}
		document.dispatchEvent(
			new CustomEvent( 'dailyos:account-detail-view-change', {
				detail: { view },
			} )
		);
	}

	function init() {
		const switcher = document.querySelector(
			'.AccountViewSwitcher_switcher'
		);
		if ( ! switcher ) {
			return;
		}
		observeChromeNav();
		switcher.addEventListener( 'click', ( event ) => {
			if ( ! event.target || ! event.target.closest ) {
				return;
			}
			const tab = event.target.closest( '[data-view]' );
			if ( ! tab ) {
				return;
			}
			event.preventDefault();
			setView( switcher, tab.getAttribute( 'data-view' ), {
				updateUrl: true,
			} );
		} );
		setView( switcher, resolveInitialView( switcher ), {
			updateUrl: false,
		} );
	}

	if ( document.readyState === 'loading' ) {
		document.addEventListener( 'DOMContentLoaded', init );
	} else {
		init();
	}
} )();
