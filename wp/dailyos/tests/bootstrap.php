<?php
/**
 * PHPUnit bootstrap for DailyOS scaffold smoke tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

// phpcs:disable
namespace {
	if ( ! defined( 'ABSPATH' ) ) {
		define( 'ABSPATH', dirname( __DIR__ ) . '/' );
	}

	if ( ! defined( 'WPINC' ) ) {
		define( 'WPINC', 'wp-includes' );
	}

	$GLOBALS['dailyos_test_actions']              = [];
	$GLOBALS['dailyos_test_current_actions']      = [];
	$GLOBALS['dailyos_test_filters']              = [];
	$GLOBALS['dailyos_test_roles']                = [];
	$GLOBALS['dailyos_test_options']              = [];
	$GLOBALS['dailyos_test_audit_events']         = [];
	$GLOBALS['dailyos_test_error_log']            = [];
	$GLOBALS['dailyos_test_mcp_server_calls']     = [];
	$GLOBALS['dailyos_test_registered_abilities'] = [];
	$GLOBALS['dailyos_test_settings_errors']      = [];
	$GLOBALS['dailyos_test_remote_post_calls']    = [];
	$GLOBALS['dailyos_test_remote_post_response'] = [
		'response' => [
			'code' => 200,
		],
		'body'     => '{"ok":true}',
	];
	$GLOBALS['dailyos_test_user_can_callback']    = null;
	$GLOBALS['dailyos_test_is_user_logged_in']    = false;
	$GLOBALS['dailyos_test_current_user_id']      = 0;
	$GLOBALS['dailyos_test_current_user_can']     = true;
	$GLOBALS['dailyos_test_check_admin_referer']  = 1;

	if ( ! defined( 'DAY_IN_SECONDS' ) ) {
		define( 'DAY_IN_SECONDS', 86400 );
	}

	if ( ! class_exists( 'WP_Error' ) ) {
		$wp_error_stub = new class( '', '' ) {
			private string $code;
			private string $message;

			public function __construct( string $code = '', string $message = '' ) {
				$this->code    = $code;
				$this->message = $message;
			}

			public function get_error_code(): string {
				return $this->code;
			}

			public function get_error_message(): string {
				return $this->message;
			}
		};
		class_alias( get_class( $wp_error_stub ), 'WP_Error' );
		unset( $wp_error_stub );
	}

	function dailyos_test_reset_globals(): void {
		$GLOBALS['dailyos_test_actions']              = [];
		$GLOBALS['dailyos_test_current_actions']      = [];
		$GLOBALS['dailyos_test_filters']              = [];
		$GLOBALS['dailyos_test_roles']                = [];
		$GLOBALS['dailyos_test_options']              = [];
		$GLOBALS['dailyos_test_audit_events']         = [];
		$GLOBALS['dailyos_test_error_log']            = [];
		$GLOBALS['dailyos_test_mcp_server_calls']     = [];
		$GLOBALS['dailyos_test_registered_abilities'] = [];
		$GLOBALS['dailyos_test_settings_errors']      = [];
		$GLOBALS['dailyos_test_remote_post_calls']    = [];
		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => '{"ok":true}',
		];
		$GLOBALS['dailyos_test_user_can_callback']    = null;
		$GLOBALS['dailyos_test_is_user_logged_in']    = false;
		$GLOBALS['dailyos_test_current_user_id']      = 0;
		$GLOBALS['dailyos_test_current_user_can']     = true;
		$GLOBALS['dailyos_test_check_admin_referer']  = 1;
	}

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

	if ( ! function_exists( 'add_filter' ) ) {
		function add_filter( string $hook_name, callable $callback, int $priority = 10, int $accepted_args = 1 ): bool {
			$GLOBALS['dailyos_test_filters'][ $hook_name ][ $priority ][] = [ $callback, $accepted_args ];
			return true;
		}
	}

	if ( ! function_exists( 'has_action' ) ) {
		function has_action( string $hook_name, mixed $callback = false ): int|false {
			if ( empty( $GLOBALS['dailyos_test_actions'][ $hook_name ] ) ) {
				return false;
			}

			ksort( $GLOBALS['dailyos_test_actions'][ $hook_name ] );

			if ( false === $callback ) {
				return (int) array_key_first( $GLOBALS['dailyos_test_actions'][ $hook_name ] );
			}

			foreach ( $GLOBALS['dailyos_test_actions'][ $hook_name ] as $priority => $callbacks ) {
				foreach ( $callbacks as [ $registered_callback ] ) {
					if ( $registered_callback === $callback ) {
						return (int) $priority;
					}
				}
			}

			return false;
		}
	}

	if ( ! function_exists( 'do_action' ) ) {
		function do_action( string $hook_name, mixed ...$args ): void {
			$GLOBALS['dailyos_test_current_actions'][] = $hook_name;

			if ( 'dailyos_mcp_audit' === $hook_name && isset( $args[0] ) && is_array( $args[0] ) ) {
				$GLOBALS['dailyos_test_audit_events'][] = $args[0];
			}

			if ( empty( $GLOBALS['dailyos_test_actions'][ $hook_name ] ) ) {
				array_pop( $GLOBALS['dailyos_test_current_actions'] );
				return;
			}

			ksort( $GLOBALS['dailyos_test_actions'][ $hook_name ] );

			foreach ( $GLOBALS['dailyos_test_actions'][ $hook_name ] as $callbacks ) {
				foreach ( $callbacks as [ $callback, $accepted_args ] ) {
					call_user_func_array( $callback, array_slice( $args, 0, $accepted_args ) );
				}
			}

			array_pop( $GLOBALS['dailyos_test_current_actions'] );
		}
	}

	if ( ! function_exists( 'doing_action' ) ) {
		function doing_action( ?string $hook_name = null ): bool {
			if ( null === $hook_name ) {
				return ! empty( $GLOBALS['dailyos_test_current_actions'] );
			}

			return in_array( $hook_name, $GLOBALS['dailyos_test_current_actions'], true );
		}
	}

	if ( ! function_exists( '_doing_it_wrong' ) ) {
		function _doing_it_wrong( string $function_name, string $message, string $version ): void {
			unset( $function_name, $message, $version );
		}
	}

	if ( ! function_exists( 'apply_filters' ) ) {
		function apply_filters( string $hook_name, mixed $value, mixed ...$args ): mixed {
			if ( empty( $GLOBALS['dailyos_test_filters'][ $hook_name ] ) ) {
				return $value;
			}

			ksort( $GLOBALS['dailyos_test_filters'][ $hook_name ] );

			foreach ( $GLOBALS['dailyos_test_filters'][ $hook_name ] as $callbacks ) {
				foreach ( $callbacks as [ $callback, $accepted_args ] ) {
					$callback_args = array_merge( [ $value ], $args );
					$value         = call_user_func_array( $callback, array_slice( $callback_args, 0, $accepted_args ) );
				}
			}

			return $value;
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

	if ( ! function_exists( 'add_role' ) ) {
		function add_role( string $role, string $display_name, array $capabilities = [] ): object {
			$GLOBALS['dailyos_test_roles'][ $role ] = new class( $role, $display_name, $capabilities ) {
				public string $name;
				public string $display_name;
				public array $capabilities;

				public function __construct( string $name, string $display_name, array $capabilities ) {
					$this->name         = $name;
					$this->display_name = $display_name;
					$this->capabilities = $capabilities;
				}

				public function add_cap( string $capability, bool $grant = true ): void {
					$this->capabilities[ $capability ] = $grant;
				}
			};
			return $GLOBALS['dailyos_test_roles'][ $role ];
		}
	}

	if ( ! function_exists( 'get_role' ) ) {
		function get_role( string $role ): ?object {
			return $GLOBALS['dailyos_test_roles'][ $role ] ?? null;
		}
	}

	if ( ! function_exists( 'remove_role' ) ) {
		function remove_role( string $role ): void {
			unset( $GLOBALS['dailyos_test_roles'][ $role ] );
		}
	}

	if ( ! function_exists( 'add_cap' ) ) {
		function add_cap( object $role, string $capability, bool $grant = true ): void {
			if ( method_exists( $role, 'add_cap' ) ) {
				$role->add_cap( $capability, $grant );
			}
		}
	}

	if ( ! function_exists( 'user_can' ) ) {
		function user_can( int $user_id, string $capability ): bool {
			$callback = $GLOBALS['dailyos_test_user_can_callback'] ?? null;

			if ( is_callable( $callback ) ) {
				return (bool) $callback( $user_id, $capability );
			}

			return false;
		}
	}

	if ( ! function_exists( 'is_user_logged_in' ) ) {
		function is_user_logged_in(): bool {
			return (bool) ( $GLOBALS['dailyos_test_is_user_logged_in'] ?? false );
		}
	}

	if ( ! function_exists( 'get_current_user_id' ) ) {
		function get_current_user_id(): int {
			return (int) ( $GLOBALS['dailyos_test_current_user_id'] ?? 0 );
		}
	}

	if ( ! function_exists( 'get_option' ) ) {
		function get_option( string $option, mixed $default_value = false ): mixed {
			return array_key_exists( $option, $GLOBALS['dailyos_test_options'] )
				? $GLOBALS['dailyos_test_options'][ $option ]
				: $default_value;
		}
	}

	if ( ! function_exists( 'update_option' ) ) {
		function update_option( string $option, mixed $value, mixed $autoload = null ): bool {
			unset( $autoload );
			$GLOBALS['dailyos_test_options'][ $option ] = $value;
			return true;
		}
	}

	if ( ! function_exists( 'delete_option' ) ) {
		function delete_option( string $option ): bool {
			unset( $GLOBALS['dailyos_test_options'][ $option ] );
			return true;
		}
	}

	if ( ! function_exists( 'add_menu_page' ) ) {
		function add_menu_page( string $page_title, string $menu_title, string $capability, string $menu_slug, callable $callback, string $icon_url = '', ?int $position = null ): string {
			unset( $page_title, $menu_title, $capability, $menu_slug, $callback, $icon_url, $position );
			return 'dailyos_page';
		}
	}

	if ( ! function_exists( 'add_submenu_page' ) ) {
		function add_submenu_page( string $parent_slug, string $page_title, string $menu_title, string $capability, string $menu_slug, callable $callback ): string {
			unset( $parent_slug, $page_title, $menu_title, $capability, $menu_slug, $callback );
			return 'dailyos_subpage';
		}
	}

	if ( ! function_exists( 'wp_remote_post' ) ) {
		function wp_remote_post( string $url, array $args = [] ): mixed {
			$GLOBALS['dailyos_test_remote_post_calls'][] = [
				'url'  => $url,
				'args' => $args,
			];

			$response = $GLOBALS['dailyos_test_remote_post_response'];

			if ( is_callable( $response ) ) {
				return $response( $url, $args );
			}

			return $response;
		}
	}

	if ( ! function_exists( 'wp_remote_retrieve_response_code' ) ) {
		function wp_remote_retrieve_response_code( mixed $response ): int {
			return is_array( $response ) ? (int) ( $response['response']['code'] ?? 0 ) : 0;
		}
	}

	if ( ! function_exists( 'wp_remote_retrieve_body' ) ) {
		function wp_remote_retrieve_body( mixed $response ): string {
			return is_array( $response ) ? (string) ( $response['body'] ?? '' ) : '';
		}
	}

	if ( ! function_exists( 'get_site_url' ) ) {
		function get_site_url(): string {
			return 'http://example.test';
		}
	}

	if ( ! function_exists( 'wp_get_current_user' ) ) {
		function wp_get_current_user(): object {
			return (object) [
				'user_login' => 'admin',
			];
		}
	}

	if ( ! function_exists( 'sanitize_text_field' ) ) {
		function sanitize_text_field( mixed $value ): string {
			if ( is_array( $value ) || is_object( $value ) ) {
				return '';
			}

			$value = (string) $value;
			$value = preg_replace( '/[\r\n\t\0\x0B]+/', ' ', $value );

			return trim( strip_tags( (string) $value ) );
		}
	}

	if ( ! function_exists( 'wp_unslash' ) ) {
		function wp_unslash( mixed $value ): mixed {
			if ( is_array( $value ) ) {
				return array_map( 'wp_unslash', $value );
			}

			return is_string( $value ) ? stripslashes( $value ) : $value;
		}
	}

	if ( ! function_exists( 'esc_html' ) ) {
		function esc_html( mixed $text ): string {
			return htmlspecialchars( (string) $text, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8' );
		}
	}

	if ( ! function_exists( 'esc_html__' ) ) {
		function esc_html__( string $text, string $domain = 'default' ): string {
			unset( $domain );
			return esc_html( $text );
		}
	}

	if ( ! function_exists( 'esc_html_e' ) ) {
		function esc_html_e( string $text, string $domain = 'default' ): void {
			echo esc_html__( $text, $domain );
		}
	}

	if ( ! function_exists( 'esc_attr' ) ) {
		function esc_attr( mixed $text ): string {
			return esc_html( $text );
		}
	}

	if ( ! function_exists( '__' ) ) {
		function __( string $text, string $domain = 'default' ): string {
			unset( $domain );
			return $text;
		}
	}

	if ( ! function_exists( '_x' ) ) {
		function _x( string $text, string $context, string $domain = 'default' ): string {
			unset( $context, $domain );
			return $text;
		}
	}

	if ( ! function_exists( 'add_settings_error' ) ) {
		function add_settings_error( string $setting, string $code, string $message, string $type = 'error' ): void {
			$GLOBALS['dailyos_test_settings_errors'][ $setting ][] = [
				'code'    => $code,
				'message' => $message,
				'type'    => $type,
			];
		}
	}

	if ( ! function_exists( 'settings_errors' ) ) {
		function settings_errors( string $setting = '' ): void {
			$errors = '' === $setting ? $GLOBALS['dailyos_test_settings_errors'] : ( $GLOBALS['dailyos_test_settings_errors'][ $setting ] ?? [] );

			foreach ( $errors as $error ) {
				if ( is_array( $error ) && isset( $error['message'] ) ) {
					echo '<div>' . esc_html( $error['message'] ) . '</div>';
				}
			}
		}
	}

	if ( ! function_exists( 'wp_nonce_field' ) ) {
		function wp_nonce_field( string|int $action = -1, string $name = '_wpnonce', bool $referer = true, bool $display = true ): string {
			unset( $action, $referer );
			$field = '<input type="hidden" name="' . esc_attr( $name ) . '" value="test-nonce" />';

			if ( $display ) {
				echo $field;
			}

			return $field;
		}
	}

	if ( ! function_exists( 'check_admin_referer' ) ) {
		function check_admin_referer( string|int $action = -1, string $query_arg = '_wpnonce' ): int|false {
			unset( $action, $query_arg );
			return $GLOBALS['dailyos_test_check_admin_referer'] ?? 1;
		}
	}

	if ( ! function_exists( 'current_user_can' ) ) {
		function current_user_can( string $capability, mixed ...$args ): bool {
			unset( $capability, $args );
			return (bool) ( $GLOBALS['dailyos_test_current_user_can'] ?? true );
		}
	}

	if ( ! function_exists( 'wp_die' ) ) {
		function wp_die( mixed $message = '' ): never {
			if ( $message instanceof WP_Error ) {
				throw new RuntimeException( $message->get_error_message() );
			}

			throw new RuntimeException( (string) $message );
		}
	}

	if ( ! function_exists( 'submit_button' ) ) {
		function submit_button( string $text = 'Save Changes' ): void {
			echo '<button type="submit">' . esc_html( $text ) . '</button>';
		}
	}

	if ( ! function_exists( 'human_time_diff' ) ) {
		function human_time_diff( int $from, ?int $to = null ): string {
			$to      = $to ?? time();
			$seconds = max( 0, abs( $to - $from ) );

			if ( 60 > $seconds ) {
				return $seconds . ' seconds';
			}

			$minutes = (int) floor( $seconds / 60 );

			if ( 60 > $minutes ) {
				return $minutes . ' minutes';
			}

			$hours = (int) floor( $minutes / 60 );

			return $hours . ' hours';
		}
	}

	if ( ! function_exists( 'wp_register_ability' ) ) {
		function wp_register_ability( string $name, array $args ): bool {
			$GLOBALS['dailyos_test_registered_abilities'][ $name ] = $args;
			return true;
		}
	}

	if ( ! function_exists( 'is_wp_error' ) ) {
		function is_wp_error( mixed $thing ): bool {
			return $thing instanceof WP_Error;
		}
	}

	if ( ! function_exists( 'wp_json_encode' ) ) {
		function wp_json_encode( mixed $value, int $flags = 0, int $depth = 512 ): string|false {
			return json_encode( $value, $flags, $depth );
		}
	}
}

namespace DailyOS\Mcp {
	function error_log( string $message ): bool {
		$GLOBALS['dailyos_test_error_log'][] = $message;
		return true;
	}
}

namespace WP\MCP\Transport {
	if ( ! class_exists( __NAMESPACE__ . '\\HttpTransport', false ) ) {
		$http_transport_stub = new class() {};
		class_alias( get_class( $http_transport_stub ), __NAMESPACE__ . '\\HttpTransport' );
		unset( $http_transport_stub );
	}
}

namespace WP\MCP\Core {
	if ( ! class_exists( __NAMESPACE__ . '\\McpAdapter', false ) ) {
		$mcp_adapter_stub = new class() {
			private static ?self $instance = null;

			public static function instance(): self {
				if ( null === self::$instance ) {
					self::$instance = new self();
				}

				return self::$instance;
			}

			public function create_server(
				string $server_id,
				string $server_route_namespace,
				string $server_route,
				string $server_name,
				string $server_description,
				string $server_version,
				array $mcp_transports,
				?string $error_handler,
				?string $observability_handler = null,
				array $tools = [],
				array $resources = [],
				array $prompts = [],
				?callable $transport_permission_callback = null
			): self {
				$GLOBALS['dailyos_test_mcp_server_calls'][] = [
					'server_id'                     => $server_id,
					'namespace'                     => $server_route_namespace,
					'route'                         => $server_route,
					'name'                          => $server_name,
					'description'                   => $server_description,
					'version'                       => $server_version,
					'transports'                    => $mcp_transports,
					'error_handler'                 => $error_handler,
					'observability_handler'         => $observability_handler,
					'tools'                         => $tools,
					'resources'                     => $resources,
					'prompts'                       => $prompts,
					'transport_permission_callback' => $transport_permission_callback,
				];

				return $this;
			}
		};
		class_alias( get_class( $mcp_adapter_stub ), __NAMESPACE__ . '\\McpAdapter' );
		unset( $mcp_adapter_stub );
	}
}

namespace {
	require dirname( __DIR__ ) . '/dailyos.php';
	do_action( 'plugins_loaded' );
}
// phpcs:enable
