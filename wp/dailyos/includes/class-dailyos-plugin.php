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
		( new DailyOS_Credential_Store() )->register_session_key_filter_safeguard();
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

		$registry = new DailyOS_Ability_Registry();
		$resolver = static function (): array {
			return apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		};

		DailyOS_Mcp_Server::bootstrap( $registry, $resolver );
	}
}
