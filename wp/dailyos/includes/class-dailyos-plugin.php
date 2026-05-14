<?php
/**
 * Main DailyOS plugin composition root.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS;

use DailyOS\Admin\DailyOS_Pairing_Page;
use DailyOS\Admin\DailyOS_Settings_Page;
use DailyOS\CLI\DailyOS_CLI;
use DailyOS\Transport\DailyOS_Credential_Store;
use DailyOS\Mcp\DailyOS_Mcp_Roles;
use DailyOS\Mcp\DailyOS_Mcp_Server;

/**
 * Coordinates WordPress hooks for the DailyOS SurfaceClient shell.
 */
final class DailyOS_Plugin {
	/**
	 * Singleton instance.
	 *
	 * @var self|null
	 */
	private static ?self $instance = null;

	/**
	 * Whether runtime hooks have already been registered.
	 *
	 * @var bool
	 */
	private bool $initialized = false;

	/**
	 * Constructor.
	 */
	private function __construct() {}

	/**
	 * Return the shared plugin instance.
	 */
	public static function instance(): self {
		if ( null === self::$instance ) {
			self::$instance = new self();
		}

		return self::$instance;
	}

	/**
	 * Register feature hooks after WordPress has loaded plugins.
	 */
	public function init(): void {
		if ( $this->initialized ) {
			return;
		}

		$this->initialized = true;

		$this->register_transport();

		// WP 6.9+ requires ability registration on the dedicated abilities-API hook.
		// Calling wp_register_ability() outside this action triggers a _doing_it_wrong
		// notice and skips the registration entirely.
		add_action( 'wp_abilities_api_categories_init', [ $this, 'register_ability_categories' ], 10 );
		add_action( 'wp_abilities_api_init', [ $this, 'register_abilities' ], 10 );
		add_action( 'init', [ $this, 'register_blocks' ], 11 );
		add_action( 'init', [ $this, 'register_mcp_server_config' ], 12 );
		add_action( 'init', [ $this, 'register_save_hooks' ], 13 );
		add_action( 'admin_menu', [ $this, 'register_admin_pages' ], 10 );
		add_action( 'rest_api_init', [ $this, 'register_rest_routes' ], 10 );

		add_action( 'dailyos_nonce_sweep', [ $this, 'sweep_presence_nonces' ] );

		if ( defined( 'WP_CLI' ) && WP_CLI ) {
			DailyOS_CLI::register();
		}
	}

	/**
	 * Activation hook.
	 */
	public static function activate(): void {
		DailyOS_Activation::activate();
	}

	/**
	 * Deactivation hook.
	 */
	public static function deactivate(): void {
		DailyOS_Activation::deactivate();
	}

	/**
	 * Uninstall hook.
	 */
	public static function uninstall(): void {
		DailyOS_Activation::uninstall();
	}

	/**
	 * Register DailyOS abilities from the local inventory.
	 */
	public function register_abilities(): void {
		$registry = new DailyOS_Ability_Registry();
		$registry->register_all();
	}

	/**
	 * Register DailyOS ability categories from the local inventory.
	 */
	public function register_ability_categories(): void {
		$registry = new DailyOS_Ability_Registry();
		$registry->register_categories();
	}

	/**
	 * Register block metadata packages when present.
	 */
	public function register_blocks(): void {
		if ( ! function_exists( 'register_block_type_from_metadata' ) ) {
			return;
		}

		$block_files = glob( DAILYOS_PLUGIN_DIR . 'blocks/*/block.json' );

		if ( false === $block_files ) {
			return;
		}

		foreach ( $block_files as $block_file ) {
			register_block_type_from_metadata( dirname( $block_file ) );
		}
	}

	/**
	 * Register admin page shells.
	 */
	public function register_admin_pages(): void {
		DailyOS_Pairing_Page::register();
		DailyOS_Settings_Page::register();
	}

	/**
	 * Register transport-layer hooks.
	 */
	public function register_transport(): void {
		$this->register_session_refresh_filter();
		( new DailyOS_Credential_Store() )->register_session_key_filter_safeguard();
	}

	/**
	 * Register the runtime refresh endpoint as the session material source.
	 */
	private function register_session_refresh_filter(): void {
		if ( ! function_exists( 'add_filter' ) ) {
			return;
		}

		add_filter(
			'dailyos_wp_bridge_session_key',
			[ $this, 'refresh_session_key' ],
			10,
			1
		);
	}

