/**
 * Project Detail block editor UX (W2 L4 wiring).
 *
 * Provides the editor-side `edit` implementation for the server-registered
 * `dailyos/project-detail` block. The InspectorControls panel lets editors
 * type a `project_id` directly — when left blank the front-end renderer
 * falls back to `dailyos_entity_id` post-meta, then the post slug, when
 * rendering inside a `dailyos_project` post (L4 quick-setup path).
 */

( function ( wp ) {
	const { __ } = wp.i18n;
	const { registerBlockType } = wp.blocks;
	const { InspectorControls, useBlockProps } = wp.blockEditor;
	const { PanelBody, TextControl } = wp.components;

	const BLOCK_NAME = 'dailyos/project-detail';

	function ProjectDetailEdit( props ) {
		const { attributes, setAttributes } = props;
		const blockProps = useBlockProps();
		const projectId = attributes.project_id || '';

		return wp.element.createElement(
			wp.element.Fragment,
			null,
			wp.element.createElement(
				InspectorControls,
				null,
				wp.element.createElement(
					PanelBody,
					{ title: __( 'Project', 'dailyos' ), initialOpen: true },
					wp.element.createElement( TextControl, {
						label: __( 'Project ID', 'dailyos' ),
						value: projectId,
						onChange: ( value ) =>
							setAttributes( { project_id: value } ),
						help: __(
							'Leave blank to auto-fill from post meta or slug when rendering inside a dailyos_project post.',
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
					projectId
						? __( 'Rendering project: ', 'dailyos' ) + projectId
						: __(
							'Project ID will auto-fill from post context (slug or meta) at render time.',
							'dailyos'
						)
				),
				wp.element.createElement(
					'p',
					null,
					__(
						'Inner blocks render the full project composition at front-end (envelope fetched via runtime).',
						'dailyos'
					)
				)
			)
		);
	}

	registerBlockType( BLOCK_NAME, {
		edit: ProjectDetailEdit,
		save: () => null,
	} );
} )( window.wp );
