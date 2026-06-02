/**
 * Account Detail block editor UX (W2 L4 wiring).
 *
 * Provides the editor-side `edit` implementation for the server-registered
 * `dailyos/account-detail` block. The InspectorControls panel lets editors
 * type an `account_id` directly — when left blank the front-end renderer
 * falls back to `dailyos_entity_id` post-meta, then the post slug, when
 * rendering inside a `dailyos_account` post (L4 quick-setup path).
 *
 * Browser-side never reaches into runtime credentials; the actual entity
 * envelope fetch happens server-side at render time via the paired
 * runtime client (AC §15 / §55).
 */

( function ( wp ) {
	const { __ } = wp.i18n;
	const { registerBlockType } = wp.blocks;
	const { InspectorControls, useBlockProps } = wp.blockEditor;
	const { PanelBody, TextControl } = wp.components;

	const BLOCK_NAME = 'dailyos/account-detail';

	function AccountDetailEdit( props ) {
		const { attributes, setAttributes } = props;
		const blockProps = useBlockProps();
		const accountId = attributes.account_id || '';

		return wp.element.createElement(
			wp.element.Fragment,
			null,
			wp.element.createElement(
				InspectorControls,
				null,
				wp.element.createElement(
					PanelBody,
					{ title: __( 'Account', 'dailyos' ), initialOpen: true },
					wp.element.createElement( TextControl, {
						label: __( 'Account ID', 'dailyos' ),
						value: accountId,
						onChange: ( value ) =>
							setAttributes( { account_id: value } ),
						help: __(
							'Leave blank to auto-fill from post meta or slug when rendering inside a dailyos_account post.',
							'dailyos'
						),
					} )
				)
			),
			wp.element.createElement(
				'section',
				blockProps,
				wp.element.createElement(
					'p',
					null,
					accountId
						? __( 'Rendering account: ', 'dailyos' ) + accountId
						: __(
							'Account ID will auto-fill from post context (slug or meta) at render time.',
							'dailyos'
						)
				),
				wp.element.createElement(
					'p',
					null,
					__(
						'Inner blocks render the full account composition at front-end (envelope fetched via runtime).',
						'dailyos'
					)
				)
			)
		);
	}

	registerBlockType( BLOCK_NAME, {
		edit: AccountDetailEdit,
		save: () => null,
	} );
} )( window.wp );
