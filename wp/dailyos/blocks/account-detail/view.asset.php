<?php
/**
 * Front-end account-detail view-switcher asset manifest.
 *
 * The view script filters FloatingNavIsland chapter items after chrome.js has
 * built the nav, so it depends on the theme's dailyos-chrome handle.
 *
 * @package dailyos
 */

return [
	'dependencies' => [
		'dailyos-chrome',
	],
	'version'      => '1.0.0',
];
