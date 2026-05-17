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
use DailyOS\Transport\DailyOS_Hmac_Signer;
use DailyOS\Transport\DailyOS_Runtime_Client;
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
		add_filter( 'block_categories_all', [ $this, 'register_block_category' ], 10, 1 );
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
	 * Register the "dailyos" block category (W4-F L4-unblock backport from
	 * wave3-l2-integration). block.json files declare `"category": "dailyos"`,
	 * but the category itself must be registered via `block_categories_all`
	 * for blocks to appear in the WP editor inserter.
	 *
	 * @param array<int,array<string,mixed>> $categories Existing categories.
	 * @return array<int,array<string,mixed>>
	 */
	public function register_block_category( array $categories ): array {
		foreach ( $categories as $category ) {
			if ( isset( $category['slug'] ) && 'dailyos' === $category['slug'] ) {
				return $categories;
			}
		}
		$categories[] = [
			'slug'  => 'dailyos',
			'title' => __( 'DailyOS', 'dailyos' ),
			'icon'  => null,
		];
		return $categories;
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
	 * In-process cache for the runtime sentinel. 5s TTL.
	 *
	 * @var array{port:int,runtime_version:string}|null
	 */
	private static ?array $sentinel_cache = null;

	/**
	 * Sentinel cache timestamp (microtime float).
	 */
	private static float $sentinel_cached_at = 0.0;

	/**
	 * Discover the current Tauri runtime endpoint via the sentinel file (W4-F DOS-636).
	 *
	 * Reads `~/.dailyos/runtime-endpoint.json` written by the Tauri runtime on bind.
	 * Payload contains ONLY `port` and `runtime_version` per W4-F packet §5/§6.4 —
	 * any payload containing `auth_token`, `session_key`, `hmac_key`, or `secret`
	 * is rejected and logged (auth material belongs in keychain, not sentinel).
	 *
	 * Defense per W4-F §6.4: sentinel is port-discovery convenience, NOT a defense.
	 * Defense is the HMAC session key in keychain. An attacker who reads the sentinel
	 * without the HMAC key cannot make any signed request. WP HMAC validation on
	 * every response catches substituted-endpoint impersonation.
	 *
	 * Mode/owner check: file MUST be 0600 and owned by the current effective user.
	 * Retry up to 3x at 100ms on ENOENT (Tauri restart race) before returning null.
	 * Result is cached in-process for 5s to avoid hot-path stat overhead.
	 *
	 * @return array{port:int,runtime_version:string}|null Decoded sentinel or null.
	 */
	public static function discover_runtime_endpoint(): ?array {
		$now = microtime( true );

		if ( null !== self::$sentinel_cache && ( $now - self::$sentinel_cached_at ) < 5.0 ) {
			return self::$sentinel_cache;
		}

		$path = self::runtime_endpoint_sentinel_path();
		if ( null === $path ) {
			return null;
		}

		$attempts = 0;
		while ( $attempts < 3 ) {
			if ( file_exists( $path ) ) {
				break;
			}
			++$attempts;
			usleep( 100000 );
		}

		if ( ! file_exists( $path ) ) {
			return null;
		}

		// Mode and ownership check. clearstatcache so we read live mode bits.
		clearstatcache( true, $path );
		$stat = @stat( $path );
		if ( false === $stat ) {
			self::log_sentinel_warning( 'stat failed' );
			return null;
		}
		// Verify mode = 0600 (only owner can read/write).
		$mode = $stat['mode'] & 0o777;
		if ( 0o600 !== $mode ) {
			self::log_sentinel_warning( sprintf( 'sentinel mode %o is not 0600', $mode ) );
			return null;
		}
		// Verify ownership matches current effective user.
		if ( function_exists( 'posix_geteuid' ) && $stat['uid'] !== posix_geteuid() ) {
			self::log_sentinel_warning( 'sentinel ownership mismatch' );
			return null;
		}

		$contents = @file_get_contents( $path );
		if ( false === $contents ) {
			self::log_sentinel_warning( 'sentinel read failed' );
			return null;
		}

		$decoded = json_decode( $contents, true );
		if ( ! is_array( $decoded ) ) {
			self::log_sentinel_warning( 'sentinel JSON decode failed' );
			return null;
		}

		// Per W4-F packet CI invariant #5 + Acceptance #14 sub-bullet: payload
		// MUST contain only port + runtime_version. Reject any auth material.
		$forbidden = array( 'auth_token', 'session_key', 'hmac_key', 'secret' );
		foreach ( $forbidden as $field ) {
			if ( array_key_exists( $field, $decoded ) ) {
				self::log_sentinel_warning( sprintf( 'sentinel contains forbidden field %s', $field ) );
				return null;
			}
		}

		if ( ! isset( $decoded['port'] ) || ! is_int( $decoded['port'] ) ) {
			return null;
		}
		if ( ! isset( $decoded['runtime_version'] ) || ! is_string( $decoded['runtime_version'] ) ) {
			return null;
		}
		$port = (int) $decoded['port'];
		if ( 1 > $port || 65535 < $port ) {
			return null;
		}

		self::$sentinel_cache     = array(
			'port'            => $port,
			'runtime_version' => (string) $decoded['runtime_version'],
		);
		self::$sentinel_cached_at = $now;

		return self::$sentinel_cache;
	}

	/**
	 * Build the loopback runtime URL from a sentinel payload.
	 *
	 * Returns null if sentinel discovery failed.
	 */
	public static function discover_runtime_base_url(): ?string {
		$sentinel = self::discover_runtime_endpoint();
		if ( null === $sentinel ) {
			return null;
		}
		$candidate = 'http://127.0.0.1:' . $sentinel['port'];
		return self::normalize_loopback_runtime_url( $candidate );
	}

	/**
	 * Reset the in-process sentinel cache. Used after ECONNREFUSED to force
	 * a fresh sentinel read on the retry (Tauri may have restarted with new port).
	 */
	public static function invalidate_runtime_endpoint_cache(): void {
		self::$sentinel_cache     = null;
		self::$sentinel_cached_at = 0.0;
	}

	/**
	 * Path to the runtime sentinel file. Returns null if HOME is unavailable.
	 */
	private static function runtime_endpoint_sentinel_path(): ?string {
		$home = getenv( 'HOME' );
		if ( ! is_string( $home ) || '' === trim( $home ) ) {
			return null;
		}
		return rtrim( $home, '/' ) . '/.dailyos/runtime-endpoint.json';
	}

	/**
	 * Best-effort warning log for sentinel anomalies. Uses error_log to avoid
	 * depending on WP_DEBUG_LOG availability at plugin init.
	 */
	private static function log_sentinel_warning( string $message ): void {
		// phpcs:ignore WordPress.PHP.DevelopmentFunctions.error_log_error_log
		error_log( '[dailyos] runtime sentinel: ' . $message );
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
	 * Register REST routes for user-presence nonce issuance.
	 */
	public function register_rest_routes(): void {
		if ( ! function_exists( 'register_rest_route' ) ) {
			return;
		}

		register_rest_route(
			'dailyos/v1',
			'/nonce',
			[
				'methods'             => 'POST',
				'callback'            => [ $this, 'issue_presence_nonce' ],
				'permission_callback' => [ $this, 'can_issue_presence_nonce' ],
			]
		);

		register_rest_route(
			'dailyos/v1',
			'/account-overview/preview',
			[
				'methods'             => 'POST',
				'callback'            => [ $this, 'account_overview_preview' ],
				'permission_callback' => [ $this, 'can_edit_posts_rest' ],
			]
		);

		register_rest_route(
			'dailyos/v1',
			'/account-overview/accounts',
			[
				'methods'             => 'GET',
				'callback'            => [ $this, 'account_overview_accounts' ],
				'permission_callback' => [ $this, 'can_edit_posts_rest' ],
			]
		);
	}

	/**
	 * Permission callback for editor-only REST routes.
	 *
	 * @return bool|\WP_Error
	 */
	public function can_edit_posts_rest(): bool|\WP_Error {
		if ( function_exists( 'is_user_logged_in' ) && ! is_user_logged_in() ) {
			return new \WP_Error( 'dailyos_unauthenticated', __( 'Sign in to use this endpoint.', 'dailyos' ), [ 'status' => 401 ] );
		}
		if ( function_exists( 'current_user_can' ) && ! current_user_can( 'edit_posts' ) ) {
			return new \WP_Error( 'dailyos_forbidden', __( 'You cannot use this endpoint.', 'dailyos' ), [ 'status' => 403 ] );
		}
		if ( ! ( new DailyOS_Credential_Store() )->is_paired() ) {
			return new \WP_Error( 'dailyos_not_paired', __( 'DailyOS is not paired with a runtime.', 'dailyos' ), [ 'status' => 403 ] );
		}
		return true;
	}

	/**
	 * Editor preview: server-side projection fetch via the runtime client.
	 *
	 * @param mixed $request REST request.
	 * @return array<string, mixed>|\WP_Error
	 */
	public function account_overview_preview( mixed $request ): array|\WP_Error {
		$params              = self::request_params( $request );
		$composition_id      = isset( $params['composition_id'] ) ? (string) $params['composition_id'] : '';
		$composition_version = isset( $params['composition_version'] ) ? (int) $params['composition_version'] : 0;
		$cache_hint_token    = isset( $params['cache_hint_token'] ) && '' !== $params['cache_hint_token']
			? (string) $params['cache_hint_token']
			: null;

		if ( '' === $composition_id ) {
			return new \WP_Error( 'dailyos_preview_invalid', __( 'composition_id is required.', 'dailyos' ), [ 'status' => 400 ] );
		}

		$client = $this->build_runtime_client_for_block();
		if ( $client instanceof \WP_Error ) {
			return $client;
		}

		$response = $client->project_composition_for_surface(
			$composition_id,
			$composition_version,
			$cache_hint_token
		);
		if ( is_wp_error( $response ) ) {
			return $response;
		}

		// Render the projection through the same render path to keep
		// preview parity with front-end.
		$attributes = [
			'composition_id'      => $composition_id,
			'composition_version' => isset( $response['projection']['composition_version'] )
				? (int) $response['projection']['composition_version']
				: $composition_version,
			'watermarks'          => isset( $response['projection']['watermarks'] )
				? (array) $response['projection']['watermarks']
				: [],
			'cache_hint_token'    => isset( $response['cache_hint_token'] )
				? (string) $response['cache_hint_token']
				: '',
		];
		// Render via the block.json render callback by calling render.php
		// with the attributes — same code path as the front end.
		$html = self::render_block_with_filter( $attributes, $client );
		return array_merge(
			$response,
			[
				'html'       => $html,
				'attributes' => $attributes,
			]
		);
	}

	/**
	 * Account list for the editor combobox. Returns id+name pairs from
	 * the runtime's account index without surfacing PII beyond what the
	 * runtime already exposes to a logged-in editor.
	 *
	 * @param mixed $request REST request (signature required by register_rest_route
	 *     callback contract; this stub-mode endpoint returns the same empty list
	 *     regardless of request payload — kept for the future search wiring).
	 * @return array<int, array<string, string>>|\WP_Error
	 */
	public function account_overview_accounts( mixed $request ): array|\WP_Error {
		unset( $request ); // Intentionally unused — stub returns empty list until account search ships.
		$client = $this->build_runtime_client_for_block();
		if ( $client instanceof \WP_Error ) {
			return $client;
		}
		// v1.4.2 doesn't ship a substrate-side account search endpoint;
		// return an empty list so the combobox renders without crashing.
		// Account discovery follows in the next iteration.
		return [];
	}

	/**
	 * Builds a runtime client for block rendering requests.
	 *
	 * @return \DailyOS\Transport\DailyOS_Runtime_Client|\WP_Error Runtime client or pairing error.
	 */
	private function build_runtime_client_for_block(): \DailyOS\Transport\DailyOS_Runtime_Client|\WP_Error {
		$store = new DailyOS_Credential_Store();
		if ( ! $store->is_paired() ) {
			return new \WP_Error( 'dailyos_not_paired', __( 'DailyOS is not paired.', 'dailyos' ), [ 'status' => 403 ] );
		}
		$signer = new \DailyOS\Transport\DailyOS_Hmac_Signer( $store );
		return new \DailyOS\Transport\DailyOS_Runtime_Client( $store, $signer );
	}

	/**
	 * Extracts parameters from a REST request-like value.
	 *
	 * @param mixed $request REST request or parameter array.
	 * @return array Request parameters.
	 */
	private static function request_params( mixed $request ): array {
		if ( is_array( $request ) ) {
			return $request;
		}
		if ( is_object( $request ) && method_exists( $request, 'get_params' ) ) {
			$params = $request->get_params();
			return is_array( $params ) ? $params : [];
		}
		return [];
	}

	/**
	 * Renders the account overview block with a scoped runtime client filter.
	 *
	 * @param array                                     $attributes Block attributes.
	 * @param \DailyOS\Transport\DailyOS_Runtime_Client $client     Runtime client.
	 * @return string Rendered block HTML.
	 */
	private static function render_block_with_filter( array $attributes, \DailyOS\Transport\DailyOS_Runtime_Client $client ): string {
		if ( ! function_exists( 'dailyos_account_overview_render' ) ) {
			require_once DAILYOS_PLUGIN_DIR . 'blocks/account-overview/render-functions.php';
		}
		$filter_cb = static function () use ( $client ) {
			return $client;
		};
		add_filter( 'dailyos_runtime_client_for_block', $filter_cb, 10, 0 );
		$html = dailyos_account_overview_render( $attributes );
		remove_filter( 'dailyos_runtime_client_for_block', $filter_cb, 10 );
		return $html;
	}

	/**
	 * Register save hooks that prevent ephemeral nonce serialization.
	 */
	public function register_save_hooks(): void {
		if ( ! function_exists( 'add_filter' ) ) {
			return;
		}

		add_filter( 'wp_insert_post_data', [ $this, 'strip_presence_nonces_from_post_data' ], 10, 2 );
	}

	/**
	 * Check whether the active user can request a nonce for a block gesture.
	 *
	 * @param mixed $request REST request object or payload array.
	 * @return bool|\WP_Error Permission result.
	 */
	public function can_issue_presence_nonce( mixed $request ): bool|\WP_Error {
		if ( function_exists( 'is_user_logged_in' ) && ! is_user_logged_in() ) {
			return new \WP_Error( 'dailyos_nonce_unauthenticated', __( 'Sign in before requesting a DailyOS nonce.', 'dailyos' ), [ 'status' => 401 ] );
		}

		$post_id  = self::post_id_from_request( $request );
		$can_edit = 0 < $post_id
			? current_user_can( 'edit_post', $post_id )
			: current_user_can( 'edit_posts' );

		if ( ! $can_edit ) {
			return new \WP_Error( 'dailyos_nonce_forbidden', __( 'You cannot request a DailyOS nonce for this surface.', 'dailyos' ), [ 'status' => 403 ] );
		}

		if ( ! ( new DailyOS_Credential_Store() )->is_paired() ) {
			return new \WP_Error( 'dailyos_not_paired', __( 'DailyOS is not paired with an active loopback runtime.', 'dailyos' ), [ 'status' => 403 ] );
		}

		return true;
	}

	/**
	 * Issue a user-presence nonce through the paired runtime.
	 *
	 * @param mixed $request REST request object or payload array.
	 * @return array<string, mixed>|\WP_Error Runtime response or validation error.
	 */
	public function issue_presence_nonce( mixed $request ): array|\WP_Error {
		$payload = $this->presence_nonce_payload( $request );

		if ( is_wp_error( $payload ) ) {
			return $payload;
		}

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );

		return $client->issue_nonce( $payload );
	}

	/**
	 * Strip ephemeral presence nonce attributes before post content is saved.
	 *
	 * @param array<string, mixed> $data Post data.
	 * @param array<string, mixed> $postarr Raw post array.
	 * @return array<string, mixed> Sanitized post data.
	 */
	public function strip_presence_nonces_from_post_data( array $data, array $postarr ): array {
		unset( $postarr );

		if ( isset( $data['post_content'] ) && is_string( $data['post_content'] ) ) {
			$data['post_content'] = self::strip_presence_nonces_from_content( $data['post_content'] );
		}

		return $data;
	}

	/**
	 * Strip ephemeral presence nonce attributes from serialized block content.
	 *
	 * @param string $content Serialized block content.
	 */
	public static function strip_presence_nonces_from_content( string $content ): string {
		if ( ! function_exists( 'parse_blocks' ) || ! function_exists( 'serialize_block' ) ) {
			return $content;
		}

		$blocks = parse_blocks( $content );

		if ( ! is_array( $blocks ) ) {
			return $content;
		}

		$changed = false;
		$blocks  = array_map(
			static function ( array $block ) use ( &$changed ): array {
				return self::strip_presence_nonce_from_block( $block, $changed );
			},
			$blocks
		);

		if ( ! $changed ) {
			return $content;
		}

		if ( function_exists( 'serialize_blocks' ) ) {
			return serialize_blocks( $blocks );
		}

		return implode( '', array_map( 'serialize_block', $blocks ) );
	}

	/**
	 * Handle the scheduled nonce sweep hook.
	 */
	public function sweep_presence_nonces(): void {}

	/**
	 * Build the runtime nonce issue payload from a REST request.
	 *
	 * @param mixed $request REST request object or payload array.
	 * @return array<string, mixed>|\WP_Error Runtime nonce payload or validation error.
	 */
	private function presence_nonce_payload( mixed $request ): array|\WP_Error {
		$params        = self::rest_request_params( $request );
		$claim_version = self::required_u64_param( $params, 'claim_version', 'malformed_claim_version' );

		if ( is_wp_error( $claim_version ) ) {
			return $claim_version;
		}

		$composition_version = self::required_u64_param( $params, 'composition_version', 'malformed_request' );

		if ( is_wp_error( $composition_version ) ) {
			return $composition_version;
		}

		$claim_id       = self::required_string_param( $params, 'claim_id' );
		$field_path     = self::required_string_param( $params, 'field_path' );
		$action         = self::required_string_param( $params, 'action' );
		$composition_id = self::required_string_param( $params, 'composition_id' );

		foreach ( [ $claim_id, $field_path, $action, $composition_id ] as $candidate ) {
			if ( is_wp_error( $candidate ) ) {
				return $candidate;
			}
		}

		if ( ! in_array( $action, [ 'correct', 'dismiss', 'corroborate', 'contradict' ], true ) ) {
			return self::nonce_payload_error( 'malformed_request', 400 );
		}

		$current_user_id = function_exists( 'get_current_user_id' ) ? (int) get_current_user_id() : 0;

		if ( 0 >= $current_user_id ) {
			return new \WP_Error( 'dailyos_nonce_unauthenticated', __( 'Sign in before requesting a DailyOS nonce.', 'dailyos' ), [ 'status' => 401 ] );
		}

		$credential_store = new DailyOS_Credential_Store();
		$marker           = $credential_store->get_marker();

		if ( null === $marker ) {
			return new \WP_Error( 'dailyos_not_paired', __( 'DailyOS is not paired with an active loopback runtime.', 'dailyos' ), [ 'status' => 403 ] );
		}

		$paired_wp_user_id = self::paired_wp_user_id( $marker, $current_user_id );

		if ( $paired_wp_user_id !== $current_user_id ) {
			return new \WP_Error( 'dailyos_nonce_wrong_user', __( 'This DailyOS session is paired to another WordPress user.', 'dailyos' ), [ 'status' => 403 ] );
		}

		$credential = $credential_store->retrieve_session_key();

		if ( null === $credential ) {
			return new \WP_Error( 'missing_session_key', __( 'DailyOS is not paired with an active runtime session.', 'dailyos' ), [ 'status' => 403 ] );
		}

		$payload = [
			'session_id'          => $credential->session_id(),
			'wp_user_id'          => $current_user_id,
			'claim_id'            => $claim_id,
			'field_path'          => $field_path,
			'action'              => $action,
			'claim_version'       => $claim_version,
			'composition_id'      => $composition_id,
			'composition_version' => $composition_version,
		];

		$request_id = self::optional_string_param( $params, 'request_id' );

		if ( null !== $request_id ) {
			$payload['request_id'] = $request_id;
		}

		return $payload;
	}

	/**
	 * Return request parameters from either WP_REST_Request or tests.
	 *
	 * @param mixed $request REST request object or payload array.
	 * @return array<string, mixed> Request params.
	 */
	private static function rest_request_params( mixed $request ): array {
		if ( is_array( $request ) ) {
			return $request;
		}

		if ( is_object( $request ) && method_exists( $request, 'get_json_params' ) ) {
			$params = $request->get_json_params();

			if ( is_array( $params ) ) {
				return $params;
			}
		}

		if ( is_object( $request ) && method_exists( $request, 'get_params' ) ) {
			$params = $request->get_params();

			if ( is_array( $params ) ) {
				return $params;
			}
		}

		return [];
	}

	/**
	 * Return an optional request string.
	 *
	 * @param array<string, mixed> $params Request params.
	 * @param string               $key Request key.
	 */
	private static function optional_string_param( array $params, string $key ): ?string {
		if ( ! isset( $params[ $key ] ) || ! is_string( $params[ $key ] ) ) {
			return null;
		}

		$value = trim( $params[ $key ] );

		return '' === $value || 128 < strlen( $value ) ? null : $value;
	}

	/**
	 * Return a required request string.
	 *
	 * @param array<string, mixed> $params Request params.
	 * @param string               $key Request key.
	 * @return string|\WP_Error Required string or validation error.
	 */
	private static function required_string_param( array $params, string $key ): string|\WP_Error {
		$value = self::optional_string_param( $params, $key );

		if ( null === $value ) {
			return self::nonce_payload_error( 'malformed_request', 400 );
		}

		return $value;
	}

	/**
	 * Return a required unsigned integer request value.
	 *
	 * @param array<string, mixed> $params Request params.
	 * @param string               $key Request key.
	 * @param string               $code Error code.
	 * @return int|\WP_Error Required unsigned integer or validation error.
	 */
	private static function required_u64_param( array $params, string $key, string $code ): int|\WP_Error {
		if ( ! array_key_exists( $key, $params ) || ! is_int( $params[ $key ] ) || 0 > $params[ $key ] ) {
			return self::nonce_payload_error( $code, 400 );
		}

		return $params[ $key ];
	}

	/**
	 * Build a nonce request validation error.
	 *
	 * @param string $code Error code.
	 * @param int    $status HTTP status.
	 */
	private static function nonce_payload_error( string $code, int $status ): \WP_Error {
		return new \WP_Error( $code, __( 'Refresh this block and try again.', 'dailyos' ), [ 'status' => $status ] );
	}

	/**
	 * Return the post id supplied with a REST request.
	 *
	 * @param mixed $request REST request object or payload array.
	 */
	private static function post_id_from_request( mixed $request ): int {
		$params  = self::rest_request_params( $request );
		$post_id = $params['post_id'] ?? 0;

		return is_int( $post_id ) && 0 < $post_id ? $post_id : 0;
	}

	/**
	 * Return the stable paired WordPress user id.
	 *
	 * @param array<string, mixed> $marker Pairing marker.
	 * @param int                  $fallback_user_id Fallback user id.
	 */
	private static function paired_wp_user_id( array $marker, int $fallback_user_id ): int {
		$paired_wp_user_id = $marker['paired_wp_user_id'] ?? null;

		if ( is_string( $paired_wp_user_id ) && ctype_digit( $paired_wp_user_id ) ) {
			return (int) $paired_wp_user_id;
		}

		if ( is_int( $paired_wp_user_id ) && 0 <= $paired_wp_user_id ) {
			return $paired_wp_user_id;
		}

		return $fallback_user_id;
	}

	/**
	 * Strip nonce keys from one parsed block.
	 *
	 * @param array<string, mixed> $block Parsed block.
	 * @param bool                 $changed Change flag.
	 * @return array<string, mixed> Sanitized block.
	 */
	private static function strip_presence_nonce_from_block( array $block, bool &$changed ): array {
		if ( isset( $block['attrs'] ) && is_array( $block['attrs'] ) ) {
			$block['attrs'] = self::strip_presence_nonce_from_value( $block['attrs'], $changed );
		}

		if ( isset( $block['innerBlocks'] ) && is_array( $block['innerBlocks'] ) ) {
			$block['innerBlocks'] = array_map(
				static function ( array $inner_block ) use ( &$changed ): array {
					return self::strip_presence_nonce_from_block( $inner_block, $changed );
				},
				$block['innerBlocks']
			);
		}

		return $block;
	}

	/**
	 * Strip nonce keys from arbitrary block attribute values.
	 *
	 * @param mixed $value Attribute value.
	 * @param bool  $changed Change flag.
	 * @return mixed Sanitized value.
	 */
	private static function strip_presence_nonce_from_value( mixed $value, bool &$changed ): mixed {
		if ( ! is_array( $value ) ) {
			return $value;
		}

		foreach ( [ 'presence_nonce', 'presenceNonce', 'dailyos_presence_nonce', 'dailyosPresenceNonce' ] as $nonce_key ) {
			if ( array_key_exists( $nonce_key, $value ) ) {
				unset( $value[ $nonce_key ] );
				$changed = true;
			}
		}

		foreach ( $value as $key => $child ) {
			$value[ $key ] = self::strip_presence_nonce_from_value( $child, $changed );
		}

		return $value;
	}

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
