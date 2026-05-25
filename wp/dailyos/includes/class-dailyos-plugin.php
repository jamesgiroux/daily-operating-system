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
	 * Block attributes that may contain raw runtime payloads or trust-boundary
	 * metadata and must never be persisted into Gutenberg post content.
	 */
	private const UNSAFE_BLOCK_ATTRIBUTE_KEYS = [
		'presence_nonce',
		'presenceNonce',
		'dailyos_presence_nonce',
		'dailyosPresenceNonce',
		'payload_json',
		'payloadJson',
		'dailyos_payload_json',
		'dailyosPayloadJson',
		'ability_payload',
		'abilityPayload',
		'dailyos_ability_payload',
		'dailyosAbilityPayload',
		'provenance',
		'provenance_json',
		'provenanceJson',
		'raw_provenance',
		'rawProvenance',
		'rendered_provenance',
		'renderedProvenance',
		'sensitivity',
		'sensitivity_label',
		'sensitivityLabel',
		'unknown_sensitive_shape',
		'unknownSensitiveShape',
	];

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
		add_action( 'init', [ $this, 'register_block_patterns' ], 11 );
		add_action( 'init', [ $this, 'register_post_types' ], 11 );
		add_filter( 'block_categories_all', [ $this, 'register_block_category' ], 10, 1 );
		add_action( 'init', [ $this, 'register_mcp_server_config' ], 12 );
		add_action( 'init', [ $this, 'register_save_hooks' ], 13 );
		add_filter( 'dailyos_runtime_client_for_block', [ $this, 'default_runtime_client_for_block' ], 5, 1 );
		add_action( 'admin_menu', [ $this, 'register_admin_pages' ], 10 );
		add_action( 'rest_api_init', [ $this, 'register_rest_routes' ], 10 );
		add_action( 'wp_enqueue_scripts', [ $this, 'enqueue_baseline_tokens' ], 9 );
		add_action( 'enqueue_block_editor_assets', [ $this, 'enqueue_baseline_tokens' ], 9 );

		add_action( 'dailyos_nonce_sweep', [ $this, 'sweep_presence_nonces' ] );

		// mechanism #2 of the scope-refresh design: auto-refresh stored granted_scopes against the
		// runtime's current DEFAULT_GRANTED_SCOPES without forcing a re-pair.
		add_action( 'admin_init', [ $this, 'maybe_refresh_pairing_scopes' ], 20 );

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

		// Load shared block-side helpers (envelope resolver shim per
		// L0-packet-W2 §5.1 envelopeHandle resolution contract) before any
		// block registers — render-functions.php in W2 inner blocks calls
		// dailyos_resolve_envelope() / dailyos_empty_chip() / etc.
		$shared_envelope = DAILYOS_PLUGIN_DIR . 'blocks/_shared/envelope/envelope-resolver.php';
		if ( file_exists( $shared_envelope ) ) {
			require_once $shared_envelope;
		}

		// Depth-1 globs (existing v1.4.2 + W2 outer blocks).
		$block_files = glob( DAILYOS_PLUGIN_DIR . 'blocks/*/block.json' );
		if ( false === $block_files ) {
			$block_files = [];
		}

		// Depth-2 globs for W2 entity-detail composites: each outer block has
		// a sibling inner/ directory containing one subdirectory per inner
		// block (24 for account-detail, 15 for project-detail, etc.). Inner
		// blocks register inserter-global per ADR-0129 §2 — no parent field
		// in their block.json.
		$inner_files = glob( DAILYOS_PLUGIN_DIR . 'blocks/*/inner/*/block.json' );
		if ( is_array( $inner_files ) ) {
			$block_files = array_merge( $block_files, $inner_files );
		}

		foreach ( $block_files as $block_file ) {
			register_block_type_from_metadata( dirname( $block_file ) );
		}
	}

	/**
	 * Register filesystem block patterns shipped under wp/dailyos/patterns/
	 * (W2 V1.2.1 §5.2 + wave §10 invariant "Filesystem pattern, not synced
	 * pattern"; insert-then-detach semantics — user reordering does not
	 * affect other instances).
	 */
	public function register_block_patterns(): void {
		if ( ! function_exists( 'register_block_pattern_from_file' ) && ! function_exists( 'register_block_pattern' ) ) {
			return;
		}

		$pattern_files = glob( DAILYOS_PLUGIN_DIR . 'patterns/*.php' );
		if ( false === $pattern_files || empty( $pattern_files ) ) {
			return;
		}

		foreach ( $pattern_files as $pattern_file ) {
			$headers = function_exists( 'get_file_data' )
				? get_file_data(
					$pattern_file,
					[
						'title'       => 'Title',
						'slug'        => 'Slug',
						'description' => 'Description',
						'categories'  => 'Categories',
						'blockTypes'  => 'Block Types',
						'inserter'    => 'Inserter',
					]
				)
				: [];

			$slug = isset( $headers['slug'] ) ? trim( (string) $headers['slug'] ) : '';
			if ( '' === $slug ) {
				continue;
			}

			$args = [
				'title'       => isset( $headers['title'] ) ? (string) $headers['title'] : $slug,
				'description' => isset( $headers['description'] ) ? (string) $headers['description'] : '',
				'content'     => $this->load_pattern_content( $pattern_file ),
			];

			if ( ! empty( $headers['categories'] ) ) {
				$args['categories'] = array_filter( array_map( 'trim', explode( ',', (string) $headers['categories'] ) ) );
			}
			if ( ! empty( $headers['blockTypes'] ) ) {
				$args['blockTypes'] = array_filter( array_map( 'trim', explode( ',', (string) $headers['blockTypes'] ) ) );
			}
			if ( isset( $headers['inserter'] ) && 'no' === strtolower( trim( (string) $headers['inserter'] ) ) ) {
				$args['inserter'] = false;
			}

			if ( function_exists( 'register_block_pattern' ) ) {
				register_block_pattern( $slug, $args );
			}
		}
	}

	/**
	 * Load the rendered pattern content (block markup after the closing
	 * PHP tag in the pattern file).
	 *
	 * @param string $pattern_file Absolute filesystem path.
	 * @return string Rendered pattern markup.
	 */
	private function load_pattern_content( string $pattern_file ): string {
		ob_start();
		include $pattern_file;
		$rendered = ob_get_clean();
		return is_string( $rendered ) ? trim( $rendered ) : '';
	}

	/**
	 * Register DailyOS custom post types.
	 *
	 * `dailyos_account` is the substrate-backed account post type the W3 magazine
	 * theme attaches templates to. v1.4.4 W2 adds `dailyos_project`,
	 * `dailyos_person`, `dailyos_meeting` so the W2 entity-detail outer blocks
	 * have a host post type whose template + post-meta-derived entity_id wire
	 * the runtime envelope into the magazine surface (one CPT per EntityKind
	 * per ADR-0129 §3 surface-typing). Each CPT also registers a
	 * `dailyos_entity_id` post-meta key — exposed in REST so editor UX can
	 * read/write the entity id alongside the post.
	 */
	public function register_post_types(): void {
		if ( ! function_exists( 'register_post_type' ) ) {
			return;
		}

		register_post_type(
			'dailyos_account',
			[
				'labels'        => [
					'name'          => __( 'Accounts', 'dailyos' ),
					'singular_name' => __( 'Account', 'dailyos' ),
				],
				'public'        => true,
				'has_archive'   => true,
				'rewrite'       => [ 'slug' => 'accounts' ],
				'show_in_rest'  => true,
				'rest_base'     => 'accounts',
				'supports'      => [ 'title', 'editor', 'custom-fields' ],
				'template_lock' => false,
				'menu_icon'     => 'dashicons-businessperson',
			]
		);

		register_post_type(
			'dailyos_project',
			[
				'labels'        => [
					'name'          => __( 'Projects', 'dailyos' ),
					'singular_name' => __( 'Project', 'dailyos' ),
				],
				'public'        => true,
				'has_archive'   => true,
				'rewrite'       => [ 'slug' => 'entities/projects' ],
				'show_in_rest'  => true,
				'rest_base'     => 'projects',
				'supports'      => [ 'title', 'editor', 'custom-fields' ],
				'template_lock' => false,
				'menu_icon'     => 'dashicons-portfolio',
			]
		);

		register_post_type(
			'dailyos_person',
			[
				'labels'        => [
					'name'          => __( 'People', 'dailyos' ),
					'singular_name' => __( 'Person', 'dailyos' ),
				],
				'public'        => true,
				'has_archive'   => true,
				'rewrite'       => [ 'slug' => 'entities/people' ],
				'show_in_rest'  => true,
				'rest_base'     => 'people',
				'supports'      => [ 'title', 'editor', 'custom-fields' ],
				'template_lock' => false,
				'menu_icon'     => 'dashicons-id',
			]
		);

		register_post_type(
			'dailyos_meeting',
			[
				'labels'        => [
					'name'          => __( 'Meetings', 'dailyos' ),
					'singular_name' => __( 'Meeting', 'dailyos' ),
				],
				'public'        => true,
				'has_archive'   => true,
				'rewrite'       => [ 'slug' => 'entities/meetings' ],
				'show_in_rest'  => true,
				'rest_base'     => 'meetings',
				'supports'      => [ 'title', 'editor', 'custom-fields' ],
				'template_lock' => false,
				'menu_icon'     => 'dashicons-calendar-alt',
			]
		);

		register_post_type(
			'dailyos_briefing',
			[
				'labels'        => [
					'name'          => __( 'Briefings', 'dailyos' ),
					'singular_name' => __( 'Briefing', 'dailyos' ),
				],
				'public'        => true,
				'has_archive'   => true,
				'rewrite'       => [ 'slug' => 'briefings' ],
				'show_in_rest'  => true,
				'rest_base'     => 'briefings',
				'supports'      => [ 'title', 'editor', 'custom-fields' ],
				'template_lock' => false,
				'menu_icon'     => 'dashicons-clipboard',
			]
		);

		// Register the shared dailyos_entity_id post-meta key on every entity
		// CPT (including the existing dailyos_account). Outer-block renderers
		// fall back to this meta value (then the post slug) when the block
		// attribute is empty — enables the L4 quick-setup path "create a
		// dailyos_<entity> post; the W2 surface renders against the runtime".
		if ( function_exists( 'register_post_meta' ) ) {
			foreach (
				[ 'dailyos_account', 'dailyos_project', 'dailyos_person', 'dailyos_meeting', 'dailyos_briefing' ]
				as $cpt
			) {
				register_post_meta(
					$cpt,
					'dailyos_entity_id',
					[
						'show_in_rest'  => true,
						'single'        => true,
						'type'          => 'string',
						'default'       => '',
						'auth_callback' => static function (): bool {
							return function_exists( 'current_user_can' )
								? current_user_can( 'edit_posts' )
								: false;
						},
					]
				);
			}
		}
	}

	/**
	 * Enqueue plugin-owned baseline token shim.
	 *
	 * Block CSS depends on var(--wp--preset--color--*) custom properties that come
	 * from the DailyOS theme.json. Under any non-DailyOS theme (TwentyTwentyFive
	 * etc), those vars don't resolve and trust/provenance rendering breaks. The
	 * shim defines all DailyOS preset vars on :root so block CSS works regardless
	 * of active theme.
	 *
	 * Per L0 Packet E V1.4 §5.7 + invariant #7.
	 */
	public function enqueue_baseline_tokens(): void {
		if ( ! function_exists( 'wp_enqueue_style' ) ) {
			return;
		}

		wp_enqueue_style(
			'dailyos-baseline-tokens',
			DAILYOS_PLUGIN_URL . 'assets/dailyos-baseline-tokens.css',
			[],
			DAILYOS_PLUGIN_VERSION
		);
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

		// Prefer the sentinel-discovered URL (current runtime port) over the
		// marker URL (post-pairing baseline). The runtime port changes on
		// every restart; without this fallback, session refresh hits the
		// stale marker port and the plugin reports a missing session key.
		$runtime_base_url = self::discover_runtime_base_url();
		if ( null === $runtime_base_url ) {
			$runtime_base_url = self::normalize_loopback_runtime_url( $runtime_url );
		}

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
	 *
	 * @var float
	 */
	private static float $sentinel_cached_at = 0.0;

	/**
	 * Test-only override for the runtime sentinel path.
	 *
	 * @var string|null
	 */
	private static ?string $runtime_endpoint_sentinel_path_for_tests = null;

	/**
	 * Discover the current Tauri runtime endpoint via the sentinel file.
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
		// phpcs:ignore WordPress.PHP.NoSilencedErrors.Discouraged
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
		if ( function_exists( 'posix_geteuid' ) && posix_geteuid() !== $stat['uid'] ) {
			self::log_sentinel_warning( 'sentinel ownership mismatch' );
			return null;
		}

		// phpcs:ignore WordPress.PHP.NoSilencedErrors.Discouraged,WordPress.WP.AlternativeFunctions.file_get_contents_file_get_contents
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
	 * Override runtime sentinel discovery in tests.
	 *
	 * @internal
	 *
	 * @param string|null $path Sentinel path, or null to restore HOME-based discovery.
	 */
	public static function set_runtime_endpoint_sentinel_path_for_tests( ?string $path ): void {
		self::$runtime_endpoint_sentinel_path_for_tests = $path;
		self::invalidate_runtime_endpoint_cache();
	}

	/**
	 * Path to the runtime sentinel file. Returns null if HOME is unavailable.
	 */
	private static function runtime_endpoint_sentinel_path(): ?string {
		if ( null !== self::$runtime_endpoint_sentinel_path_for_tests ) {
			return self::$runtime_endpoint_sentinel_path_for_tests;
		}

		$home = getenv( 'HOME' );
		if ( ! is_string( $home ) || '' === trim( $home ) ) {
			return null;
		}
		return rtrim( $home, '/' ) . '/.dailyos/runtime-endpoint.json';
	}

	/**
	 * Best-effort warning log for sentinel anomalies. Uses error_log to avoid
	 * depending on WP_DEBUG_LOG availability at plugin init.
	 *
	 * @param string $message Warning message body.
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
			'/nonce/verify',
			[
				'methods'             => 'POST',
				'callback'            => [ $this, 'verify_presence_nonce' ],
				'permission_callback' => [ $this, 'can_issue_presence_nonce' ],
			]
		);
	}

	/**
	 * Default provider for the dailyos_runtime_client_for_block filter.
	 *
	 * Registered at priority 5 in init() so every block's render path
	 * (render.php → render-functions.php → apply_filters) resolves to a real
	 * transport client when paired. Without this default, apply_filters returns
	 * null and every block short-circuits to is-empty regardless of runtime state.
	 *
	 * Per-render overrides at priority 10 (test fixtures and REST callers that
	 * scope a client to a single render) run after this and win — preserving
	 * the existing test seam.
	 *
	 * When unpaired, returns the existing filter value (null by default) so the
	 * renderer short-circuits to its is-empty fallback. When paired but transport
	 * is unreachable, the client's request() returns WP_Error and the renderer
	 * routes to runtime_unavailable_notice downstream.
	 *
	 * @param mixed $existing Existing filter value from prior callbacks.
	 * @return mixed Runtime client when paired and no override; $existing otherwise.
	 */
	public function default_runtime_client_for_block( mixed $existing ): mixed {
		if ( $existing instanceof DailyOS_Runtime_Client ) {
			return $existing;
		}
		$store = new DailyOS_Credential_Store();
		if ( ! $store->is_paired() ) {
			return $existing;
		}
		return new DailyOS_Runtime_Client( $store, new DailyOS_Hmac_Signer( $store ) );
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

		$response = $client->issue_nonce( $payload );
		if ( is_array( $response ) && ! isset( $response['nonce_digest'] ) && isset( $response['presence_nonce'] ) ) {
			$response['nonce_digest'] = $response['presence_nonce'];
		}
		return self::sanitize_presence_nonce_response( $response );
	}

	/**
	 * Verify and consume a user-presence nonce through the paired runtime.
	 *
	 * Consume side of the two-call feedback affordance. Body shape: { nonce_digest }.
	 * wp_user_id is server-derived from the authenticated WP session — never trusted from the request body.
	 *
	 * @param mixed $request REST request object or payload array.
	 * @return array<string, mixed>|\WP_Error Runtime verify response or validation error.
	 */
	public function verify_presence_nonce( mixed $request ): array|\WP_Error {
		// V4-W4: verify forwards the full binding tuple per the runtime's
		// VerifyNonceRequest::parse contract — compare_binding_tuple uses
		// these to validate the verify request matches the issue binding
		// before consuming the nonce. Accept presence_nonce (canonical) or
		// nonce_digest (legacy alias) at the WP boundary; forward as
		// presence_nonce which is the runtime's required key name.
		$params              = self::rest_request_params( $request );
		$presence_nonce      = self::required_string_param( $params, isset( $params['presence_nonce'] ) ? 'presence_nonce' : 'nonce_digest' );
		$claim_id            = self::required_string_param( $params, 'claim_id' );
		$action              = self::required_string_param( $params, isset( $params['action_kind'] ) ? 'action_kind' : 'action' );
		$field_path          = self::required_string_param( $params, 'field_path' );
		$composition_id      = self::required_string_param( $params, 'composition_id' );
		$claim_version       = self::required_u64_param( $params, 'claim_version', 'malformed_claim_version' );
		$composition_version = self::required_u64_param( $params, 'composition_version', 'malformed_request' );

		foreach ( [ $presence_nonce, $claim_id, $action, $field_path, $composition_id, $claim_version, $composition_version ] as $candidate ) {
			if ( is_wp_error( $candidate ) ) {
				return $candidate;
			}
		}

		$current_user_id = function_exists( 'get_current_user_id' ) ? (int) get_current_user_id() : 0;

		if ( 0 >= $current_user_id ) {
			return new \WP_Error( 'dailyos_nonce_unauthenticated', __( 'Sign in before verifying a DailyOS nonce.', 'dailyos' ), [ 'status' => 401 ] );
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
			'presence_nonce'      => $presence_nonce,
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
			$payload['feedback_request_id'] = $request_id;
		}

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );

		return self::sanitize_presence_nonce_response( $client->verify_nonce( $payload ) );
	}

	/**
	 * Strip user-authored feedback payload echoes from nonce bridge responses.
	 *
	 * @param array<string, mixed>|\WP_Error $response Runtime response.
	 * @return array<string, mixed>|\WP_Error Sanitized response.
	 */
	private static function sanitize_presence_nonce_response( array|\WP_Error $response ): array|\WP_Error {
		if ( is_array( $response ) ) {
			unset( $response['payload_json'] );
		}
		return $response;
	}

	/**
	 * Strip unsafe DailyOS runtime attributes before post content is saved.
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
	 * Strip unsafe DailyOS runtime attributes from serialized block content.
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
	 * Auto-refresh stored granted_scopes against the runtime's current
	 * DEFAULT_GRANTED_SCOPES catalog .
	 *
	 * Throttled to once per 24h via transient `dailyos_last_scope_refresh_at`.
	 * Forces a refresh when the marker's `endpoint_version` differs from
	 * the version observed at last refresh.
	 *
	 * @return void
	 */
	public function maybe_refresh_pairing_scopes(): void {
		$credential_store = new DailyOS_Credential_Store();
		$marker           = $credential_store->get_marker();
		if ( null === $marker ) {
			return;
		}

		$current_endpoint_version = isset( $marker['endpoint_version'] ) ? (string) $marker['endpoint_version'] : '';
		$throttle                 = get_transient( 'dailyos_last_scope_refresh_at' );
		$throttle_at              = is_array( $throttle ) && isset( $throttle['at'] ) ? (int) $throttle['at'] : 0;
		$throttle_version         = is_array( $throttle ) && isset( $throttle['endpoint_version'] )
			? (string) $throttle['endpoint_version']
			: '';

		$within_window           = ( time() - $throttle_at ) < DAY_IN_SECONDS;
		$endpoint_version_stable = '' !== $throttle_version && $throttle_version === $current_endpoint_version;
		if ( $within_window && $endpoint_version_stable ) {
			return;
		}

		$client   = new DailyOS_Runtime_Client( $credential_store, new DailyOS_Hmac_Signer() );
		$response = $client->refresh_pairing_scopes();

		if ( is_wp_error( $response ) ) {
			if ( defined( 'WP_DEBUG' ) && WP_DEBUG ) {
				error_log( '[dailyos] scope refresh transport error: ' . $response->get_error_code() );
			}
			return;
		}

		if ( ! is_array( $response ) || true !== ( $response['ok'] ?? false ) ) {
			if ( defined( 'WP_DEBUG' ) && WP_DEBUG ) {
				error_log( '[dailyos] scope refresh returned non-ok envelope' );
			}
			return;
		}

		$refresh = isset( $response['refresh'] ) && is_array( $response['refresh'] ) ? $response['refresh'] : [];
		$changed = isset( $refresh['changed'] ) ? (bool) $refresh['changed'] : false;
		$scopes  = isset( $refresh['granted_scopes'] ) && is_array( $refresh['granted_scopes'] )
			? array_values( array_filter( $refresh['granted_scopes'], 'is_string' ) )
			: null;

		if ( $changed && null !== $scopes ) {
			$credential_store->update_granted_scopes( $scopes );
		}

		$observed_endpoint_version = isset( $response['endpoint_version'] )
			? (string) $response['endpoint_version']
			: $current_endpoint_version;

		set_transient(
			'dailyos_last_scope_refresh_at',
			[
				'at'               => time(),
				'endpoint_version' => $observed_endpoint_version,
			],
			DAY_IN_SECONDS
		);
	}

	/**
	 * Build the runtime nonce issue payload from a REST request.
	 *
	 * @param mixed $request REST request object or payload array.
	 * @return array<string, mixed>|\WP_Error Runtime nonce payload or validation error.
	 */
	private function presence_nonce_payload( mixed $request ): array|\WP_Error {
		// V4-W4: presence_nonce_payload always routes through
		// feedback_presence_nonce_payload now that PresenceNonceAction is the
		// 9-variant FeedbackAction set. The pre-W4 4-variant legacy branch
		// (correct/dismiss/corroborate/contradict) has been removed —
		// PresenceNonceAction no longer accepts those strings, and keeping
		// the branch alive would let a request bypass the 9-variant allowlist.
		$params = self::rest_request_params( $request );
		if ( isset( $params['action'] ) && is_string( $params['action'] ) && ! isset( $params['action_kind'] ) ) {
			$params['action_kind'] = $params['action'];
		}
		return $this->feedback_presence_nonce_payload( $params );
	}

	/**
	 * Build the runtime nonce issue payload for the feedback affordance contract.
	 *
	 * @param array<string, mixed> $params Request params.
	 * @return array<string, mixed>|\WP_Error Runtime nonce payload or validation error.
	 */
	private function feedback_presence_nonce_payload( array $params ): array|\WP_Error {
		// V4-W4: every nonce mint requires the full binding tuple per the
		// runtime's IssueNonceRequest::parse contract. action_kind is the
		// canonical name the WP feedback path uses; the runtime expects
		// `action`, so we rename at the boundary.
		$claim_id            = self::required_string_param( $params, 'claim_id' );
		$action              = self::required_string_param( $params, isset( $params['action_kind'] ) ? 'action_kind' : 'action' );
		$field_path          = self::required_string_param( $params, 'field_path' );
		$composition_id      = self::required_string_param( $params, 'composition_id' );
		$claim_version       = self::required_u64_param( $params, 'claim_version', 'malformed_claim_version' );
		$composition_version = self::required_u64_param( $params, 'composition_version', 'malformed_request' );

		foreach ( [ $claim_id, $action, $field_path, $composition_id, $claim_version, $composition_version ] as $candidate ) {
			if ( is_wp_error( $candidate ) ) {
				return $candidate;
			}
		}

		if ( ! in_array(
			$action,
			[
				'confirm_current',
				'mark_outdated',
				'mark_false',
				'wrong_subject',
				'wrong_source',
				'cannot_verify',
				'needs_nuance',
				'surface_inappropriate',
				'not_relevant_here',
			],
			true
		) ) {
			return self::nonce_payload_error( 'malformed_request', 400 );
		}

		$payload_json = self::validate_payload_json( $params, $action );
		if ( is_wp_error( $payload_json ) ) {
			return $payload_json;
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

		if ( null !== $payload_json ) {
			$payload['payload_json'] = $payload_json;
		}

		$request_id = self::optional_string_param( $params, 'request_id' );

		if ( null !== $request_id ) {
			$payload['request_id'] = $request_id;
		}

		return $payload;
	}

	/**
	 * Validate the optional payload_json field per FeedbackAction variant shape.
	 *
	 * Variants that REQUIRE payload_json: wrong_source, needs_nuance, surface_inappropriate, not_relevant_here.
	 * Variants where it is OPTIONAL: wrong_subject.
	 * Variants where it MUST be absent or null: confirm_current, mark_outdated, mark_false, cannot_verify.
	 *
	 * Rejects arrays, deeply-nested objects, and non-plain-object values. Caps user-authored strings at 500 chars.
	 *
	 * @param array<string, mixed> $params Request params.
	 * @param string               $action FeedbackAction variant.
	 * @return array<string, mixed>|null|\WP_Error Validated payload (associative array — encoded as a JSON object by the runtime transport), null if absent, or WP_Error on invalid shape.
	 */
	private static function validate_payload_json( array $params, string $action ): array|null|\WP_Error {
		$raw_payload = $params['payload_json'] ?? null;
		$max_chars   = 500;

		$variants_required_payload = [ 'wrong_source', 'needs_nuance', 'surface_inappropriate', 'not_relevant_here' ];
		$variants_optional_payload = [ 'wrong_subject' ];

		if ( null === $raw_payload ) {
			if ( in_array( $action, $variants_required_payload, true ) ) {
				return self::nonce_payload_error( 'malformed_request', 400 );
			}
			return null;
		}

		if ( is_string( $raw_payload ) ) {
			$payload_json = json_decode( $raw_payload, true );
			if ( ! is_array( $payload_json ) ) {
				return self::nonce_payload_error( 'malformed_request', 400 );
			}
		} elseif ( is_array( $raw_payload ) ) {
			$payload_json = $raw_payload;
		} else {
			return self::nonce_payload_error( 'malformed_request', 400 );
		}

		if ( self::is_list_array( $payload_json ) ) {
			return self::nonce_payload_error( 'malformed_request', 400 );
		}

		if ( ! in_array( $action, array_merge( $variants_required_payload, $variants_optional_payload ), true ) ) {
			return self::nonce_payload_error( 'malformed_request', 400 );
		}

		foreach ( $payload_json as $key => $value ) {
			if ( ! is_string( $key ) ) {
				return self::nonce_payload_error( 'malformed_request', 400 );
			}
			if ( is_array( $value ) ) {
				return self::nonce_payload_error( 'malformed_request', 400 );
			}
			if ( is_string( $value ) && strlen( $value ) > $max_chars ) {
				return self::nonce_payload_error( 'malformed_request', 400 );
			}
		}

		switch ( $action ) {
			case 'wrong_source':
				// Rust record_claim_feedback requires source_ref (string) per
				// claims.rs:5185 validate_feedback_action_metadata.
				// source_index is an optional companion that helps the JS
				// affordance render the selection but the runtime never reads
				// it. The previous OR-shape would accept source_index alone
				// and fail at the runtime.
				$shape_ok = isset( $payload_json['source_ref'] )
					&& is_string( $payload_json['source_ref'] )
					&& '' !== trim( $payload_json['source_ref'] );
				break;
			case 'needs_nuance':
				$shape_ok = isset( $payload_json['corrected_text'] ) && is_string( $payload_json['corrected_text'] ) && '' !== trim( $payload_json['corrected_text'] );
				break;
			case 'surface_inappropriate':
				$shape_ok = isset( $payload_json['surface'] ) && is_string( $payload_json['surface'] ) && '' !== trim( $payload_json['surface'] );
				break;
			case 'not_relevant_here':
				$shape_ok = isset( $payload_json['invocation_id'] ) && is_string( $payload_json['invocation_id'] ) && '' !== trim( $payload_json['invocation_id'] );
				break;
			case 'wrong_subject':
				$shape_ok = true;
				break;
			default:
				$shape_ok = false;
				break;
		}

		if ( ! $shape_ok ) {
			return self::nonce_payload_error( 'malformed_request', 400 );
		}

		// Return as associative array. The runtime transport JSON-encodes the
		// full outbound body, so payload_json lands as a JSON OBJECT on the
		// wire — which is what surface_nonce::optional_payload_json requires
		// (it drops non-object values silently). Cycle-2 L2 codex challenge
		// caught the prior wp_json_encode→string forwarding bug.
		return $payload_json;
	}

	/**
	 * Portable array-list check for PHP 7.2+.
	 *
	 * @param array<mixed> $value Candidate array.
	 * @return bool
	 */
	private static function is_list_array( array $value ): bool {
		if ( [] === $value ) {
			return true;
		}
		return array_keys( $value ) === range( 0, count( $value ) - 1 );
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
	 * Strip unsafe DailyOS runtime keys from one parsed block.
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
	 * Strip unsafe DailyOS runtime keys from arbitrary block attribute values.
	 *
	 * @param mixed $value Attribute value.
	 * @param bool  $changed Change flag.
	 * @return mixed Sanitized value.
	 */
	private static function strip_presence_nonce_from_value( mixed $value, bool &$changed ): mixed {
		if ( ! is_array( $value ) ) {
			return $value;
		}

		foreach ( self::UNSAFE_BLOCK_ATTRIBUTE_KEYS as $unsafe_key ) {
			if ( array_key_exists( $unsafe_key, $value ) ) {
				unset( $value[ $unsafe_key ] );
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
