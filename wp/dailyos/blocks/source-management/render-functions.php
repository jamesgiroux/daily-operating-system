<?php
/**
 * Source Management block server-side render helpers.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_source_management_render' ) ) {
	if ( function_exists( 'add_action' ) ) {
		add_action(
			'rest_api_init',
			static function (): void {
				dailyos_source_management_register_routes();
			}
		);
	}

	/**
	 * Register source-management action route.
	 */
	function dailyos_source_management_register_routes(): void {
		if ( ! function_exists( 'register_rest_route' ) ) {
			return;
		}
		register_rest_route(
			'dailyos/v1',
			'/source-management/action',
			[
				'methods'             => 'POST',
				'callback'            => 'dailyos_source_management_handle_action',
				'permission_callback' => 'dailyos_source_management_can_mutate',
			]
		);
	}

	/**
	 * Gate source mutations to editors.
	 *
	 * @return bool
	 */
	function dailyos_source_management_can_mutate(): bool {
		return function_exists( 'current_user_can' ) && current_user_can( 'edit_posts' );
	}

	/**
	 * Handle a source-management action request.
	 *
	 * @param mixed $request REST request.
	 * @return mixed
	 */
	function dailyos_source_management_handle_action( mixed $request ): mixed {
		$params    = is_object( $request ) && is_callable( [ $request, 'get_json_params' ] )
			? $request->get_json_params()
			: [];
		$validated = dailyos_source_management_validate_action_payload( is_array( $params ) ? $params : [] );
		if ( is_wp_error( $validated ) ) {
			return $validated;
		}
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! is_callable( [ $runtime_client, 'apply_source_management_action' ] ) ) {
			return new WP_Error( 'dailyos_runtime_unavailable', __( 'Sources are temporarily unavailable.', 'dailyos' ), [ 'status' => 503 ] );
		}
		$response = $runtime_client->apply_source_management_action(
			$validated['action'],
			$validated['entity_type'],
			$validated['entity_id'],
			$validated['source_key']
		);
		if ( is_wp_error( $response ) ) {
			return $response;
		}
		return function_exists( 'rest_ensure_response' ) ? rest_ensure_response( $response ) : $response;
	}

	/**
	 * Validate a source action REST payload.
	 *
	 * @param array<string, mixed> $params Request params.
	 * @return array{action:string,entity_type:string,entity_id:string,source_key:string}|\WP_Error
	 */
	function dailyos_source_management_validate_action_payload( array $params ): array|\WP_Error {
		$action      = isset( $params['action'] ) && is_scalar( $params['action'] ) ? trim( (string) $params['action'] ) : '';
		$entity_type = isset( $params['entityType'] ) && is_scalar( $params['entityType'] ) ? strtolower( trim( (string) $params['entityType'] ) ) : '';
		$entity_id   = isset( $params['entityId'] ) && is_scalar( $params['entityId'] ) ? trim( (string) $params['entityId'] ) : '';
		$source_key  = isset( $params['sourceKey'] ) && is_scalar( $params['sourceKey'] ) ? trim( (string) $params['sourceKey'] ) : '';
		if ( ! in_array( $action, [ 'reingest', 'quarantine', 'relink', 'ignore', 'scratchpad', 'archive', 'delete' ], true ) ) {
			return new WP_Error( 'dailyos_invalid_source_action', __( 'Source action is unavailable.', 'dailyos' ), [ 'status' => 400 ] );
		}
		if ( ! dailyos_source_management_is_entity_type( $entity_type ) || ! dailyos_source_management_is_entity_id( $entity_id ) || ! dailyos_source_management_is_source_key( $source_key ) ) {
			return new WP_Error( 'dailyos_invalid_source_target', __( 'Source action is unavailable.', 'dailyos' ), [ 'status' => 400 ] );
		}
		return [
			'action'      => $action,
			'entity_type' => $entity_type,
			'entity_id'   => $entity_id,
			'source_key'  => $source_key,
		];
	}

	/**
	 * Render the Source Management block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content Inner content.
	 * @param \WP_Block|null       $block Parsed block.
	 * @return string
	 */
	function dailyos_source_management_render( array $attributes, string $content = '', $block = null ): string {
		unset( $content, $block );

		$validated = dailyos_source_management_validate_attrs( $attributes );
		if ( null === $validated ) {
			return dailyos_source_management_render_state(
				'not_ready',
				__( 'Sources appear when an entity is selected.', 'dailyos' )
			);
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( is_object( $runtime_client ) && is_callable( [ $runtime_client, 'read_source_management_ledger' ] ) ) {
			$response = $runtime_client->read_source_management_ledger(
				$validated['entity_type'],
				$validated['entity_id'],
				$validated['page_size']
			);
			return dailyos_source_management_render_from_response( $response );
		}

		return dailyos_source_management_render_state(
			'unavailable',
			__( 'Sources are temporarily unavailable.', 'dailyos' )
		);
	}

	/**
	 * Validate entity-scoped block attributes.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return array{entity_type:string,entity_id:string,page_size:int}|null
	 */
	function dailyos_source_management_validate_attrs( array $attributes ): ?array {
		$entity_type = isset( $attributes['entity_type'] ) && is_scalar( $attributes['entity_type'] )
			? strtolower( trim( (string) $attributes['entity_type'] ) )
			: '';
		$entity_id   = isset( $attributes['entity_id'] ) && is_scalar( $attributes['entity_id'] )
			? trim( (string) $attributes['entity_id'] )
			: '';
		if ( ! in_array( $entity_type, [ 'account', 'person', 'project' ], true ) ) {
			return null;
		}
		if ( 1 !== preg_match( '/^[A-Za-z0-9:_-]{1,160}$/', $entity_id ) ) {
			return null;
		}
		$page_size = isset( $attributes['page_size'] ) && is_numeric( $attributes['page_size'] )
			? (int) $attributes['page_size']
			: 25;
		return [
			'entity_type' => $entity_type,
			'entity_id'   => $entity_id,
			'page_size'   => max( 1, min( 100, $page_size ) ),
		];
	}

	/**
	 * Render a runtime response.
	 *
	 * @param mixed $response Runtime response.
	 * @return string
	 */
	function dailyos_source_management_render_from_response( mixed $response ): string {
		if ( is_wp_error( $response ) || ! is_array( $response ) ) {
			return dailyos_source_management_render_state(
				'unavailable',
				__( 'Sources are temporarily unavailable.', 'dailyos' )
			);
		}
		if ( isset( $response['error'] ) ) {
			return dailyos_source_management_render_state(
				'unavailable',
				__( 'Sources are temporarily unavailable.', 'dailyos' )
			);
		}
		$payload = isset( $response['data'] ) && is_array( $response['data'] ) ? $response['data'] : $response;
		return dailyos_source_management_render_payload( $payload );
	}

	/**
	 * Render the safe source-management payload.
	 *
	 * @param array<string, mixed> $payload Runtime payload.
	 * @return string
	 */
	function dailyos_source_management_render_payload( array $payload ): string {
		$sources = isset( $payload['sources'] ) && is_array( $payload['sources'] ) ? $payload['sources'] : [];
		$out     = '<section ' . dailyos_source_management_wrapper_attrs( empty( $sources ) ? 'empty' : 'ready' ) . ' aria-labelledby="dailyos-source-management-heading">';
		$out    .= '<div class="dailyos-source-management__header">';
		$out    .= '<div>';
		$out    .= '<p class="dailyos-source-management__eyebrow">' . esc_html__( 'Workspace memory', 'dailyos' ) . '</p>';
		$out    .= '<h2 id="dailyos-source-management-heading" class="dailyos-source-management__title">' . esc_html__( 'Sources', 'dailyos' ) . '</h2>';
		$out    .= '</div>';
		$out    .= dailyos_source_management_action_policy( $payload );
		$out    .= '</div>';

		if ( empty( $sources ) ) {
			$out .= '<p class="dailyos-source-management__empty">' . esc_html__( 'No workspace sources are linked yet.', 'dailyos' ) . '</p>';
			$out .= '</section>';
			return $out;
		}

		$out .= '<ul class="dailyos-source-management__list">';
		foreach ( $sources as $source ) {
			if ( is_array( $source ) ) {
				$out .= dailyos_source_management_render_source_row( $source );
			}
		}
		$out .= '</ul>';
		$out .= '</section>';
		return $out;
	}

	/**
	 * Render one source row without leaking opaque handles.
	 *
	 * @param array<string, mixed> $source Source payload.
	 * @return string
	 */
	function dailyos_source_management_render_source_row( array $source ): string {
		$kind      = dailyos_source_management_kind_label( dailyos_source_management_first_string( $source, [ 'sourceKind', 'source_kind' ], 'workspace_source' ) );
		$category  = dailyos_source_management_safe_text( dailyos_source_management_first_string( $source, [ 'category' ], '' ), '', 40 );
		$lifecycle = dailyos_source_management_lifecycle_label( dailyos_source_management_first_string( $source, [ 'lifecycleState', 'lifecycle_state' ], 'pending' ) );
		$date      = dailyos_source_management_source_date( $source );
			$run   = isset( $source['latestRun'] ) && is_array( $source['latestRun'] )
				? $source['latestRun']
				: ( isset( $source['latest_run'] ) && is_array( $source['latest_run'] ) ? $source['latest_run'] : [] );
			$runs  = isset( $source['ingestionRuns'] ) && is_array( $source['ingestionRuns'] )
				? $source['ingestionRuns']
				: ( isset( $source['ingestion_runs'] ) && is_array( $source['ingestion_runs'] ) ? $source['ingestion_runs'] : [] );
			$trust = isset( $source['trustBandSummary'] ) && is_array( $source['trustBandSummary'] )
				? $source['trustBandSummary']
				: ( isset( $source['trust_band_summary'] ) && is_array( $source['trust_band_summary'] ) ? $source['trust_band_summary'] : [] );
		$actions   = isset( $source['actions'] ) && is_array( $source['actions'] ) ? $source['actions'] : [];

		$out  = '<li class="dailyos-source-management__item">';
		$out .= '<div class="dailyos-source-management__main">';
		$out .= '<p class="dailyos-source-management__sourceKind">' . esc_html( $kind ) . '</p>';
		$out .= '<dl class="dailyos-source-management__meta">';
		$out .= dailyos_source_management_meta_item( __( 'State', 'dailyos' ), $lifecycle );
		if ( '' !== $category ) {
			$out .= dailyos_source_management_meta_item( __( 'Category', 'dailyos' ), $category );
		}
		$out     .= dailyos_source_management_meta_item( __( 'Date', 'dailyos' ), $date );
			$out .= dailyos_source_management_meta_item( __( 'Run', 'dailyos' ), dailyos_source_management_run_label( $run ) );
			$out .= '</dl>';
			$out .= dailyos_source_management_run_history( $runs );
			$out .= dailyos_source_management_trust_summary( $trust );
			$out .= '</div>';
			$out .= dailyos_source_management_actions( $source, $actions );
		$out     .= '</li>';

		return $out;
	}

	/**
	 * Render a whole-block state.
	 *
	 * @param string $state State key.
	 * @param string $message Public message.
	 * @return string
	 */
	function dailyos_source_management_render_state( string $state, string $message ): string {
		$out  = '<section ' . dailyos_source_management_wrapper_attrs( $state ) . ' aria-labelledby="dailyos-source-management-heading">';
		$out .= '<div class="dailyos-source-management__header">';
		$out .= '<div>';
		$out .= '<p class="dailyos-source-management__eyebrow">' . esc_html__( 'Workspace memory', 'dailyos' ) . '</p>';
		$out .= '<h2 id="dailyos-source-management-heading" class="dailyos-source-management__title">' . esc_html__( 'Sources', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '</div>';
		$out .= '<p class="dailyos-source-management__empty">' . esc_html( $message ) . '</p>';
		$out .= '</section>';
		return $out;
	}

	/**
	 * Wrapper attributes.
	 *
	 * @param string $state Public state key.
	 * @return string
	 */
	function dailyos_source_management_wrapper_attrs( string $state ): string {
		return sprintf(
			'class="%s" data-dailyos-surface="source-management" data-dailyos-state="%s"',
			esc_attr( 'wp-block-dailyos-source-management dailyos-source-management' ),
			esc_attr( dailyos_source_management_safe_key( $state, 'unavailable' ) )
		);
	}

	/**
	 * Render action-policy status.
	 *
	 * @param array<string, mixed> $payload Runtime payload.
	 * @return string
	 */
	function dailyos_source_management_action_policy( array $payload ): string {
		$policy     = isset( $payload['actionPolicy'] ) && is_array( $payload['actionPolicy'] )
			? $payload['actionPolicy']
			: ( isset( $payload['action_policy'] ) && is_array( $payload['action_policy'] ) ? $payload['action_policy'] : [] );
			$reason = dailyos_source_management_first_string( $policy, [ 'disabledReason', 'disabled_reason' ], 'read_only' );
		if ( '' === $reason ) {
			return '';
		}
			return '<p class="dailyos-source-management__policy">' . esc_html( dailyos_source_management_policy_label( $reason ) ) . '</p>';
	}

		/**
		 * Render row actions.
		 *
		 * @param array<string, mixed> $source Source payload.
		 * @param array<string, mixed> $actions Action payload.
		 * @return string
		 */
	function dailyos_source_management_actions( array $source, array $actions ): string {
		$source_key      = dailyos_source_management_first_string( $source, [ 'sourceKey', 'source_key' ], '' );
		$entity          = isset( $source['entity'] ) && is_array( $source['entity'] ) ? $source['entity'] : [];
		$entity_type     = dailyos_source_management_first_string( $entity, [ 'entityType', 'entity_type' ], '' );
		$entity_id       = dailyos_source_management_first_string( $entity, [ 'entityId', 'entity_id' ], '' );
		$has_target      = dailyos_source_management_is_source_key( $source_key )
			&& dailyos_source_management_is_entity_type( $entity_type )
			&& dailyos_source_management_is_entity_id( $entity_id );
		$disabled_reason = dailyos_source_management_policy_label(
			dailyos_source_management_first_string( $actions, [ 'disabledReason', 'disabled_reason' ], '' )
		);
		$out             = '<div class="dailyos-source-management__actions" aria-label="' . esc_attr( __( 'Source actions', 'dailyos' ) ) . '">';
		$out            .= dailyos_source_management_action_button( __( 'Re-ingest', 'dailyos' ), 'reingest', $has_target && dailyos_source_management_bool_value( $actions, [ 'canReingest', 'can_reingest' ] ), $source_key, $entity_type, $entity_id, $disabled_reason );
		$out            .= dailyos_source_management_action_button( __( 'Quarantine', 'dailyos' ), 'quarantine', $has_target && dailyos_source_management_bool_value( $actions, [ 'canQuarantine', 'can_quarantine' ] ), $source_key, $entity_type, $entity_id, $disabled_reason );
		$out            .= dailyos_source_management_action_button( __( 'Re-link', 'dailyos' ), 'relink', $has_target && dailyos_source_management_bool_value( $actions, [ 'canRelink', 'can_relink' ] ), $source_key, $entity_type, $entity_id, $disabled_reason );
		$out            .= dailyos_source_management_action_button( __( 'Ignore', 'dailyos' ), 'ignore', $has_target && dailyos_source_management_bool_value( $actions, [ 'canIgnore', 'can_ignore' ] ), $source_key, $entity_type, $entity_id, $disabled_reason );
		$out            .= dailyos_source_management_action_button( __( 'Scratchpad', 'dailyos' ), 'scratchpad', $has_target && dailyos_source_management_bool_value( $actions, [ 'canScratchpad', 'can_scratchpad' ] ), $source_key, $entity_type, $entity_id, $disabled_reason );
		$out            .= dailyos_source_management_action_button( __( 'Archive', 'dailyos' ), 'archive', $has_target && dailyos_source_management_bool_value( $actions, [ 'canArchive', 'can_archive' ] ), $source_key, $entity_type, $entity_id, $disabled_reason );
		$out            .= dailyos_source_management_action_button( __( 'Delete', 'dailyos' ), 'delete', $has_target && dailyos_source_management_bool_value( $actions, [ 'canDelete', 'can_delete' ] ), $source_key, $entity_type, $entity_id, $disabled_reason );
		$out            .= '</div>';
		return $out;
	}

		/**
		 * Render an action affordance.
		 *
		 * @param string $label Button label.
		 * @param string $action Action key.
		 * @param bool   $enabled Whether the action can be invoked.
		 * @param string $source_key Opaque source key.
		 * @param string $entity_type Entity type.
		 * @param string $entity_id Entity identifier.
		 * @param string $reason Disabled reason.
		 * @return string
		 */
	function dailyos_source_management_action_button( string $label, string $action, bool $enabled, string $source_key, string $entity_type, string $entity_id, string $reason ): string {
		$attrs = [
			'type'                       => 'button',
			'class'                      => 'dailyos-source-management__action',
			'data-dailyos-source-action' => $action,
			'data-dailyos-source-key'    => $source_key,
			'data-dailyos-entity-type'   => $entity_type,
			'data-dailyos-entity-id'     => $entity_id,
		];
		if ( ! $enabled ) {
			$attrs['disabled']      = 'disabled';
			$attrs['aria-disabled'] = 'true';
			$attrs['title']         = '' === $reason ? __( 'Unavailable', 'dailyos' ) : $reason;
		}
		$out = '<button';
		foreach ( $attrs as $name => $value ) {
			$out .= ' ' . esc_attr( $name ) . '="' . esc_attr( $value ) . '"';
		}
		return $out . '>' . esc_html( $label ) . '</button>';
	}

	/**
	 * Render trust distribution.
	 *
	 * @param array<string, mixed> $trust Trust summary.
	 * @return string
	 */
	function dailyos_source_management_trust_summary( array $trust ): string {
		$total = dailyos_source_management_int_value( $trust, [ 'total' ] );
		if ( 0 === $total ) {
			return '<p class="dailyos-source-management__trust">' . esc_html__( 'No derived claims', 'dailyos' ) . '</p>';
		}
		$likely   = dailyos_source_management_int_value( $trust, [ 'likelyCurrent', 'likely_current' ] );
		$caution  = dailyos_source_management_int_value( $trust, [ 'useWithCaution', 'use_with_caution' ] );
		$verify   = dailyos_source_management_int_value( $trust, [ 'needsVerification', 'needs_verification' ] );
		$unscored = dailyos_source_management_int_value( $trust, [ 'unscored' ] );
		$parts    = [
			sprintf(
				/* translators: %d: number of likely-current claims. */
				_n( '%d likely current', '%d likely current', $likely, 'dailyos' ),
				$likely
			),
			sprintf(
				/* translators: %d: number of use-with-caution claims. */
				_n( '%d use with caution', '%d use with caution', $caution, 'dailyos' ),
				$caution
			),
			sprintf(
				/* translators: %d: number of needs-verification claims. */
				_n( '%d needs verification', '%d needs verification', $verify, 'dailyos' ),
				$verify
			),
		];
		if ( $unscored > 0 ) {
			$parts[] = sprintf(
				/* translators: %d: number of unscored claims. */
				_n( '%d unscored', '%d unscored', $unscored, 'dailyos' ),
				$unscored
			);
		}
		return '<p class="dailyos-source-management__trust">' . esc_html( implode( ', ', $parts ) ) . '</p>';
	}

	/**
	 * Render one metadata item.
	 *
	 * @param string $term Metadata term.
	 * @param string $description Metadata value.
	 * @return string
	 */
	function dailyos_source_management_meta_item( string $term, string $description ): string {
		return '<div class="dailyos-source-management__metaItem">'
			. '<dt>' . esc_html( $term ) . '</dt>'
			. '<dd>' . esc_html( $description ) . '</dd>'
			. '</div>';
	}

	/**
	 * Safe source kind display.
	 *
	 * @param string $kind Raw kind.
	 * @return string
	 */
	function dailyos_source_management_kind_label( string $kind ): string {
		$key    = dailyos_source_management_safe_key( $kind, 'workspace_source' );
		$labels = [
			'drive_sync'         => __( 'Drive file', 'dailyos' ),
			'entity_doc'         => __( 'Entity document', 'dailyos' ),
			'granola_transcript' => __( 'Transcript', 'dailyos' ),
			'inbox'              => __( 'Inbox item', 'dailyos' ),
			'mcp_placement'      => __( 'Workspace document', 'dailyos' ),
			'quill_transcript'   => __( 'Transcript', 'dailyos' ),
			'user_attachment'    => __( 'Attachment', 'dailyos' ),
			'workspace_source'   => __( 'Workspace source', 'dailyos' ),
		];
		return $labels[ $key ] ?? __( 'Workspace source', 'dailyos' );
	}

	/**
	 * Safe lifecycle display.
	 *
	 * @param string $state Raw state.
	 * @return string
	 */
	function dailyos_source_management_lifecycle_label( string $state ): string {
		$key    = dailyos_source_management_safe_key( $state, 'pending' );
		$labels = [
			'ingested'                  => __( 'Active', 'dailyos' ),
			'ingesting'                 => __( 'Ingesting', 'dailyos' ),
			'pending'                   => __( 'Pending', 'dailyos' ),
			'pending_entity_assignment' => __( 'Needs entity', 'dailyos' ),
			'quarantined'               => __( 'Needs review', 'dailyos' ),
			'rejected'                  => __( 'Rejected', 'dailyos' ),
			'superseded'                => __( 'Superseded', 'dailyos' ),
			'ignored'                   => __( 'Ignored', 'dailyos' ),
			'scratchpad'                => __( 'Scratchpad', 'dailyos' ),
			'archived'                  => __( 'Archived', 'dailyos' ),
			'deleted'                   => __( 'Deleted', 'dailyos' ),
		];
		return $labels[ $key ] ?? __( 'Pending', 'dailyos' );
	}

	/**
	 * Safe policy display.
	 *
	 * @param string $reason Raw reason.
	 * @return string
	 */
	function dailyos_source_management_policy_label( string $reason ): string {
		$key = dailyos_source_management_safe_key( $reason, 'read_only' );
		if ( 'already_quarantined' === $key ) {
			return __( 'Already quarantined', 'dailyos' );
		}
		if ( 'write_actions_deferred' === $key ) {
			return __( 'Read-only until source actions land', 'dailyos' );
		}
		return __( 'Read-only', 'dailyos' );
	}

	/**
	 * Render latest run summary.
	 *
	 * @param array<string, mixed> $run Latest run.
	 * @return string
	 */
	function dailyos_source_management_run_label( array $run ): string {
		if ( empty( $run ) ) {
			return __( 'No runs', 'dailyos' );
		}
		$status = dailyos_source_management_safe_text(
			dailyos_source_management_first_string( $run, [ 'status' ], 'unknown' ),
			__( 'Unknown', 'dailyos' ),
			32
		);
		$count  = dailyos_source_management_int_value( $run, [ 'claimCountProduced', 'claim_count_produced' ] );
		return sprintf(
		/* translators: 1: run status, 2: claim count */
			__( '%1$s, %2$d claims', 'dailyos' ),
			ucfirst( $status ),
			$count
		);
	}

		/**
		 * Render bounded ingestion run history.
		 *
		 * @param array<int, mixed> $runs Ingestion run payloads.
		 * @return string
		 */
	function dailyos_source_management_run_history( array $runs ): string {
		$items = [];
		foreach ( $runs as $run ) {
			if ( ! is_array( $run ) ) {
				continue;
			}
			$items[] = dailyos_source_management_run_label( $run );
			if ( count( $items ) >= 5 ) {
				break;
			}
		}
		if ( empty( $items ) ) {
			return '';
		}
		$out = '<ol class="dailyos-source-management__runs" aria-label="' . esc_attr( __( 'Ingestion run history', 'dailyos' ) ) . '">';
		foreach ( $items as $item ) {
			$out .= '<li>' . esc_html( $item ) . '</li>';
		}
		$out .= '</ol>';
		return $out;
	}

		/**
		 * Render source date.
		 *
		 * @param array<string, mixed> $source Source payload.
		 * @return string
		 */
	function dailyos_source_management_source_date( array $source ): string {
		$raw = dailyos_source_management_first_string( $source, [ 'sourceAsof', 'source_asof' ], '' );
		if ( '' === $raw ) {
			return __( 'Unknown', 'dailyos' );
		}
		$timestamp = strtotime( $raw );
		if ( false === $timestamp ) {
			return __( 'Unknown', 'dailyos' );
		}
		if ( function_exists( 'wp_date' ) ) {
			return wp_date( 'M j, Y', $timestamp );
		}
		return gmdate( 'M j, Y', $timestamp );
	}

	/**
	 * Pick the first scalar string value.
	 *
	 * @param array<string, mixed> $payload Payload.
	 * @param array<int, string>   $keys Candidate keys.
	 * @param string               $fallback Fallback.
	 * @return string
	 */
	function dailyos_source_management_first_string( array $payload, array $keys, string $fallback = '' ): string {
		foreach ( $keys as $key ) {
			if ( isset( $payload[ $key ] ) && is_scalar( $payload[ $key ] ) && '' !== trim( (string) $payload[ $key ] ) ) {
				return trim( (string) $payload[ $key ] );
			}
		}
		return $fallback;
	}

	/**
	 * Pick the first integer value.
	 *
	 * @param array<string, mixed> $payload Payload.
	 * @param array<int, string>   $keys Candidate keys.
	 * @return int
	 */
	function dailyos_source_management_int_value( array $payload, array $keys ): int {
		foreach ( $keys as $key ) {
			if ( isset( $payload[ $key ] ) && is_numeric( $payload[ $key ] ) ) {
				return max( 0, (int) $payload[ $key ] );
			}
		}
		return 0;
	}

		/**
		 * Pick the first boolean value.
		 *
		 * @param array<string, mixed> $payload Payload.
		 * @param array<int, string>   $keys Candidate keys.
		 * @return bool
		 */
	function dailyos_source_management_bool_value( array $payload, array $keys ): bool {
		foreach ( $keys as $key ) {
			if ( isset( $payload[ $key ] ) ) {
				return true === $payload[ $key ] || 1 === $payload[ $key ] || '1' === $payload[ $key ];
			}
		}
		return false;
	}

		/**
		 * Validate an opaque source action key.
		 *
		 * @param string $value Candidate key.
		 * @return bool
		 */
	function dailyos_source_management_is_source_key( string $value ): bool {
		return 1 === preg_match( '/^source:v1:[A-Za-z0-9_-]{16,160}$/', $value );
	}

		/**
		 * Validate an entity type.
		 *
		 * @param string $value Candidate entity type.
		 * @return bool
		 */
	function dailyos_source_management_is_entity_type( string $value ): bool {
		return in_array( $value, [ 'account', 'person', 'project' ], true );
	}

		/**
		 * Validate an entity id.
		 *
		 * @param string $value Candidate entity id.
		 * @return bool
		 */
	function dailyos_source_management_is_entity_id( string $value ): bool {
		return 1 === preg_match( '/^[A-Za-z0-9:_-]{1,160}$/', $value );
	}

		/**
		 * Sanitize display text and reject path-like values.
		 *
		 * @param mixed  $value Candidate value.
		 * @param string $fallback Fallback.
		 * @param int    $max_length Maximum output length.
		 * @return string
		 */
	function dailyos_source_management_safe_text( mixed $value, string $fallback, int $max_length = 120 ): string {
		if ( ! is_scalar( $value ) ) {
			return $fallback;
		}
		$text = preg_replace( '/\s+/', ' ', trim( wp_strip_all_tags( (string) $value ) ) ) ?? '';
		if ( '' === $text || str_contains( $text, '/' ) || str_contains( $text, '\\' ) ) {
			return $fallback;
		}
		if ( strlen( $text ) > $max_length ) {
			return rtrim( substr( $text, 0, $max_length - 1 ) ) . '...';
		}
		return $text;
	}

	/**
	 * Normalize a safe key.
	 *
	 * @param string $value Candidate key.
	 * @param string $fallback Fallback.
	 * @return string
	 */
	function dailyos_source_management_safe_key( string $value, string $fallback ): string {
		$key = strtolower( trim( $value ) );
		if ( 1 === preg_match( '/^[a-z0-9_-]{1,64}$/', $key ) ) {
			return $key;
		}
		return $fallback;
	}
}
