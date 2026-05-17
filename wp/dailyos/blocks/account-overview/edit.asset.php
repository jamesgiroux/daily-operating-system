<?php
/**
 * W4-F L4-unblock backport from wave3-l2-integration.
 *
 * Modern WordPress block editor expects a sibling `*.asset.php` manifest
 * for each editor script declaring its `dependencies` and `version`.
 * Without it, `wp.components`, `wp.blockEditor`, `wp.element`, `wp.i18n`,
 * and `wp.apiFetch` may not be loaded before `edit.js` runs, causing
 * silent block-render failures in the editor.
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
