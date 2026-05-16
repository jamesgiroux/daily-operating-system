<?php
/**
 * Asset manifest for blocks/account-overview/edit.js.
 *
 * WordPress's `register_block_type_from_metadata()` reads this file alongside
 * `edit.js` to discover script dependencies. Without it, the script enqueues
 * with an empty dependency list and may run before `wp.blocks` /
 * `wp.blockEditor` / `wp.components` / `wp.element` / `wp.i18n` /
 * `wp.apiFetch` are defined, silently failing to register the block.
 *
 * Hand-authored here because we don't run @wordpress/scripts build for the
 * account-overview block in v1.4.2. Update the version string when edit.js
 * changes in a way that should bust the browser cache.
 *
 * @package DailyOS
 */

return [
	'dependencies' => [
		'wp-api-fetch',
		'wp-block-editor',
		'wp-blocks',
		'wp-components',
		'wp-element',
		'wp-i18n',
	],
	'version'      => '0.1.1',
];
