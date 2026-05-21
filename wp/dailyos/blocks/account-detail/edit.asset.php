<?php
/**
 * Block editor script asset manifest for dailyos/account-detail (W2 L4 wiring).
 *
 * Declares the wp-* dependencies edit.js consumes: blocks (registerBlockType),
 * block-editor (InspectorControls, useBlockProps), components (PanelBody,
 * TextControl), element (createElement, Fragment), i18n (__).
 *
 * Filename convention matches account-overview/edit.asset.php — phpcs.xml.dist
 * exempts asset-manifest filenames from the dot-rule lint.
 *
 * @package dailyos
 */

return [
	'dependencies' => [
		'wp-blocks',
		'wp-block-editor',
		'wp-components',
		'wp-element',
		'wp-i18n',
	],
	'version'      => '1.0.0',
];
