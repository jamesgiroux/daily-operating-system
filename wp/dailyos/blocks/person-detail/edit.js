/**
 * Person Detail block editor UX (W2 L4 wiring).
 *
 * Provides the editor-side `edit` implementation for the server-registered
 * `dailyos/person-detail` block. The InspectorControls panel lets editors
 * type a `person_id` directly — when left blank the front-end renderer
 * falls back to `dailyos_entity_id` post-meta, then the post slug, when
 * rendering inside a `dailyos_person` post (L4 quick-setup path).
 */

( function ( wp ) {
	const { __ } = wp.i18n;
	const { registerBlockType } = wp.blocks;
	const { InspectorControls, useBlockProps } = wp.blockEditor;
	const { PanelBody, TextControl } = wp.components;

	const BLOCK_NAME = 'dailyos/person-detail';

	function PersonDetailEdit( props ) {
		const { attributes, setAttributes } = props;
		const blockProps = useBlockProps();
		const personId = attributes.person_id || '';

		return wp.element.createElement(
			wp.element.Fragment,
			null,
			wp.element.createElement(
				InspectorControls,
				null,
				wp.element.createElement(
					PanelBody,
					{ title: __( 'Person', 'dailyos' ), initialOpen: true },
					wp.element.createElement( TextControl, {
						label: __( 'Person ID', 'dailyos' ),
						value: personId,
						onChange: ( value ) =>
							setAttributes( { person_id: value } ),
						help: __(
							'Leave blank to auto-fill from post meta or slug when rendering inside a dailyos_person post.',
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
					personId
						? __( 'Rendering person: ', 'dailyos' ) + personId
						: __(
							'Person ID will auto-fill from post context (slug or meta) at render time.',
							'dailyos'
						)
				),
				wp.element.createElement(
					'p',
					null,
					__(
						'Inner blocks render the full person composition at front-end (envelope fetched via runtime).',
						'dailyos'
					)
				)
			)
		);
	}

	registerBlockType( BLOCK_NAME, {
		edit: PersonDetailEdit,
		save: () => null,
	} );
} )( window.wp );
