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
	$GLOBALS['dailyos_test_users']                = [];
	$GLOBALS['dailyos_test_next_user_id']         = 1;
	$GLOBALS['dailyos_test_deleted_users']        = [];
	$GLOBALS['dailyos_test_options']              = [];
	$GLOBALS['dailyos_test_post_meta_keys']       = [];
	$GLOBALS['dailyos_test_user_meta_keys']       = [];
	$GLOBALS['dailyos_test_post_types']           = [];
	$GLOBALS['dailyos_test_tables']               = [];
	$GLOBALS['dailyos_test_audit_events']         = [];
	$GLOBALS['dailyos_test_error_log']            = [];
	$GLOBALS['dailyos_test_mcp_server_calls']     = [];
	$GLOBALS['dailyos_test_registered_abilities'] = [];
	$GLOBALS['dailyos_test_settings_errors']      = [];
	$GLOBALS['dailyos_test_remote_post_calls']    = [];
	$GLOBALS['dailyos_test_rest_routes']          = [];
	$GLOBALS['dailyos_test_parse_blocks_result']  = null;
	$GLOBALS['dailyos_test_serialized_blocks']    = null;
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
	$GLOBALS['dailyos_test_force_wp_create_user_error'] = false;
	$GLOBALS['dailyos_test_next_uuid']            = 1;
	$GLOBALS['dailyos_test_current_blog_id']      = 1;
	$GLOBALS['dailyos_test_is_multisite']         = false;

	if ( ! defined( 'DAY_IN_SECONDS' ) ) {
		define( 'DAY_IN_SECONDS', 86400 );
	}

	if ( ! class_exists( 'WP_Error' ) ) {
		$wp_error_stub = new class( '', '' ) {
			private string $code;
			private string $message;
			private mixed $data;

			public function __construct( string $code = '', string $message = '', mixed $data = null ) {
				$this->code    = $code;
				$this->message = $message;
				$this->data    = $data;
			}

			public function get_error_code(): string {
				return $this->code;
			}

			public function get_error_message(): string {
				return $this->message;
			}

			public function get_error_data(): mixed {
				return $this->data;
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
		$GLOBALS['dailyos_test_users']                = [];
		$GLOBALS['dailyos_test_next_user_id']         = 1;
		$GLOBALS['dailyos_test_deleted_users']        = [];
		$GLOBALS['dailyos_test_options']              = [];
		$GLOBALS['dailyos_test_post_meta_keys']       = [];
		$GLOBALS['dailyos_test_user_meta_keys']       = [];
		$GLOBALS['dailyos_test_post_types']           = [];
		$GLOBALS['dailyos_test_tables']               = [];
		$GLOBALS['dailyos_test_audit_events']         = [];
		$GLOBALS['dailyos_test_error_log']            = [];
		$GLOBALS['dailyos_test_mcp_server_calls']     = [];
		$GLOBALS['dailyos_test_registered_abilities'] = [];
		$GLOBALS['dailyos_test_settings_errors']      = [];
		$GLOBALS['dailyos_test_remote_post_calls']    = [];
		$GLOBALS['dailyos_test_rest_routes']          = [];
		$GLOBALS['dailyos_test_parse_blocks_result']  = null;
		$GLOBALS['dailyos_test_serialized_blocks']    = null;
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
		$GLOBALS['dailyos_test_force_wp_create_user_error'] = false;
		$GLOBALS['dailyos_test_next_uuid']            = 1;
		$GLOBALS['dailyos_test_current_blog_id']      = 1;
		$GLOBALS['dailyos_test_is_multisite']         = false;
	}

	$GLOBALS['wpdb'] = new class() {
		public string $options = 'wp_options';
		public string $postmeta = 'wp_postmeta';
		public string $posts = 'wp_posts';
		public string $usermeta = 'wp_usermeta';
		public string $prefix = 'wp_';

		public function esc_like( string $text ): string {
			return addcslashes( $text, '_%\\' );
		}

		public function prepare( string $query, mixed ...$args ): string {
			foreach ( $args as $arg ) {
				$query = preg_replace( '/%s/', "'" . (string) $arg . "'", $query, 1 ) ?? $query;
			}

			return $query;
		}

		/**
		 * @return array<int, string>
		 */
		public function get_col( string $sql ): array {
			if ( str_contains( $sql, $this->options ) ) {
				if ( str_contains( $sql, 'transient' ) ) {
					return array_values(
						array_filter(
							array_keys( $GLOBALS['dailyos_test_options'] ),
							static function ( string $option_name ): bool {
								return str_starts_with( $option_name, '_transient_dailyos_' )
									|| str_starts_with( $option_name, '_transient_timeout_dailyos_' );
							}
						)
					);
				}

				return array_values(
					array_filter(
						array_keys( $GLOBALS['dailyos_test_options'] ),
						static function ( string $option_name ): bool {
							return str_starts_with( $option_name, 'dailyos_' );
						}
					)
				);
			}

			if ( str_contains( $sql, $this->postmeta ) ) {
				return array_values(
					array_filter(
						$GLOBALS['dailyos_test_post_meta_keys'],
						static function ( string $meta_key ): bool {
							return str_starts_with( $meta_key, '_dailyos_' );
						}
					)
				);
			}

			if ( str_contains( $sql, $this->usermeta ) ) {
				return array_values(
					array_filter(
						$GLOBALS['dailyos_test_user_meta_keys'] ?? [],
						static function ( string $meta_key ): bool {
							return str_starts_with( $meta_key, 'dailyos_' );
						}
					)
				);
			}

			if ( str_contains( $sql, $this->posts ) ) {
				return array_values(
					array_filter(
						$GLOBALS['dailyos_test_post_types'] ?? [],
						static function ( string $post_type ): bool {
							return str_starts_with( $post_type, 'dailyos_' );
						}
					)
				);
			}

			return [];
		}

		public function get_var( string $sql ): ?string {
			foreach ( $GLOBALS['dailyos_test_tables'] as $table ) {
				if ( str_contains( $sql, (string) $table ) ) {
					return (string) $table;
				}
			}

			return null;
		}

		public function query( string $sql ): int|false {
			if ( str_contains( $sql, $this->options ) && str_contains( $sql, '_transient_dailyos_' ) ) {
				foreach ( array_keys( $GLOBALS['dailyos_test_options'] ) as $option_name ) {
					if (
						str_starts_with( $option_name, '_transient_dailyos_' )
						|| str_starts_with( $option_name, '_transient_timeout_dailyos_' )
					) {
						unset( $GLOBALS['dailyos_test_options'][ $option_name ] );
					}
				}
			}

			if ( str_contains( $sql, $this->postmeta ) ) {
				$GLOBALS['dailyos_test_post_meta_keys'] = array_values(
					array_filter(
						$GLOBALS['dailyos_test_post_meta_keys'],
						static function ( string $meta_key ): bool {
							return ! str_starts_with( $meta_key, '_dailyos_' );
						}
					)
				);
			}

			return 1;
		}
	};

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

	if ( ! function_exists( 'register_rest_route' ) ) {
		function register_rest_route( string $namespace, string $route, array $args = [], bool $override = false ): bool {
			unset( $override );
			$GLOBALS['dailyos_test_rest_routes'][ $namespace . $route ] = $args;
			return true;
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

	if ( ! function_exists( 'wp_roles' ) ) {
		function wp_roles(): object {
			return new class() {
				/**
				 * Slug → display name pairs, sourced from the test role registry.
				 *
				 * @var array<string, string>
				 */
				public array $role_names;

				public function __construct() {
					$this->role_names = [];

					foreach ( $GLOBALS['dailyos_test_roles'] ?? [] as $slug => $role ) {
						$display              = is_object( $role ) && isset( $role->name ) ? (string) $role->name : (string) $slug;
						$this->role_names[ $slug ] = $display;
					}
				}
			};
		}
	}

	if ( ! function_exists( 'wp_generate_password' ) ) {
		function wp_generate_password( int $length = 12, bool $special_chars = true, bool $extra_special_chars = false ): string {
			unset( $special_chars, $extra_special_chars );
			return str_repeat( 'p', $length );
		}
	}

	if ( ! function_exists( 'get_user_by' ) ) {
		function get_user_by( string $field, int|string $value ): object|false {
			foreach ( $GLOBALS['dailyos_test_users'] as $user ) {
				if ( 'id' === $field && (int) $user->ID === (int) $value ) {
					return $user;
				}

				if ( 'login' === $field && $user->user_login === (string) $value ) {
					return $user;
				}
			}

			return false;
		}
	}

	if ( ! function_exists( 'wp_create_user' ) ) {
		function wp_create_user( string $username, string $password, string $email = '' ): int|\WP_Error {
			unset( $password );

			if ( ! empty( $GLOBALS['dailyos_test_force_wp_create_user_error'] ) ) {
				return new \WP_Error( 'dailyos_test_forced_create_error', 'Forced test error.' );
			}

			if ( false !== get_user_by( 'login', $username ) ) {
				return new \WP_Error( 'existing_user_login', 'User already exists.' );
			}

			$user_id = (int) $GLOBALS['dailyos_test_next_user_id']++;
			$user    = new class( $user_id, $username, $email ) {
				public int $ID;
				public string $user_login;
				public string $user_email;
				public array $roles = [];

				public function __construct( int $user_id, string $username, string $email ) {
					$this->ID         = $user_id;
					$this->user_login = $username;
					$this->user_email = $email;
				}

				public function set_role( string $role ): void {
					$this->roles = [ $role ];
				}

				public function add_role( string $role ): void {
					if ( ! in_array( $role, $this->roles, true ) ) {
						$this->roles[] = $role;
					}
				}
			};

			$GLOBALS['dailyos_test_users'][ $user_id ] = $user;

			return $user_id;
		}
	}

	if ( ! function_exists( 'wp_delete_user' ) ) {
		function wp_delete_user( int $user_id, ?int $reassign = null ): bool {
			unset( $reassign );
			$GLOBALS['dailyos_test_deleted_users'][] = $user_id;
			unset( $GLOBALS['dailyos_test_users'][ $user_id ] );

			return true;
		}
	}

	if ( ! function_exists( 'wp_set_current_user' ) ) {
		function wp_set_current_user( int $user_id, string $name = '' ): object|false {
			unset( $name );
			$GLOBALS['dailyos_test_current_user_id'] = $user_id;

			return get_user_by( 'id', $user_id );
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

			$user = get_user_by( 'id', $user_id );

			if ( false !== $user && isset( $user->roles ) && is_array( $user->roles ) ) {
				foreach ( $user->roles as $role_name ) {
					$role = get_role( (string) $role_name );

					if ( null !== $role && ! empty( $role->capabilities[ $capability ] ) ) {
						return true;
					}
				}
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

	if ( ! function_exists( 'wp_parse_url' ) ) {
		function wp_parse_url( string $url, int $component = -1 ): mixed {
			if ( -1 === $component ) {
				return parse_url( $url );
			}

			return parse_url( $url, $component );
		}
	}

	if ( ! function_exists( 'home_url' ) ) {
		function home_url(): string {
			return 'https://example.test';
		}
	}

	if ( ! function_exists( 'site_url' ) ) {
		function site_url(): string {
			return 'https://example.test';
		}
	}

	if ( ! function_exists( 'get_current_blog_id' ) ) {
		function get_current_blog_id(): int {
			return (int) ( $GLOBALS['dailyos_test_current_blog_id'] ?? 1 );
		}
	}

	if ( ! function_exists( 'is_multisite' ) ) {
		function is_multisite(): bool {
			return (bool) ( $GLOBALS['dailyos_test_is_multisite'] ?? false );
		}
	}

	if ( ! function_exists( 'wp_generate_uuid4' ) ) {
		function wp_generate_uuid4(): string {
			$next = (int) ( $GLOBALS['dailyos_test_next_uuid'] ?? 1 );
			$GLOBALS['dailyos_test_next_uuid'] = $next + 1;

			return sprintf( '00000000-0000-4000-8000-%012d', $next );
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

	if ( ! function_exists( 'parse_blocks' ) ) {
		function parse_blocks( string $content ): array {
			if ( is_array( $GLOBALS['dailyos_test_parse_blocks_result'] ) ) {
				return $GLOBALS['dailyos_test_parse_blocks_result'];
			}

			$decoded = json_decode( $content, true );

			return is_array( $decoded ) ? $decoded : [];
		}
	}

	if ( ! function_exists( 'serialize_block' ) ) {
		function serialize_block( array $block ): string {
			$encoded = wp_json_encode( $block );

			return is_string( $encoded ) ? $encoded : '';
		}
	}

	if ( ! function_exists( 'serialize_blocks' ) ) {
		function serialize_blocks( array $blocks ): string {
			$GLOBALS['dailyos_test_serialized_blocks'] = $blocks;
			$encoded = wp_json_encode( $blocks );

			return is_string( $encoded ) ? $encoded : '';
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
				$server = new class( $server_id, $server_route_namespace ) {
					private string $server_id;
					private string $server_route_namespace;

					public function __construct( string $server_id, string $server_route_namespace ) {
						$this->server_id              = $server_id;
						$this->server_route_namespace = $server_route_namespace;
					}

					public function get_server_id(): string {
						return $this->server_id;
					}

					public function get_server_route_namespace(): string {
						return $this->server_route_namespace;
					}
				};
				$tool_dtos = array_map(
					static function ( mixed $tool ): object {
						if ( is_object( $tool ) && method_exists( $tool, 'get_protocol_dto' ) ) {
							return $tool->get_protocol_dto();
						}

						$name = is_string( $tool ) ? str_replace( '/', '-', $tool ) : 'unknown-tool';

						return new class( $name ) {
							private string $name;

							public function __construct( string $name ) {
								$this->name = $name;
							}

							public function getName(): string {
								return $this->name;
							}
						};
					},
					$tools
				);
				$enumerated_tools = \apply_filters( 'mcp_adapter_tools_list', $tool_dtos, $server );

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
					'enumerated_tools'              => array_map(
						static function ( object $tool ): string {
							return method_exists( $tool, 'getName' ) ? $tool->getName() : '';
						},
						is_array( $enumerated_tools ) ? $enumerated_tools : []
					),
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
