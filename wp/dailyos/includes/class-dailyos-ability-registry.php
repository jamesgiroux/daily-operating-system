<?php
/**
 * DailyOS ability inventory reader and registrar.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS;

use DailyOS\Transport\DailyOS_Credential_Store;
use DailyOS\Transport\DailyOS_Hmac_Signer;
use DailyOS\Transport\DailyOS_Runtime_Client;

/**
 * Registers inventory-backed abilities without hardcoded ability names.
 */
final class DailyOS_Ability_Registry {
	public const INVENTORY_SCHEMA_VERSION = '1.0';

	/**
	 * Absolute path to the ability inventory.
	 *
	 * @var string
	 */
	private string $inventory_path;

	/**
	 * Constructor.
	 *
	 * @param string|null $inventory_path Absolute path to the ability inventory.
	 */
	public function __construct( ?string $inventory_path = null ) {
		$this->inventory_path = $inventory_path ?? DAILYOS_PLUGIN_DIR . 'tools/dailyos-abilities.json';
	}

	/**
	 * Load the JSON inventory. Missing or empty inventory is not an error.
	 *
	 * @return array<string, mixed>
	 */
	public function load_inventory(): array {
		if ( ! is_readable( $this->inventory_path ) ) {
			return $this->empty_inventory();
		}

		// phpcs:ignore WordPress.WP.AlternativeFunctions.file_get_contents_file_get_contents -- Local plugin inventory file.
		$contents = file_get_contents( $this->inventory_path );

		if ( false === $contents || '' === trim( $contents ) ) {
			return $this->empty_inventory();
		}

		try {
			$inventory = json_decode( $contents, true, 512, JSON_THROW_ON_ERROR );
		} catch ( \JsonException $exception ) {
			return $this->empty_inventory();
		}

		if ( ! is_array( $inventory ) || ! isset( $inventory['abilities'] ) || ! is_array( $inventory['abilities'] ) ) {
			return $this->empty_inventory();
		}

		return $inventory;
	}

	/**
	 * Register every inventory entry with the WP Abilities API when available.
	 */
	public function register_all(): int {
		$inventory = $this->load_inventory();
		$abilities = $inventory['abilities'] ?? [];

		if ( empty( $abilities ) || ! function_exists( 'wp_register_ability' ) ) {
			return 0;
		}

		$registered = 0;

		foreach ( $abilities as $ability ) {
			if ( ! is_array( $ability ) || empty( $ability['name'] ) ) {
				continue;
			}

			$ability_name = 'dailyos/' . $this->normalize_ability_name( (string) $ability['name'] );
			$result       = wp_register_ability(
				$ability_name,
				$this->build_registration_args( $ability, $ability_name )
			);

			if ( null !== $result ) {
				++$registered;
			}
		}

		return $registered;
	}

	/**
	 * Register inventory-backed ability categories with the WP Abilities API when available.
	 */
	public function register_categories(): int {
		$inventory = $this->load_inventory();
		$abilities = $inventory['abilities'] ?? [];

		if ( empty( $abilities ) || ! function_exists( 'wp_register_ability_category' ) ) {
			return 0;
		}

		$categories = [];

		foreach ( $abilities as $ability ) {
			if ( ! is_array( $ability ) ) {
				continue;
			}

			$categories[ $this->category_slug( $ability ) ] = $this->category_label( $ability );
		}

		$registered = 0;

		foreach ( $categories as $slug => $label ) {
			if ( function_exists( 'wp_has_ability_category' ) && wp_has_ability_category( $slug ) ) {
				++$registered;
				continue;
			}

			$result = wp_register_ability_category(
				$slug,
				[
					'label'       => $label,
					'description' => $this->category_description( $label ),
				]
			);

			if ( null !== $result ) {
				++$registered;
			}
		}

		return $registered;
	}

	/**
	 * Return the inventory path.
	 */
	public function get_inventory_path(): string {
		return $this->inventory_path;
	}

	/**
	 * Normalize an inventory name into a DailyOS ability slug suffix.
	 *
	 * @param string $name Inventory ability name.
	 */
	public function normalize_name( string $name ): string {
		return $this->normalize_ability_name( $name );
	}

