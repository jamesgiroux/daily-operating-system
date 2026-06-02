( function () {
	const { registerBlockType } = wp.blocks;
	const { useBlockProps } = wp.blockEditor;
	const { __ } = wp.i18n;

	const BLOCK_NAME = 'dailyos/markdown-preview';

	function MarkdownPreviewEdit( props ) {
		const { attributes, setAttributes } = props;
		const blockProps = useBlockProps( {
			className: 'dailyos-markdown-preview-editor',
		} );

		return wp.element.createElement(
			'div',
			blockProps,
			wp.element.createElement(
				'label',
				{ className: 'dailyos-markdown-preview-editor__field' },
				__( 'Source handle', 'dailyos' ),
				wp.element.createElement( 'input', {
					type: 'text',
					value: attributes.source_handle || '',
					onChange: ( event ) =>
						setAttributes( { source_handle: event.target.value } ),
				} )
			),
			wp.element.createElement(
				'p',
				{
					className: 'dailyos-markdown-preview-editor__notice',
					role: 'status',
				},
				__( 'Preview is temporarily unavailable.', 'dailyos' )
			)
		);
	}

	registerBlockType( BLOCK_NAME, {
		edit: MarkdownPreviewEdit,
		save: () => null,
	} );
} )();
