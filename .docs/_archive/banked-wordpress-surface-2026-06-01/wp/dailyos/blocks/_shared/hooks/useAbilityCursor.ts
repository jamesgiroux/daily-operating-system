/**
 * useAbilityCursor — shared client-side pagination hook for entity list shells.
 *
 * Per L0 Packet W2 V1.2.1 §5.5 + wave-plan §10 "Entity list pagination contract":
 * consumes W1's `Paginated<T>` + `CursorState` shape (see
 * `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/contracts.rs:120-175`)
 * via the WP 7.0 client-side Abilities API (`wp.abilities.executeAbility`), NOT
 * `@wordpress/core-data` `useEntityRecords`. The cursor is opaque + server-
 * signed; the client must never parse it. Reset semantics follow the wave §10
 * invariant: cursor resets when the watermark prop changes, OR when the server
 * returns `CursorState::Invalidated { restart_required: true }`.
 *
 * Path-α: WP 6.x fallback (no `wp.abilities`) routed through the maintenance
 * project (M1 per L0 V1.1 cycle-1 fold). At the W2 L1 shell, we degrade
 * gracefully and surface `unsupported` in the loading flag so callers can
 * render an explicit empty state.
 *
 * @package DailyOS
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

/**
 * Opaque server-signed cursor token. Clients MUST NOT parse, split, or
 * otherwise inspect this value — it is round-tripped as-is.
 */
export type Cursor = string;

/**
 * Per-list cursor lifecycle. Mirrors W1's
 * `abilities-runtime::CursorState` (serde snake_case tag = "kind").
 */
export type CursorState =
	| { kind: "stable" }
	| { kind: "data_shifted"; advisory: string }
	| { kind: "invalidated"; reason: string; restart_required: boolean };

/**
 * Generic paginated envelope wrapper. Mirrors W1's
 * `abilities-runtime::Paginated<T>` (serde camelCase).
 */
export interface Paginated< T > {
	items: T[];
	nextCursor: Cursor | null;
	totalHint: number | null;
	cursorState: CursorState;
}

/**
 * Minimal shape of `window.wp.abilities.executeAbility` per WP 7.0.
 * Typed locally rather than depending on `@wordpress/abilities` so the
 * hand-rolled WP plugin build (no bundler) does not need an extra dep.
 *
 * Note: `window.wp` is also augmented in FeedbackAffordance.tsx with
 * `apiFetch`. We only narrow to the abilities slice we use; the cast at
 * the callsite picks the optional `abilities` field without colliding.
 */
interface WpAbilitiesShape {
	executeAbility: <Payload, Result>(
		ability: string,
		payload: Payload
	) => Promise< Result >;
}

export interface UseAbilityCursorOptions {
	/**
	 * Watermark token from the outer block / envelope. When it changes, the
	 * cursor + items reset. Per wave §10 cache discipline.
	 */
	watermark?: string;
	/**
	 * Auto-load the next page when the sentinel element enters the viewport.
	 * Caller MUST attach `sentinelRef` to a DOM node below the last item.
	 */
	autoLoad?: boolean;
}

export interface UseAbilityCursorResult< T > {
	items: T[];
	loading: boolean;
	loadMore: () => void;
	reset: () => void;
	done: boolean;
	advisory: string | null;
	cursorState: CursorState;
	error: string | null;
	/**
	 * Attach to a DOM node *below* the rendered list. When `autoLoad` is true
	 * and IntersectionObserver is available, intersecting the sentinel calls
	 * `loadMore()`.
	 */
	sentinelRef: ( node: Element | null ) => void;
}

const INITIAL_CURSOR_STATE: CursorState = { kind: "stable" };

/**
 * Pull-based pagination hook for ability-backed list endpoints.
 *
 * @param abilityName Server-side ability id (e.g. `list_accounts`).
 * @param payload Stable payload object; first-page request omits `cursor`,
 *   subsequent calls inject `cursor` from the previous response.
 * @param scopeSet Scope tokens the surface client has been granted. Unused by
 *   the client API today (the runtime authoritatively enforces) but threaded
 *   through for callsite-visibility per ADR-0129 §4.
 * @param options Watermark + auto-load.
 */
