<?php
/**
 * Block editor script asset manifest.
 *
 * Modern WordPress block editor expects a sibling `*.asset.php` manifest
 * for each editor script declaring its `dependencies` and `version`.
 * Without it, `wp.components`, `wp.blockEditor`, `wp.element`, `wp.i18n`,
 * and `wp.apiFetch` may not be loaded before `edit.js` runs, causing
 * silent block-render failures in the editor.
 *
 * The dotted filename (`edit.asset.php`) is the WordPress block-script
 * convention — `@wordpress/scripts` auto-generates this shape and core
 * looks for it explicitly. Filename rule is excluded for `blocks/*/*.asset.php`
 * in `phpcs.xml.dist`.
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
		'wp-api-fetch',
	],
	'version'      => '1.0.0',
];