	/**
	 * Build WP Abilities API registration arguments.
	 *
	 * @param array<string, mixed> $ability Ability descriptor.
	 * @param string               $ability_name Full DailyOS ability name.
	 * @return array<string, mixed>
	 */
	private function build_registration_args( array $ability, string $ability_name ): array {
		$mcp_exposure           = $this->normalize_mcp_exposure( $ability['mcp_exposure'] ?? 'None' );
		$client_side_executable = isset( $ability['client_side_executable'] )
			? (bool) $ability['client_side_executable']
			: false;
		$required_scopes        = self::normalize_scope_list( $ability['required_scopes'] ?? [] );
		$allowed_actors         = self::normalize_string_list( $ability['allowed_actors'] ?? [] );
		$category_label         = $this->category_label( $ability );
		$is_read_only           = $this->is_read_only_category( $category_label );

		return [
			'label'               => $this->ability_label( $ability, $ability_name ),
			'description'         => $this->ability_description( $ability, $ability_name ),
			'category'            => $this->category_slug( $ability ),
			'input_schema'        => $this->schema_arg( $ability['input_schema'] ?? [] ),
			'output_schema'       => $this->schema_arg( $ability['output_schema'] ?? [] ),
			'execute_callback'    => static function ( mixed $payload = [] ) use ( $ability_name ) {
				$scope_set = self::normalize_scope_list(
					apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] )
				);
				$payload   = is_array( $payload ) ? $payload : [ 'input' => $payload ];
				$client    = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
				$result    = $client->invoke_ability( $ability_name, $payload, $scope_set );

				if ( function_exists( 'is_wp_error' ) && is_wp_error( $result ) ) {
					return $result;
				}

				if ( self::is_runtime_unreachable_result( $result ) ) {
					return new \WP_Error(
						'dailyos_runtime_unreachable',
						__( 'DailyOS runtime is unreachable. Confirm this site is paired, restart the DailyOS runtime if needed, and retry.', 'dailyos' )
					);
				}

				return $result;
			},
			'permission_callback' => static function ( mixed $payload = [] ) use ( $required_scopes ) {
				unset( $payload );

				if ( [] === $required_scopes ) {
					return true;
				}

				$scope_set    = self::normalize_scope_list(
					apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] )
				);
				$scope_lookup = array_fill_keys( $scope_set, true );

				foreach ( $required_scopes as $required_scope ) {
					if ( ! isset( $scope_lookup[ $required_scope ] ) ) {
						return false;
					}
				}

				return true;
			},
			'meta'                => [
				'annotations'  => [
					'readonly'    => $is_read_only ? true : null,
					'destructive' => $is_read_only ? false : null,
					'idempotent'  => $is_read_only ? true : null,
				],
				'show_in_rest' => false,
				'dailyos'      => [
					'allowed_actors'         => $allowed_actors,
					'client_side_executable' => $client_side_executable,
					'inventory_category'     => $category_label,
					'mcp_exposure'           => $mcp_exposure,
					'required_scopes'        => $required_scopes,
				],
			],
		];
	}

	/**
	 * Build a human-readable ability label.
	 *
	 * @param array<string, mixed> $ability Ability descriptor.
	 * @param string               $ability_name Full DailyOS ability name.
	 */
	private function ability_label( array $ability, string $ability_name ): string {
		if ( isset( $ability['label'] ) && is_string( $ability['label'] ) && '' !== trim( $ability['label'] ) ) {
			return trim( $ability['label'] );
		}

		$name   = isset( $ability['name'] ) ? (string) $ability['name'] : $ability_name;
		$prefix = 'dailyos/';
		$suffix = str_starts_with( $name, $prefix ) ? substr( $name, strlen( $prefix ) ) : $name;
		$label  = trim( preg_replace( '/[-_]+/', ' ', $suffix ) ?? '' );

		return '' === $label ? 'DailyOS Ability' : ucwords( $label );
	}

	/**
	 * Build a non-empty ability description.
	 *
	 * @param array<string, mixed> $ability Ability descriptor.
	 * @param string               $ability_name Full DailyOS ability name.
	 */
	private function ability_description( array $ability, string $ability_name ): string {
		if ( isset( $ability['description'] ) && is_string( $ability['description'] ) && '' !== trim( $ability['description'] ) ) {
			return trim( $ability['description'] );
		}

		return sprintf(
			/* translators: %s: DailyOS ability label. */
			__( 'Runs the %s ability through DailyOS.', 'dailyos' ),
			$this->ability_label( $ability, $ability_name )
		);
	}

	/**
	 * Return the inventory category display label.
	 *
	 * @param array<string, mixed> $ability Ability descriptor.
	 */
	private function category_label( array $ability ): string {
		$label = isset( $ability['category'] ) ? trim( (string) $ability['category'] ) : '';

		return '' === $label ? 'DailyOS' : $label;
	}

	/**
	 * Return the WP Abilities API category slug for an inventory category.
	 *
	 * @param array<string, mixed> $ability Ability descriptor.
	 */
	private function category_slug( array $ability ): string {
		$slug = strtolower( preg_replace( '/[^a-zA-Z0-9]+/', '-', $this->category_label( $ability ) ) ?? '' );
		$slug = trim( $slug, '-' );

		if ( '' === $slug || 'dailyos' === $slug ) {
			return 'dailyos';
		}

		return str_starts_with( $slug, 'dailyos-' ) ? $slug : 'dailyos-' . $slug;
	}

	/**
	 * Build a WP Abilities API category description.
	 *
	 * @param string $label Category label.
	 */
	private function category_description( string $label ): string {
		if ( 'dailyos' === strtolower( $label ) ) {
			return __( 'DailyOS abilities available to connected clients.', 'dailyos' );
		}

		return sprintf(
			/* translators: %s: DailyOS ability category label. */
			__( 'DailyOS %s abilities available to connected clients.', 'dailyos' ),
			strtolower( $label )
		);
	}

	/**
	 * Determine whether a category should be annotated as read-only.
	 *
	 * @param string $label Category label.
	 */
	private function is_read_only_category( string $label ): bool {
		return in_array( strtolower( $label ), [ 'read', 'read-only', 'readonly' ], true );
	}

	/**
	 * Normalize a schema candidate for WP Abilities API registration.
	 *
	 * @param mixed $schema Schema candidate.
	 * @return array<string, mixed>
	 */
	private function schema_arg( mixed $schema ): array {
		return is_array( $schema ) ? $schema : [];
	}

	/**
	 * Preserve the MCP enum separately from browser-side execution eligibility.
	 *
	 * @param mixed $value MCP exposure value.
	 */
	private function normalize_mcp_exposure( mixed $value ): string {
		$allowed = [ 'None', 'MetadataOnly', 'Invocable' ];

		if ( is_string( $value ) && in_array( $value, $allowed, true ) ) {
			return $value;
		}

		return 'None';
	}

	/**
	 * Normalize an inventory name into a DailyOS ability slug suffix.
	 *
	 * @param string $name Inventory ability name.
	 */
	private function normalize_ability_name( string $name ): string {
		$normalized = strtolower( preg_replace( '/[^a-zA-Z0-9_-]+/', '-', $name ) ?? '' );

		return trim( $normalized, '-' );
	}

	/**
	 * Keep only unique string scopes.
	 *
	 * @param mixed $scope_set Scope candidate.
	 * @return array<int, string>
	 */
	private static function normalize_scope_list( mixed $scope_set ): array {
		return self::normalize_string_list( $scope_set );
	}

	/**
	 * Keep only unique string values.
	 *
	 * @param mixed $values String list candidate.
	 * @return array<int, string>
	 */
	private static function normalize_string_list( mixed $values ): array {
		if ( ! is_array( $values ) ) {
			return [];
		}

		$normalized = [];

		foreach ( $values as $value ) {
			if ( is_string( $value ) && '' !== $value ) {
				$normalized[] = $value;
			}
		}

		return array_values( array_unique( $normalized ) );
	}

	/**
	 * Determine whether a runtime client response means the runtime cannot be reached.
	 *
	 * @param mixed $result Runtime response.
	 */
	private static function is_runtime_unreachable_result( mixed $result ): bool {
		if ( ! is_array( $result ) ) {
			return false;
		}

		if ( true === ( $result['ok'] ?? false ) ) {
			return false;
		}

		$error = isset( $result['error'] ) && is_array( $result['error'] ) ? $result['error'] : [];
		$code  = isset( $error['code'] ) ? (string) $error['code'] : '';

		return in_array(
			$code,
			[
				'missing_session_key',
				'runtime_request_failed',
				'runtime_invalid_json',
				'runtime_http_error',
			],
			true
		);
	}

	/**
	 * Return the empty inventory shape.
	 *
	 * @return array<string, mixed>
	 */
	private function empty_inventory(): array {
		return [
			'schema_version' => self::INVENTORY_SCHEMA_VERSION,
			'abilities'      => [],
		];
	}
}
