<?php
/**
 * Plugin Name: DailyOS
 * Description: DailyOS SurfaceClient for WordPress
 * Version: 0.1.0
 * Requires at least: 6.9
 * Requires PHP: 8.1
 * Text Domain: dailyos
 * License: GPL-2.0-or-later
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;

if ( ! defined( 'ABSPATH' ) ) {
	exit;
}

define( 'DAILYOS_PLUGIN_FILE', __FILE__ );
define( 'DAILYOS_PLUGIN_DIR', plugin_dir_path( __FILE__ ) );
define( 'DAILYOS_PLUGIN_URL', plugin_dir_url( __FILE__ ) );
define( 'DAILYOS_VERSION', '0.1.0' );
define( 'DAILYOS_PLUGIN_VERSION', DAILYOS_VERSION );

$dailyos_autoload = DAILYOS_PLUGIN_DIR . 'vendor/autoload.php';

if ( file_exists( $dailyos_autoload ) ) {
	require_once $dailyos_autoload;
}

if ( ! class_exists( DailyOS_Plugin::class ) ) {
	spl_autoload_register(
		static function ( string $class_name ): void {
			$prefix = 'DailyOS\\';

			if ( 0 !== strpos( $class_name, $prefix ) ) {
				return;
			}

			$relative_class = substr( $class_name, strlen( $prefix ) );
			$parts          = explode( '\\', $relative_class );
			$class          = array_pop( $parts );
			$directory      = DAILYOS_PLUGIN_DIR . 'includes/';

			if ( ! empty( $parts ) ) {
				$directory .= strtolower( implode( '/', $parts ) ) . '/';
			}

			$file = $directory . 'class-' . strtolower( str_replace( '_', '-', $class ) ) . '.php';

			if ( file_exists( $file ) ) {
				require_once $file;
			}
		}
	);
}

register_activation_hook( DAILYOS_PLUGIN_FILE, [ DailyOS_Plugin::class, 'activate' ] );
register_deactivation_hook( DAILYOS_PLUGIN_FILE, [ DailyOS_Plugin::class, 'deactivate' ] );
register_uninstall_hook( DAILYOS_PLUGIN_FILE, [ DailyOS_Plugin::class, 'uninstall' ] );

add_action(
	'plugins_loaded',
	static function (): void {
		DailyOS_Plugin::instance()->init();
	}
);