	/**
	 * Resolve process-local session material from the paired runtime.
	 *
	 * @param mixed $candidate Existing filter value.
	 * @return mixed Existing candidate, normalized session material, or null.
	 */
	public function refresh_session_key( mixed $candidate ): mixed {
		if ( null !== $candidate ) {
			return $candidate;
		}

		$marker = ( new DailyOS_Credential_Store() )->get_marker();

		if ( null === $marker ) {
			return null;
		}

		$session_id           = self::marker_string( $marker, 'session_id' );
		$site_binding_digest  = self::marker_string( $marker, 'site_binding_digest' );
		$wp_install_uuid      = self::marker_string( $marker, 'wp_install_uuid' );
		$plugin_instance_uuid = self::marker_string( $marker, 'plugin_instance_uuid' );
		$runtime_url          = self::marker_string( $marker, 'runtime_url' );

		if (
			null === $session_id
			|| null === $site_binding_digest
			|| null === $wp_install_uuid
			|| null === $plugin_instance_uuid
			|| null === $runtime_url
		) {
			return null;
		}

		$runtime_base_url = self::normalize_loopback_runtime_url( $runtime_url );

		if ( null === $runtime_base_url ) {
			return null;
		}

		$body_bytes = wp_json_encode(
			[
				'session_id'           => $session_id,
				'site_binding_digest'  => $site_binding_digest,
				'wp_install_uuid'      => $wp_install_uuid,
				'plugin_instance_uuid' => $plugin_instance_uuid,
			],
			JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE
		);

		if ( ! is_string( $body_bytes ) ) {
			return null;
		}

		$response = wp_remote_post(
			$runtime_base_url . '/v1/surface/session/refresh',
			[
				'body'        => $body_bytes,
				'headers'     => [
					'Content-Type' => 'application/json',
					'Accept'       => 'application/json',
				],
				'redirection' => 0,
				'timeout'     => 5,
				'sslverify'   => false,
				'blocking'    => true,
				'data_format' => 'body',
			]
		);

		if ( is_wp_error( $response ) || 200 !== (int) wp_remote_retrieve_response_code( $response ) ) {
			return null;
		}

		$body    = (string) wp_remote_retrieve_body( $response );
		$decoded = '' === $body ? null : json_decode( $body, true );

		if ( ! is_array( $decoded ) || true !== ( $decoded['ok'] ?? false ) ) {
			return null;
		}

		$hmac_key_hex = $decoded['hmac_key'] ?? null;

		if ( ! is_string( $hmac_key_hex ) || 64 !== strlen( $hmac_key_hex ) || ! ctype_xdigit( $hmac_key_hex ) ) {
			return null;
		}

		$hmac_key = hex2bin( $hmac_key_hex );

		if ( ! is_string( $hmac_key ) || 32 !== strlen( $hmac_key ) ) {
			return null;
		}

		return [
			'hmac_key'   => $hmac_key,
			'session_id' => $session_id,
		];
	}

	/**
	 * Return a required string marker field.
	 *
	 * @param array<string, mixed> $marker Pairing marker.
	 * @param string               $key Marker key.
	 */
	private static function marker_string( array $marker, string $key ): ?string {
		if ( ! isset( $marker[ $key ] ) || ! is_string( $marker[ $key ] ) || '' === trim( $marker[ $key ] ) ) {
			return null;
		}

		return $marker[ $key ];
	}

	/**
	 * Validate and normalize a loopback runtime base URL.
	 *
	 * @param string $runtime_url Runtime URL candidate.
	 */
	private static function normalize_loopback_runtime_url( string $runtime_url ): ?string {
		$parts = wp_parse_url( trim( $runtime_url ) );

		if ( ! is_array( $parts ) ) {
			return null;
		}

		$scheme    = isset( $parts['scheme'] ) ? strtolower( (string) $parts['scheme'] ) : '';
		$host      = isset( $parts['host'] ) ? strtolower( (string) $parts['host'] ) : '';
		$port      = isset( $parts['port'] ) ? (int) $parts['port'] : 0;
		$path      = isset( $parts['path'] ) ? (string) $parts['path'] : '';
		$has_extra = isset( $parts['query'] ) || isset( $parts['fragment'] ) || ( '' !== $path && '/' !== $path );

		if ( 'http' !== $scheme || '127.0.0.1' !== $host || 1 > $port || 65535 < $port || $has_extra ) {
			return null;
		}

		return 'http://127.0.0.1:' . $port;
	}

	/**
	 * Register REST routes in later waves.
	 */
	public function register_rest_routes(): void {}

	/**
	 * Register save hooks in later waves.
	 */
	public function register_save_hooks(): void {}

	/**
	 * Handle the scheduled nonce sweep hook.
	 *
	 * Full nonce-sweep implementation lands in W4-E presence nonce lifecycle.
	 */
	public function sweep_presence_nonces(): void {}

	/**
	 * Register MCP server configuration.
	 */
	public function register_mcp_server_config(): void {
		DailyOS_Mcp_Roles::register();

		if ( function_exists( 'add_filter' ) ) {
			add_filter(
				'dailyos_surfaceclient_resolved_scopes',
				static function (): array {
					$marker = ( new DailyOS_Credential_Store() )->get_marker();

					if ( null === $marker || ! isset( $marker['granted_scopes'] ) || ! is_array( $marker['granted_scopes'] ) ) {
						return [];
					}

					return array_values(
						array_filter(
							$marker['granted_scopes'],
							static fn( mixed $scope ): bool => is_string( $scope )
						)
					);
				}
			);
		}

		$registry = new DailyOS_Ability_Registry();
		$resolver = static function (): array {
			return apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		};

		DailyOS_Mcp_Server::bootstrap( $registry, $resolver );
	}
}
