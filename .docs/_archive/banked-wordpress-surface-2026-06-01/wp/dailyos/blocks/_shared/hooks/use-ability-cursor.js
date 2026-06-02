/**
 * useAbilityCursor — JS runtime mirror of blocks/_shared/hooks/useAbilityCursor.ts.
 *
 * The TS file is the canonical contract (typechecked under wp/dailyos/tsconfig.json).
 * This JS file is the runtime artifact used by hand-coded `view.js` files because
 * the DailyOS WP plugin does not run a JS bundler today (see new-block.mjs L0
 * Packet C V1.3 §5.1). Keep the two in lockstep: any signature change here MUST
 * change the TS, and vice versa.
 *
 * @package DailyOS
 */

( function ( wp ) {
	if ( ! wp || ! wp.element ) {
		return;
	}

	var element = wp.element;
	var useState = element.useState;
	var useEffect = element.useEffect;
	var useCallback = element.useCallback;
	var useMemo = element.useMemo;
	var useRef = element.useRef;

	var INITIAL_CURSOR_STATE = { kind: 'stable' };

	function useAbilityCursor( abilityName, payload, scopeSet, options ) {
		options = options || {};
		var watermark = options.watermark;
		var autoLoad = options.autoLoad === true;

		var itemsState = useState( [] );
		var items = itemsState[ 0 ];
		var setItems = itemsState[ 1 ];

		var cursorState_ = useState( null );
		var cursor = cursorState_[ 0 ];
		var setCursor = cursorState_[ 1 ];

		var cursorLifecycleState = useState( INITIAL_CURSOR_STATE );
		var cursorLifecycle = cursorLifecycleState[ 0 ];
		var setCursorLifecycle = cursorLifecycleState[ 1 ];

		var loadingState = useState( false );
		var loading = loadingState[ 0 ];
		var setLoading = loadingState[ 1 ];

		var errorState = useState( null );
		var error = errorState[ 0 ];
		var setError = errorState[ 1 ];

		var doneState = useState( false );
		var done = doneState[ 0 ];
		var setDone = doneState[ 1 ];

		var payloadKey = useMemo(
			function () {
				try {
					return JSON.stringify( payload );
				} catch ( _e ) {
					return '';
				}
			},
			[ payload ]
		);
		var scopeKey = useMemo(
			function () {
				return ( scopeSet || [] ).join( ',' );
			},
			[ scopeSet ]
		);

		var requestSeqRef = useRef( 0 );
		var isFetchingRef = useRef( false );
		var sentinelObserverRef = useRef( null );

		var reset = useCallback( function () {
			requestSeqRef.current += 1;
			isFetchingRef.current = false;
			setItems( [] );
			setCursor( null );
			setCursorLifecycle( INITIAL_CURSOR_STATE );
			setLoading( false );
			setError( null );
			setDone( false );
		}, [] );

		// Reset on watermark / payload / scope change.
		useEffect(
			function () {
				reset();
			},
			[ watermark, payloadKey, scopeKey, reset ]
		);

		var fetchPage = useCallback(
			function ( nextCursor ) {
				var execute =
					wp && wp.abilities && wp.abilities.executeAbility;
				if ( typeof execute !== 'function' ) {
					setError(
						'wp.abilities.executeAbility unavailable (WP < 7.0)'
					);
					setDone( true );
					return Promise.resolve();
				}
				if ( isFetchingRef.current ) {
					return Promise.resolve();
				}
				isFetchingRef.current = true;
				requestSeqRef.current += 1;
				var seq = requestSeqRef.current;
				setLoading( true );
				setError( null );

				var requestPayload;
				if ( nextCursor === null || nextCursor === undefined ) {
					requestPayload = payload || {};
				} else {
					requestPayload = Object.assign( {}, payload || {}, {
						cursor: nextCursor,
					} );
				}

				return execute( abilityName, requestPayload )
					.then( function ( response ) {
						if ( seq !== requestSeqRef.current ) return;
						var nextItems = Array.isArray( response.items )
							? response.items
							: [];
						setItems( function ( prev ) {
							return nextCursor === null ||
								nextCursor === undefined
								? nextItems
								: prev.concat( nextItems );
						} );
						var nextCursorState =
							response.cursorState || INITIAL_CURSOR_STATE;
						setCursorLifecycle( nextCursorState );
						if (
							nextCursorState.kind === 'invalidated' &&
							nextCursorState.restart_required === true
						) {
							setCursor( null );
							setDone( false );
							setError(
								'cursor invalidated: ' +
									( nextCursorState.reason || '' )
							);
						} else {
							var nc = response.nextCursor || null;
							setCursor( nc );
							setDone( nc === null );
						}
					} )
					.catch( function ( err ) {
						if ( seq !== requestSeqRef.current ) return;
						setError(
							err && err.message ? err.message : String( err )
						);
					} )
					.then( function () {
						if ( seq === requestSeqRef.current ) {
							setLoading( false );
						}
						isFetchingRef.current = false;
					} );
			},
			[ abilityName, payload ]
		);

		// Initial fetch + refetch on key change.
		useEffect(
			function () {
				fetchPage( null );
				// eslint-disable-next-line react-hooks/exhaustive-deps
			},
			[ payloadKey, scopeKey, watermark ]
		);

		var loadMore = useCallback(
			function () {
				if ( loading || done || cursor === null ) return;
				fetchPage( cursor );
			},
			[ loading, done, cursor, fetchPage ]
		);

		var sentinelRef = useCallback(
			function ( node ) {
				if ( ! autoLoad ) return;
				if ( typeof IntersectionObserver === 'undefined' ) return;
				if ( sentinelObserverRef.current ) {
					sentinelObserverRef.current.disconnect();
					sentinelObserverRef.current = null;
				}
				if ( node === null ) return;
				var observer = new IntersectionObserver( function ( entries ) {
					for ( var i = 0; i < entries.length; i++ ) {
						if ( entries[ i ].isIntersecting ) {
							loadMore();
						}
					}
				} );
				observer.observe( node );
				sentinelObserverRef.current = observer;
			},
			[ autoLoad, loadMore ]
		);

		useEffect( function () {
			return function () {
				if ( sentinelObserverRef.current ) {
					sentinelObserverRef.current.disconnect();
					sentinelObserverRef.current = null;
				}
			};
		}, [] );

		var advisory =
			cursorLifecycle && cursorLifecycle.kind === 'data_shifted'
				? cursorLifecycle.advisory
				: null;

		return {
			items: items,
			loading: loading,
			loadMore: loadMore,
			reset: reset,
			done: done,
			advisory: advisory,
			cursorState: cursorLifecycle,
			error: error,
			sentinelRef: sentinelRef,
		};
	}

	wp.dailyosShared = wp.dailyosShared || {};
	wp.dailyosShared.useAbilityCursor = useAbilityCursor;
} )( window.wp );
