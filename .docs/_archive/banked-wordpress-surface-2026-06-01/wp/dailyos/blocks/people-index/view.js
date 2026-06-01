/**
 * People index — client-side hydration.
 *
 * Mounts a React shell at every `[data-dailyos-people-index-mount]` node.
 * Drives `list_people` via the shared `useAbilityCursor` hook and exposes a
 * per-row merge-intent affordance (AC-L.5) that calls
 * `record_claim_feedback` with `FeedbackAction::MergeIntent` — never a direct
 * mutation.
 */

( function ( wp ) {
	if ( ! wp || ! wp.element ) return;
	if ( ! wp.dailyosShared || ! wp.dailyosShared.useAbilityCursor ) return;

	var element = wp.element;
	var createElement = element.createElement;
	var createRoot = element.createRoot;
	var useState = element.useState;
	var useAbilityCursor = wp.dailyosShared.useAbilityCursor;

	var MOUNT_SELECTOR = '[data-dailyos-people-index-mount]';
	var BLOCK_ROOT_SELECTOR = '.wp-block-dailyos-people-index';

	function getConfigForMount( node ) {
		var root = node.closest( BLOCK_ROOT_SELECTOR );
		if ( ! root ) {
			return { pageSize: 25, watermark: '', ability: 'list_people' };
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
				root.getAttribute( 'data-dailyos-ability' ) || 'list_people',
		};
	}

	function emitMergeIntent( personId ) {
		var execute = wp && wp.abilities && wp.abilities.executeAbility;
		if ( typeof execute !== 'function' ) return;
		execute( 'record_claim_feedback', {
			action: {
				kind: 'merge_intent',
				subject_id: personId,
			},
		} ).catch( function () {
			/* swallowed: the affordance shows error state via DOM event upstream */
		} );
	}

	function PeopleIndexShell( props ) {
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

		var pendingMergeState = useState( null );
		var pendingMerge = pendingMergeState[ 0 ];
		var setPendingMerge = pendingMergeState[ 1 ];

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
					: 'People unavailable'
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
				'Loading people…'
			);
		}

		if ( cursor.items.length === 0 ) {
			return createElement(
				'span',
				{
					className: 'dailyos-empty-chip',
					'data-empty-reason': 'no-people',
				},
				'No people to show.'
			);
		}

		var rows = cursor.items.map( function ( item, idx ) {
			var personId =
				item && ( item.id || item.subjectRef || item.subject_ref );
			var key = personId || String( idx );
			var label =
				( item && ( item.displayLabel || item.display_label || item.name ) ) ||
				'(unnamed person)';
			var mergeButton = personId
				? createElement(
						'button',
						{
							type: 'button',
							className: 'dailyos-people-index__merge-intent',
							'data-dailyos-action': 'merge-intent',
							onClick: function () {
								setPendingMerge( personId );
								emitMergeIntent( personId );
							},
							disabled: pendingMerge === personId,
						},
						pendingMerge === personId
							? 'Merge proposed'
							: 'Propose merge'
					)
				: null;
			return createElement(
				'li',
				{
					key: key,
					className: 'dailyos-people-index__row',
					'data-person-id': personId || null,
				},
				createElement(
					'span',
					{ className: 'dailyos-people-index__name' },
					label
				),
				mergeButton
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
						className: 'dailyos-people-index__end-mark',
						'aria-hidden': 'true',
					},
					'* * *'
				)
			: createElement(
					'button',
					{
						type: 'button',
						className: 'dailyos-people-index__load-more',
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
							className: 'dailyos-people-index__refresh',
							onClick: cursor.reset,
						},
						'Refresh to continue'
					)
				: null;

		return createElement(
			'div',
			{ className: 'dailyos-people-index__shell' },
			advisoryChip,
			createElement(
				'ul',
				{ className: 'dailyos-people-index__list' },
				rows
			),
			loadMoreAffordance,
			restartChip
		);
	}

	function mountAll() {
		document.querySelectorAll( MOUNT_SELECTOR ).forEach( function ( node ) {
			if ( node._dailyosPeopleIndexMounted ) return;
			node._dailyosPeopleIndexMounted = true;
			var cfg = getConfigForMount( node );
			node.innerHTML = '';
			var root = createRoot( node );
			root.render(
				createElement( PeopleIndexShell, {
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
