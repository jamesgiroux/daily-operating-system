/**
 * Projects index — client-side hydration.
 *
 * Mounts a React shell at every `[data-dailyos-projects-index-mount]` node.
 * Drives `list_projects` via the shared `useAbilityCursor` hook. No
 * per-row affordance beyond row click-through (the entity-detail block opens
 * via its own surface route).
 */

( function ( wp ) {
	if ( ! wp || ! wp.element ) return;
	if ( ! wp.dailyosShared || ! wp.dailyosShared.useAbilityCursor ) return;

	var element = wp.element;
	var createElement = element.createElement;
	var createRoot = element.createRoot;
	var useAbilityCursor = wp.dailyosShared.useAbilityCursor;

	var MOUNT_SELECTOR = '[data-dailyos-projects-index-mount]';
	var BLOCK_ROOT_SELECTOR = '.wp-block-dailyos-projects-index';

	function getConfigForMount( node ) {
		var root = node.closest( BLOCK_ROOT_SELECTOR );
		if ( ! root ) {
			return { pageSize: 25, watermark: '', ability: 'list_projects' };
		}
		var pageSize = parseInt(
			root.getAttribute( 'data-dailyos-page-size' ) || '25',
			10
		);
		if ( isNaN( pageSize ) || pageSize < 1 ) pageSize = 25;
		return {
			pageSize: pageSize,
			watermark: root.getAttribute( 'data-dailyos-watermark' ) || '',
			ability:
				root.getAttribute( 'data-dailyos-ability' ) || 'list_projects',
		};
	}

	function ProjectsIndexShell( props ) {
		var ability = props.ability;
		var pageSize = props.pageSize;
		var watermark = props.watermark;

		var payload = element.useMemo(
			function () {
				return { depth: 'shallow', page_size: pageSize };
			},
			[ pageSize ]
		);

		var cursor = useAbilityCursor( ability, payload, [], {
			watermark: watermark,
			autoLoad: true,
		} );

		if ( cursor.error && cursor.items.length === 0 ) {
			return createElement(
				'span',
				{
					className: 'dailyos-empty-chip',
					'data-empty-reason': 'load-error',
					role: 'alert',
				},
				cursor.cursorState &&
					cursor.cursorState.kind === 'invalidated' &&
					cursor.cursorState.restart_required
					? 'Refresh to continue'
					: 'Projects unavailable'
			);
		}

		if ( cursor.items.length === 0 && cursor.loading ) {
			return createElement(
				'span',
				{
					className: 'dailyos-empty-chip',
					'data-empty-reason': 'loading',
					'aria-live': 'polite',
				},
				'Loading projects…'
			);
		}

		if ( cursor.items.length === 0 ) {
			return createElement(
				'span',
				{
					className: 'dailyos-empty-chip',
					'data-empty-reason': 'no-projects',
				},
				'No projects to show.'
			);
		}

		var rows = cursor.items.map( function ( item, idx ) {
			var key =
				( item && ( item.id || item.subjectRef || item.subject_ref ) ) ||
				String( idx );
			var label =
				( item &&
					( item.displayLabel ||
						item.display_label ||
						item.name ||
						item.title ) ) ||
				'(unnamed project)';
			return createElement(
				'li',
				{
					key: key,
					className: 'dailyos-projects-index__row',
					'data-project-id': item && item.id ? item.id : null,
				},
				label
			);
		} );

		var advisoryChip =
			cursor.advisory !== null
				? createElement(
						'span',
						{
							className: 'dailyos-empty-chip',
							'data-empty-reason': 'data-shifted',
							role: 'status',
						},
						cursor.advisory
					)
				: null;

		var loadMoreAffordance = cursor.done
			? createElement(
					'span',
					{
						className: 'dailyos-projects-index__end-mark',
						'aria-hidden': 'true',
					},
					'* * *'
				)
			: createElement(
					'button',
					{
						type: 'button',
						className: 'dailyos-projects-index__load-more',
						onClick: cursor.loadMore,
						disabled: cursor.loading,
						ref: cursor.sentinelRef,
					},
					cursor.loading ? 'Loading…' : 'Load more'
				);

		var restartChip =
			cursor.cursorState &&
			cursor.cursorState.kind === 'invalidated' &&
			cursor.cursorState.restart_required
				? createElement(
						'button',
						{
							type: 'button',
							className: 'dailyos-projects-index__refresh',
							onClick: cursor.reset,
						},
						'Refresh to continue'
					)
				: null;

		return createElement(
			'div',
			{ className: 'dailyos-projects-index__shell' },
			advisoryChip,
			createElement(
				'ul',
				{ className: 'dailyos-projects-index__list' },
				rows
			),
			loadMoreAffordance,
			restartChip
		);
	}

	function mountAll() {
		document.querySelectorAll( MOUNT_SELECTOR ).forEach( function ( node ) {
			if ( node._dailyosProjectsIndexMounted ) return;
			node._dailyosProjectsIndexMounted = true;
			var cfg = getConfigForMount( node );
			node.innerHTML = '';
			var root = createRoot( node );
			root.render(
				createElement( ProjectsIndexShell, {
					ability: cfg.ability,
					pageSize: cfg.pageSize,
					watermark: cfg.watermark,
				} )
			);
		} );
	}

	if ( document.readyState === 'loading' ) {
		document.addEventListener( 'DOMContentLoaded', mountAll, {
			once: true,
		} );
	} else {
		mountAll();
	}
} )( window.wp );
