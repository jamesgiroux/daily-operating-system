( function () {
	const { registerBlockType } = wp.blocks;
	const { useBlockProps } = wp.blockEditor;
	const { __ } = wp.i18n;

	const BLOCK_NAME = 'dailyos/source-management';

	function SourceManagementEdit( props ) {
		const { attributes, setAttributes } = props;
		const blockProps = useBlockProps( {
			className: 'dailyos-source-management-editor',
		} );

		return wp.element.createElement(
			'div',
			blockProps,
			dailyosSourceManagementField(
				__( 'Entity type', 'dailyos' ),
				'text',
				attributes.entity_type || '',
				( value ) => setAttributes( { entity_type: value } )
			),
			dailyosSourceManagementField(
				__( 'Entity ID', 'dailyos' ),
				'text',
				attributes.entity_id || '',
				( value ) => setAttributes( { entity_id: value } )
			),
			dailyosSourceManagementField(
				__( 'Page size', 'dailyos' ),
				'number',
				attributes.page_size || 25,
				( value ) => setAttributes( { page_size: parseInt( value, 10 ) || 25 } )
			)
		);
	}

	function dailyosSourceManagementField( label, type, value, onChange ) {
		return wp.element.createElement(
			'label',
			{ className: 'dailyos-source-management-editor__field' },
			label,
			wp.element.createElement( 'input', {
				type,
				value,
				min: 'number' === type ? 1 : undefined,
				max: 'number' === type ? 100 : undefined,
				onChange: ( event ) => onChange( event.target.value ),
			} )
		);
	}

	registerBlockType( BLOCK_NAME, {
		edit: SourceManagementEdit,
		save: () => null,
	} );
} )();
