( function () {
	const { registerBlockType } = wp.blocks;
	const { useBlockProps } = wp.blockEditor;
	const { __ } = wp.i18n;

	const BLOCK_NAME = "dailyos/the-record";
	const BLOCK_TITLE = "The Record";

	function AccountDetailInnerEdit() {
		const blockProps = useBlockProps( {
			className: 'dailyos-account-detail-inner-editor-placeholder',
		} );

		return wp.element.createElement(
			'div',
			blockProps,
			wp.element.createElement( 'strong', null, BLOCK_TITLE ),
			wp.element.createElement(
				'p',
				null,
				__( 'Dynamic account detail block.', 'dailyos' )
			)
		);
	}

	registerBlockType( BLOCK_NAME, {
		apiVersion: 3,
		title: BLOCK_TITLE,
		category: 'dailyos',
		supports: {
			html: false,
			reusable: false,
			inserter: true,
		},
		usesContext: [
			'dailyos/entityType',
			'dailyos/entityId',
			'dailyos/envelopeHandle',
		],
		edit: AccountDetailInnerEdit,
		save: () => null,
	} );
} )();
