/**
 * Account Detail block editor UX (W2 L4 wiring).
 *
 * Provides the editor-side `edit` implementation for the server-registered
 * `dailyos/account-detail` block. The InspectorControls panel lets editors
 * type an `account_id` directly, and the canvas exposes the chapter blocks as
 * a real InnerBlocks slot so they can be moved, removed, and inserted.
 *
 * Browser-side never reaches into runtime credentials; the actual entity
 * envelope fetch happens server-side at render time via the paired
 * runtime client (AC §15 / §55).
 */

( function ( wp ) {
	const { __ } = wp.i18n;
	const { registerBlockType } = wp.blocks;
	const { InnerBlocks, InspectorControls, useBlockProps } = wp.blockEditor;
	const { PanelBody, TextControl } = wp.components;

	const BLOCK_NAME = 'dailyos/account-detail';
	const TEMPLATE = [
		[ 'dailyos/account-hero' ],
		[ 'dailyos/sentiment-hero' ],
		[ 'dailyos/triage-section' ],
		[ 'dailyos/outlook-panel' ],
		[ 'dailyos/supporting-tension' ],
		[ 'dailyos/about-intelligence' ],
		[ 'dailyos/account-pull-quote' ],
		[ 'dailyos/stakeholder-grid' ],
		[ 'dailyos/strategic-landscape' ],
		[ 'dailyos/value-commitments' ],
		[ 'dailyos/quote-wall' ],
		[ 'dailyos/commercial-shape' ],
		[ 'dailyos/account-technical-footprint' ],
		[ 'dailyos/relationship-fabric' ],
		[ 'dailyos/about-this-dossier' ],
		[ 'dailyos/account-detail-unified-timeline' ],
		[ 'dailyos/account-detail-reports' ],
		[ 'dailyos/the-record' ],
		[ 'dailyos/file-list' ],
		[ 'dailyos/linear-issues-chapter' ],
	];
	const ALLOWED_BLOCKS = TEMPLATE.map( ( entry ) => entry[ 0 ] );

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
				{
					...blockProps,
					'data-dailyos-editor-surface': 'account-detail',
				},
				wp.element.createElement(
					'div',
					{ className: 'dailyos-account-detail-editor__header' },
					wp.element.createElement(
						'strong',
						null,
						accountId
							? __( 'Account detail: ', 'dailyos' ) + accountId
							: __( 'Account detail', 'dailyos' )
					),
					wp.element.createElement(
						'span',
						null,
						__(
							'Chapters render from DailyOS intelligence on the front end.',
							'dailyos'
						)
					)
				),
				wp.element.createElement( InnerBlocks, {
					template: TEMPLATE,
					allowedBlocks: ALLOWED_BLOCKS,
					templateLock: false,
				} )
			)
		);
	}

	registerBlockType( BLOCK_NAME, {
		edit: AccountDetailEdit,
		save: () => null,
	} );
} )( window.wp );
