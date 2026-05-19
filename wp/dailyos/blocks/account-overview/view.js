/**
 * Account Overview feedback affordance runtime.
 *
 * This mirrors wp/dailyos/src/components/FeedbackAffordance so the
 * server-rendered dynamic block can mount React controls without a separate
 * block build pipeline.
 */
( function ( wp ) {
	if ( ! wp || ! wp.element ) {
		return;
	}

	const { createElement: h, useMemo, useState } = wp.element;
	const apiFetch = wp.apiFetch;

	const ACTIONS = [
		{
			kind: 'confirm_current',
			label: 'Still current',
			description: 'Confirm this claim is accurate now.',
		},
		{
			kind: 'mark_outdated',
			label: 'Outdated',
			description: 'It used to be true, but no longer is.',
		},
		{
			kind: 'mark_false',
			label: 'False',
			description: 'Mark this claim as wrong.',
		},
		{
			kind: 'wrong_subject',
			label: 'Wrong subject',
			description: 'The fact belongs somewhere else.',
		},
		{
			kind: 'wrong_source',
			label: 'Wrong source',
			description: 'The cited source does not support it.',
		},
		{
			kind: 'cannot_verify',
			label: 'Cannot verify',
			description: 'Ask DailyOS to corroborate it.',
		},
		{
			kind: 'needs_nuance',
			label: 'Needs nuance',
			description: 'Provide a more precise version.',
		},
		{
			kind: 'surface_inappropriate',
			label: 'Wrong surface',
			description: 'Hide it from this surface only.',
		},
		{
			kind: 'not_relevant_here',
			label: 'Not relevant here',
			description: 'Deprioritize it in this context.',
		},
	];

	const SURFACES = [
		'account_overview',
		'entity_detail',
		'briefing',
		'meeting_prep',
		'project_detail',
	];

	const cls = {
		feedbackAffordance: 'feedbackAffordance',
		trigger: 'trigger',
		menu: 'menu',
		menuItem: 'menuItem',
		menuItemTitle: 'menuItemTitle',
		menuItemDescription: 'menuItemDescription',
		panel: 'panel',
		panelHeader: 'panelHeader',
		panelTitle: 'panelTitle',
		panelCopy: 'panelCopy',
		field: 'field',
		label: 'label',
		textarea: 'textarea',
		select: 'select',
		meta: 'meta',
		status: 'status',
		actions: 'actions',
		button: 'button',
		confirm: 'confirm',
		dismiss: 'dismiss',
		success: 'success',
		error: 'error',
	};

	function sourceLabel( source, index ) {
		if ( source && typeof source.label === 'string' && source.label.trim() ) {
			return source.label;
		}
		if ( source && typeof source.source_ref === 'string' && source.source_ref.trim() ) {
			return source.source_ref;
		}
		if ( source && typeof source.ref === 'string' && source.ref.trim() ) {
			return source.ref;
		}
		if ( source && typeof source.id === 'string' && source.id.trim() ) {
			return source.id;
		}
		return 'Source ' + ( index + 1 );
	}

	function sourceRef( source, index ) {
		const keys = [ 'source_ref', 'ref', 'id', 'invocation_id' ];
		for ( const key of keys ) {
			if ( source && typeof source[ key ] === 'string' && source[ key ].trim() ) {
				return source[ key ];
			}
		}
		return String( index );
	}

	function messageFromError( error ) {
		if ( error && typeof error === 'object' ) {
			for ( const key of [ 'rejection_reason', 'reason', 'message', 'error' ] ) {
				if ( typeof error[ key ] === 'string' && error[ key ].trim() ) {
					return error[ key ];
				}
			}
			if ( error.data && typeof error.data === 'object' ) {
				for ( const key of [ 'rejection_reason', 'reason', 'message' ] ) {
					if ( typeof error.data[ key ] === 'string' && error.data[ key ].trim() ) {
						return error.data[ key ];
					}
				}
			}
		}
		return 'Feedback was rejected. Dismiss this message and try again.';
	}

	function FeedbackAffordance( props ) {
		const claimId = props.claimId || '';
		const sources = Array.isArray( props.sources ) ? props.sources : [];
		const currentSurface = props.currentSurface || 'account_overview';
		const currentInvocationId = props.currentInvocationId || '';
		const [ state, setState ] = useState( 'idle' );
		const [ selectedAction, setSelectedAction ] = useState( null );
		const [ note, setNote ] = useState( '' );
		const [ sourceIndex, setSourceIndex ] = useState( '0' );
		const [ surface, setSurface ] = useState( currentSurface );
		const [ invocationId, setInvocationId ] = useState( currentInvocationId );
		const [ error, setError ] = useState( '' );

		const invocationOptions = useMemo( () => {
			const values = new Set();
			if ( currentInvocationId ) {
				values.add( currentInvocationId );
			}
			for ( const source of sources ) {
				if ( source && typeof source.invocation_id === 'string' && source.invocation_id.trim() ) {
					values.add( source.invocation_id );
				}
			}
			return Array.from( values );
		}, [ currentInvocationId, sources ] );

		const close = () => {
			setState( 'idle' );
			setSelectedAction( null );
			setNote( '' );
			setError( '' );
		};

		const openForm = ( action ) => {
			setSelectedAction( action );
			setNote( '' );
			setSourceIndex( '0' );
			setSurface( currentSurface );
			setInvocationId( currentInvocationId || invocationOptions[ 0 ] || '' );
			setError( '' );
			setState( 'form-open' );
		};

		const buildPayload = () => {
			if ( ! selectedAction ) {
				return undefined;
			}
			const trimmedNote = note.trim();
			switch ( selectedAction.kind ) {
				case 'needs_nuance':
					return { corrected_text: trimmedNote };
				case 'wrong_subject':
					return trimmedNote ? { reason: trimmedNote } : undefined;
				case 'wrong_source': {
					const index = Number.parseInt( sourceIndex, 10 );
					const safeIndex = Number.isNaN( index ) ? 0 : index;
					return {
						source_index: safeIndex,
						source_ref: sourceRef( sources[ safeIndex ], safeIndex ),
						...( trimmedNote ? { reason: trimmedNote } : {} ),
					};
				}
				case 'surface_inappropriate':
					return { surface };
				case 'not_relevant_here':
					return { invocation_id: invocationId };
				default:
					return undefined;
			}
		};

		const confirm = async () => {
			if ( ! selectedAction || ! claimId ) {
				return;
			}
			if ( selectedAction.kind === 'needs_nuance' && ! note.trim() ) {
				setError( 'What needs nuance? is required.' );
				setState( 'error' );
				return;
			}
			if ( selectedAction.kind === 'not_relevant_here' && ! invocationId.trim() ) {
				setError( 'Choose an invocation before confirming.' );
				setState( 'error' );
				return;
			}
			if ( ! apiFetch ) {
				setError( 'WordPress apiFetch is unavailable.' );
				setState( 'error' );
				return;
			}

			setState( 'loading' );
			setError( '' );

			try {
				const payload = buildPayload();
				const nonceResponse = await apiFetch( {
					path: '/dailyos/v1/nonce',
					method: 'POST',
					data: {
						claim_id: claimId,
						action_kind: selectedAction.kind,
						...( payload ? { payload_json: JSON.stringify( payload ) } : {} ),
					},
				} );
				const nonceDigest =
					nonceResponse &&
					( nonceResponse.nonce_digest ||
						nonceResponse.presence_nonce ||
						nonceResponse.nonce );

				if ( typeof nonceDigest !== 'string' || ! nonceDigest.trim() ) {
					throw new Error( 'Runtime did not return a nonce digest.' );
				}

				const verifyResponse = await apiFetch( {
					path: '/dailyos/v1/nonce/verify',
					method: 'POST',
					data: { nonce_digest: nonceDigest },
				} );

				if ( verifyResponse && verifyResponse.ok === false ) {
					throw verifyResponse;
				}

				setState( 'success' );
				if ( typeof props.onFeedbackRecorded === 'function' ) {
					props.onFeedbackRecorded( verifyResponse || {} );
				}
			} catch ( caught ) {
				setError( messageFromError( caught ) );
				setState( 'error' );
			}
		};

		const confirmDisabled =
			state === 'loading' ||
			( selectedAction &&
				selectedAction.kind === 'needs_nuance' &&
				! note.trim() ) ||
			( selectedAction &&
				selectedAction.kind === 'not_relevant_here' &&
				! invocationId.trim() );

		return h(
			'span',
			{ className: cls.feedbackAffordance, 'data-state': state },
			h(
				'button',
				{
					type: 'button',
					className: cls.trigger,
					onClick: () => setState( state === 'menu-open' ? 'idle' : 'menu-open' ),
					disabled: ! claimId || state === 'loading',
					'aria-expanded': state === 'menu-open',
				},
				'Feedback'
			),
			state === 'menu-open'
				? h(
						'div',
						{ className: cls.menu, role: 'menu' },
						ACTIONS.map( ( action ) =>
							h(
								'button',
								{
									key: action.kind,
									type: 'button',
									className: cls.menuItem,
									onClick: () => openForm( action ),
									role: 'menuitem',
								},
								h( 'span', { className: cls.menuItemTitle }, action.label ),
								h(
									'span',
									{ className: cls.menuItemDescription },
									action.description
								)
							)
						)
				  )
				: null,
			state === 'form-open' || state === 'loading'
				? h(
						'div',
						{ className: cls.panel, role: 'dialog', 'aria-label': 'Feedback' },
						h(
							'div',
							{ className: cls.panelHeader },
							h( 'p', { className: cls.panelTitle }, selectedAction && selectedAction.label ),
							h(
								'p',
								{ className: cls.panelCopy },
								selectedAction && selectedAction.description
							)
						),
						selectedAction && selectedAction.kind === 'needs_nuance'
							? textareaField( 'What needs nuance?', note, setNote, true )
							: null,
						selectedAction &&
							( selectedAction.kind === 'wrong_subject' ||
								selectedAction.kind === 'wrong_source' )
							? textareaField( 'Optional note', note, setNote, false )
							: null,
						selectedAction && selectedAction.kind === 'wrong_source'
							? h(
									'label',
									{ className: cls.field },
									h( 'span', { className: cls.label }, 'Source' ),
									h(
										'select',
										{
											className: cls.select,
											value: sourceIndex,
											onChange: ( event ) => setSourceIndex( event.currentTarget.value ),
										},
										sources.length
											? sources.map( ( source, index ) =>
													h(
														'option',
														{
															key: sourceLabel( source, index ) + '-' + index,
															value: String( index ),
														},
														sourceLabel( source, index )
													)
											  )
											: h( 'option', { value: '0' }, 'Source 1' )
									)
							  )
							: null,
						selectedAction && selectedAction.kind === 'surface_inappropriate'
							? pickerField(
									'Surface',
									surface,
									( event ) => setSurface( event.currentTarget.value ),
									Array.from( new Set( [ currentSurface, ...SURFACES ].filter( Boolean ) ) )
							  )
							: null,
						selectedAction && selectedAction.kind === 'not_relevant_here'
							? pickerField(
									'Invocation',
									invocationId,
									( event ) => setInvocationId( event.currentTarget.value ),
									invocationOptions.length ? invocationOptions : [ '' ],
									invocationOptions.length ? null : 'No invocation detected'
							  )
							: null,
						h( 'p', { className: cls.status }, state === 'loading' ? 'Recording feedback...' : '' ),
						h(
							'div',
							{ className: cls.actions },
							h(
								'button',
								{ type: 'button', className: cls.button, onClick: close },
								'Cancel'
							),
							h(
								'button',
								{
									type: 'button',
									className: cls.button + ' ' + cls.confirm,
									onClick: confirm,
									disabled: Boolean( confirmDisabled ),
								},
								'Confirm'
							)
						)
				  )
				: null,
			state === 'success'
				? statusPanel( cls.success, 'Feedback recorded.', close )
				: null,
			state === 'error' ? statusPanel( cls.error, error, close ) : null
		);
	}

	function textareaField( label, value, setValue, required ) {
		return h(
			'label',
			{ className: cls.field },
			h( 'span', { className: cls.label }, label ),
			h( 'textarea', {
				className: cls.textarea,
				value,
				onChange: ( event ) => setValue( event.currentTarget.value.slice( 0, 500 ) ),
				maxLength: 500,
				required,
			} ),
			h( 'span', { className: cls.meta }, value.length + '/500' )
		);
	}

	function pickerField( label, value, onChange, values, fallbackLabel ) {
		return h(
			'label',
			{ className: cls.field },
			h( 'span', { className: cls.label }, label ),
			h(
				'select',
				{ className: cls.select, value, onChange },
				values.map( ( option ) =>
					h( 'option', { key: option || fallbackLabel, value: option }, option || fallbackLabel )
				)
			)
		);
	}

	function statusPanel( className, message, close ) {
		return h(
			'div',
			{ className: cls.panel, role: className === cls.error ? 'alert' : 'status' },
			h( 'p', { className: cls.status + ' ' + className }, message ),
			h(
				'div',
				{ className: cls.actions },
				h( 'button', { type: 'button', className: cls.dismiss, onClick: close }, 'Dismiss' )
			)
		);
	}

	function readJsonAttribute( node, name, fallback ) {
		const raw = node.getAttribute( name );
		if ( ! raw ) {
			return fallback;
		}
		try {
			return JSON.parse( raw );
		} catch ( error ) {
			return fallback;
		}
	}

	function mountFeedbackAffordances( root, options ) {
		const scope = root || document;
		const slots = scope.querySelectorAll( '[data-dailyos-feedback-affordance]' );
		slots.forEach( ( slot ) => {
			if ( slot.dataset.dailyosFeedbackMounted === '1' ) {
				return;
			}
			const encodedProps = readJsonAttribute( slot, 'data-dailyos-feedback-props', null ) || {};
			const props = {
				claimId: encodedProps.claimId || slot.getAttribute( 'data-claim-id' ) || '',
				sources: Array.isArray( encodedProps.sources )
					? encodedProps.sources
					: readJsonAttribute( slot, 'data-sources', [] ),
				currentSurface:
					encodedProps.currentSurface ||
					encodedProps.surface ||
					slot.getAttribute( 'data-current-surface' ) ||
					'account_overview',
				currentInvocationId:
					encodedProps.currentInvocationId ||
					encodedProps.invocationId ||
					slot.getAttribute( 'data-current-invocation-id' ) ||
					'',
				onFeedbackRecorded: ( result ) => {
					slot.dispatchEvent(
						new CustomEvent( 'dailyos:feedback-recorded', {
							bubbles: true,
							detail: result,
						} )
					);
					if ( options && typeof options.onFeedbackRecorded === 'function' ) {
						options.onFeedbackRecorded( result );
					}
				},
			};
			slot.dataset.dailyosFeedbackMounted = '1';
			if ( wp.element.createRoot ) {
				wp.element.createRoot( slot ).render( h( FeedbackAffordance, props ) );
			} else if ( wp.element.render ) {
				wp.element.render( h( FeedbackAffordance, props ), slot );
			}
		} );
	}

	function hydrateAccountOverviewFeedback() {
		mountFeedbackAffordances( document );
	}

	window.DailyOSFeedbackAffordance = {
		FeedbackAffordance,
		mountFeedbackAffordances,
		hydrateAccountOverviewFeedback,
	};

	if ( document.readyState === 'loading' ) {
		document.addEventListener( 'DOMContentLoaded', hydrateAccountOverviewFeedback );
	} else {
		hydrateAccountOverviewFeedback();
	}
} )( window.wp );
