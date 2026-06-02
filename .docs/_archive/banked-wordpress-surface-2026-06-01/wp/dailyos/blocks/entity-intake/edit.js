( function () {
	const { registerBlockType } = wp.blocks;
	const { useBlockProps } = wp.blockEditor;
	const { useState } = wp.element;
	const { __ } = wp.i18n;

	const BLOCK_NAME = 'dailyos/entity-intake';

	function EntityIntakeEdit( props ) {
		const { attributes, setAttributes } = props;
		const blockProps = useBlockProps( {
			className: 'dailyos-entity-intake-editor',
		} );
		const [ status, setStatus ] = useState( 'idle' );
		const routeAvailable = false;

		const ingest = () => {
			setStatus( 'deferred' );
		};

		return wp.element.createElement(
			'div',
			blockProps,
			wp.element.createElement(
				'div',
				{ className: 'dailyos-entity-intake-editor__fields' },
				wp.element.createElement( 'label', {}, __( 'Entity type', 'dailyos' ),
					wp.element.createElement( 'input', {
						type: 'text',
						value: attributes.entity_type || '',
						onChange: ( event ) => setAttributes( { entity_type: event.target.value } ),
					} )
				),
				wp.element.createElement( 'label', {}, __( 'Entity ID', 'dailyos' ),
					wp.element.createElement( 'input', {
						type: 'text',
						value: attributes.entity_id || '',
						onChange: ( event ) => setAttributes( { entity_id: event.target.value } ),
					} )
				),
				wp.element.createElement( 'label', {}, __( 'File ref', 'dailyos' ),
					wp.element.createElement( 'input', {
						type: 'text',
						value: attributes.file_ref || '',
						onChange: ( event ) => setAttributes( { file_ref: event.target.value } ),
					} )
				)
			),
			wp.element.createElement(
				'button',
				{
					type: 'button',
					className: 'dailyos-entity-intake-editor__button',
					onClick: ingest,
					disabled: ! routeAvailable,
				},
				__( 'Ingest', 'dailyos' )
			),
			status === 'deferred' &&
				wp.element.createElement(
					'p',
					{ className: 'dailyos-entity-intake-editor__notice', role: 'status' },
					__( 'Editor intake transport is deferred to the next transport extension.', 'dailyos' )
				)
		);
	}

	registerBlockType( BLOCK_NAME, {
		edit: EntityIntakeEdit,
		save: () => null,
	} );
} )();
