/**
 * Account Overview block editor preview.
 *
 * - Renders the same projected composition shape as front-end render.php
 *   by proxying preview through a server route (no direct runtime call
 *   from the browser; AC §15).
 * - Exposes an InspectorControls `ComboboxControl` for account selection
 *   per V3 §6.9.
 * - Exposes a "Reload from runtime" button that re-invokes the preview
 *   endpoint and updates `composition_id`, `composition_version`,
 *   `watermarks`, and optionally `cache_hint_token` on success.
 *
 * Browser-side never reaches into request-signing material, nonce
 * material, or pairing secrets (AC §55). All authentication is
 * server-side via the WordPress REST nonce and the DailyOS runtime
 * client.
 */

( function ( wp ) {
	const { __ } = wp.i18n;
	const { registerBlockType } = wp.blocks;
	const { InspectorControls, useBlockProps } = wp.blockEditor;
	// W4-F L4-unblock backport: swap ComboboxControl → TextControl. The combobox
	// requires a populated `options` list (account discovery endpoint is
	// stubbed at [] until path-α implements list-discovery), and rejects
	// arbitrary input. TextControl lets the user type an account_id directly,
	// which is sufficient for the L4 render proof. Re-promote to combobox
	// once the discovery endpoint is wired.
	const { PanelBody, TextControl, Button, Spinner, Notice } = wp.components;
	const { useState, useEffect, useCallback } = wp.element;
	const apiFetch = wp.apiFetch;

	const BLOCK_NAME = 'dailyos/account-overview';

	function AccountOverviewEdit( props ) {
		const { attributes, setAttributes } = props;
		const blockProps = useBlockProps();
		const [ preview, setPreview ] = useState( null );
		const [ isLoading, setIsLoading ] = useState( false );
		const [ error, setError ] = useState( null );
		const [ accounts, setAccounts ] = useState( [] );

		const reload = useCallback( () => {
			if ( ! attributes.composition_id ) {
				setPreview( null );
				return;
			}
			setIsLoading( true );
			setError( null );
			apiFetch( {
				path: '/dailyos/v1/account-overview/preview',
				method: 'POST',
				data: {
					composition_id: attributes.composition_id,
					composition_version: attributes.composition_version || 0,
					cache_hint_token: attributes.cache_hint_token || '',
				},
			} )
				.then( ( response ) => {
					setPreview( response );
					if ( response && response.projection ) {
						setAttributes( {
							composition_version:
								response.projection.composition_version ||
								attributes.composition_version,
							watermarks: response.projection.watermarks || {},
							cache_hint_token: response.cache_hint_token || '',
						} );
					}
				} )
				.catch( ( e ) => {
					setError(
						__(
							"Something about this account doesn't line up. Verify before acting.",
							'dailyos'
						)
					);
					// Per AC §44: failed reload must leave attributes
					// unchanged. We only setAttributes on success.
				} )
				.finally( () => setIsLoading( false ) );
		}, [ attributes.composition_id, attributes.composition_version, attributes.cache_hint_token, setAttributes ] );

		useEffect( () => {
			reload();
		}, [ reload ] );

		useEffect( () => {
			apiFetch( { path: '/dailyos/v1/account-overview/accounts' } )
				.then( ( list ) => {
					if ( Array.isArray( list ) ) {
						setAccounts(
							list.map( ( a ) => ( {
								value: a.id,
								label: a.name || a.id,
							} ) )
						);
					}
				} )
				.catch( () => {} );
		}, [] );

		const onAccountChange = ( accountId ) => {
			if ( ! accountId ) {
				return;
			}
			setAttributes( {
				account_id: accountId,
				composition_id:
					'dailyos/account-overview:account:' + accountId,
				composition_version: 0,
				watermarks: {},
				cache_hint_token: '',
			} );
		};

		return wp.element.createElement(
			wp.element.Fragment,
			null,
			wp.element.createElement(
				InspectorControls,
				null,
				wp.element.createElement(
					PanelBody,
					{ title: __( 'Account', 'dailyos' ) },
					wp.element.createElement( TextControl, {
						label: __( 'Account ID', 'dailyos' ),
						value: attributes.account_id || '',
						onChange: onAccountChange,
						help: __( 'Enter the account_id (e.g. acme-corp).', 'dailyos' ),
					} ),
					wp.element.createElement(
						Button,
						{
							variant: 'secondary',
							onClick: reload,
							disabled: isLoading || ! attributes.composition_id,
						},
						__( 'Reload from runtime', 'dailyos' )
					)
				)
			),
			wp.element.createElement(
				'section',
				blockProps,
				isLoading && wp.element.createElement( Spinner, null ),
				error &&
					wp.element.createElement(
						Notice,
						{ status: 'warning', isDismissible: false },
						error
					),
				preview &&
					preview.projection &&
					wp.element.createElement( 'div', {
						dangerouslySetInnerHTML: { __html: preview.html || '' },
					} )
			)
		);
	}

	registerBlockType( BLOCK_NAME, {
		edit: AccountOverviewEdit,
		save: () => null,
	} );
} )( window.wp );
