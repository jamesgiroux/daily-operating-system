<?php
/**
 * Inline WP-CLI stubs for PHPStan.
 *
 * The plugin guards every WP-CLI call site with `defined('WP_CLI') && WP_CLI`,
 * so the WP-CLI class is only present at runtime under wp-cli/wp-cli. These
 * stubs declare the surface PHPStan needs to type-check `wp dailyos status`
 * and `wp dailyos repair-namespace`.
 *
 * @package DailyOS
 */

if ( ! class_exists( 'WP_CLI' ) ) {
	class WP_CLI {
		public static function line( string $message = '' ): void {}

		public static function success( string $message ): void {}

		public static function warning( string $message ): void {}

		/**
		 * @param string|\Throwable $message
		 */
		public static function error( $message, bool $exit = true ): void {}

		public static function log( string $message ): void {}

		/**
		 * @param string                $name
		 * @param callable|string|array $callable
		 * @param array<string, mixed>  $args
		 */
		public static function add_command( string $name, $callable, array $args = [] ): void {}
	}
}
