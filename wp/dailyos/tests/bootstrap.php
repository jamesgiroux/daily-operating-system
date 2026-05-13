<?php
/**
 * PHPUnit bootstrap for DailyOS scaffold smoke tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	define( 'ABSPATH', dirname( __DIR__ ) . '/' );
}

if ( ! defined( 'WPINC' ) ) {
	define( 'WPINC', 'wp-includes' );
}

$GLOBALS['dailyos_test_actions'] = [];

if ( ! function_exists( 'plugin_dir_path' ) ) {
	function plugin_dir_path( string $file ): string {
		return trailingslashit( dirname( $file ) );
	}
}

if ( ! function_exists( 'plugin_dir_url' ) ) {
	function plugin_dir_url( string $file ): string {
		return 'http://example.test/wp-content/plugins/' . basename( dirname( $file ) ) . '/';
	}
}

if ( ! function_exists( 'trailingslashit' ) ) {
	function trailingslashit( string $value ): string {
		return rtrim( $value, '/\\' ) . '/';
	}
}

if ( ! function_exists( 'add_action' ) ) {
	function add_action( string $hook_name, callable $callback, int $priority = 10, int $accepted_args = 1 ): bool {
		$GLOBALS['dailyos_test_actions'][ $hook_name ][ $priority ][] = [ $callback, $accepted_args ];
		return true;
	}
}

if ( ! function_exists( 'do_action' ) ) {
	function do_action( string $hook_name, mixed ...$args ): void {
		if ( empty( $GLOBALS['dailyos_test_actions'][ $hook_name ] ) ) {
			return;
		}

		ksort( $GLOBALS['dailyos_test_actions'][ $hook_name ] );

		foreach ( $GLOBALS['dailyos_test_actions'][ $hook_name ] as $callbacks ) {
			foreach ( $callbacks as [ $callback, $accepted_args ] ) {
				call_user_func_array( $callback, array_slice( $args, 0, $accepted_args ) );
			}
		}
	}
}

if ( ! function_exists( 'register_activation_hook' ) ) {
	function register_activation_hook( string $file, callable $callback ): void {
		unset( $file, $callback );
	}
}

if ( ! function_exists( 'register_deactivation_hook' ) ) {
	function register_deactivation_hook( string $file, callable $callback ): void {
		unset( $file, $callback );
	}
}

if ( ! function_exists( 'register_uninstall_hook' ) ) {
	function register_uninstall_hook( string $file, callable $callback ): void {
		unset( $file, $callback );
	}
}

require dirname( __DIR__ ) . '/dailyos.php';
do_action( 'plugins_loaded' );