export function useAbilityCursor< T >(
	abilityName: string,
	payload: Record< string, unknown >,
	scopeSet: readonly string[],
	options: UseAbilityCursorOptions = {}
): UseAbilityCursorResult< T > {
	const { watermark, autoLoad = false } = options;

	const [ items, setItems ] = useState< T[] >( [] );
	const [ cursor, setCursor ] = useState< Cursor | null >( null );
	const [ cursorState, setCursorState ] = useState< CursorState >(
		INITIAL_CURSOR_STATE
	);
	const [ loading, setLoading ] = useState( false );
	const [ error, setError ] = useState< string | null >( null );
	const [ done, setDone ] = useState( false );

	// Stable payload reference for the dep array; serialize order-independent.
	const payloadKey = useMemo( () => JSON.stringify( payload ), [ payload ] );
	const scopeKey = useMemo( () => scopeSet.join( "," ), [ scopeSet ] );

	const requestSeqRef = useRef( 0 );
	const sentinelObserverRef = useRef< IntersectionObserver | null >( null );
	const isFetchingRef = useRef( false );

	const reset = useCallback( () => {
		requestSeqRef.current += 1;
		isFetchingRef.current = false;
		setItems( [] );
		setCursor( null );
		setCursorState( INITIAL_CURSOR_STATE );
		setLoading( false );
		setError( null );
		setDone( false );
	}, [] );

	// Reset on watermark / payload / scope change. Per wave §10 invariant.
	useEffect( () => {
		reset();
	}, [ watermark, payloadKey, scopeKey, reset ] );

	const fetchPage = useCallback(
		async ( nextCursor: Cursor | null ) => {
			const wpWithAbilities = window.wp as
				| { abilities?: WpAbilitiesShape }
				| undefined;
			const execute = wpWithAbilities?.abilities?.executeAbility;
			if ( typeof execute !== "function" ) {
				setError( "wp.abilities.executeAbility unavailable (WP < 7.0)" );
				setDone( true );
				return;
			}
			if ( isFetchingRef.current ) return;
			isFetchingRef.current = true;
			const seq = ++requestSeqRef.current;
			setLoading( true );
			setError( null );

			try {
				const requestPayload =
					nextCursor === null
						? payload
						: { ...payload, cursor: nextCursor };
				const response = ( await execute(
					abilityName,
					requestPayload
				) ) as Paginated< T >;

				// Drop stale responses if a reset happened mid-flight.
				if ( seq !== requestSeqRef.current ) return;

				setItems( ( prev ) =>
					nextCursor === null
						? response.items
						: prev.concat( response.items )
				);
				setCursorState( response.cursorState );

				if (
					response.cursorState.kind === "invalidated" &&
					response.cursorState.restart_required
				) {
					// Server told us to restart: drop pagination state but
					// keep error surfacing so caller can toast.
					setCursor( null );
					setDone( false );
					setError(
						`cursor invalidated: ${ response.cursorState.reason }`
					);
				} else {
					setCursor( response.nextCursor );
					setDone( response.nextCursor === null );
				}
			} catch ( err: unknown ) {
				if ( seq !== requestSeqRef.current ) return;
				const message =
					err instanceof Error ? err.message : String( err );
				setError( message );
			} finally {
				if ( seq === requestSeqRef.current ) {
					setLoading( false );
				}
				isFetchingRef.current = false;
			}
		},
		[ abilityName, payload ]
	);

	// Initial load whenever the request key changes.
	useEffect( () => {
		void fetchPage( null );
		// We deliberately key on payloadKey + scopeKey + watermark; fetchPage
		// closes over the latest payload via the dep array.
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [ payloadKey, scopeKey, watermark ] );

	const loadMore = useCallback( () => {
		if ( loading || done || cursor === null ) return;
		void fetchPage( cursor );
	}, [ loading, done, cursor, fetchPage ] );

	const sentinelRef = useCallback(
		( node: Element | null ) => {
			if ( ! autoLoad ) return;
			if ( typeof IntersectionObserver === "undefined" ) return;

			// Disconnect any prior observer.
			if ( sentinelObserverRef.current ) {
				sentinelObserverRef.current.disconnect();
				sentinelObserverRef.current = null;
			}

			if ( node === null ) return;

			const observer = new IntersectionObserver( ( entries ) => {
				for ( const entry of entries ) {
					if ( entry.isIntersecting ) {
						loadMore();
					}
				}
			} );
			observer.observe( node );
			sentinelObserverRef.current = observer;
		},
		[ autoLoad, loadMore ]
	);

	// Clean up the observer on unmount.
	useEffect( () => {
		return () => {
			if ( sentinelObserverRef.current ) {
				sentinelObserverRef.current.disconnect();
				sentinelObserverRef.current = null;
			}
		};
	}, [] );

	const advisory =
		cursorState.kind === "data_shifted" ? cursorState.advisory : null;

	return {
		items,
		loading,
		loadMore,
		reset,
		done,
		advisory,
		cursorState,
		error,
		sentinelRef,
	};
}
