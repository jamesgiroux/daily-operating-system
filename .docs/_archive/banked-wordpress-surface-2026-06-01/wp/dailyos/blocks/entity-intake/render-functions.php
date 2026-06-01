<?php
/**
 * Entity Intake block server-side render.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

require_once dirname( __DIR__ ) . '/_shared/dailyos-block-error-panel.php';
require_once dirname( __DIR__ ) . '/trust-band-badge/render-functions.php';

if ( ! function_exists( 'dailyos_entity_intake_render' ) ) {
	/**
	 * Render the Entity Intake block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_entity_intake_render( array $attributes ): string {
		$entity_id   = dailyos_entity_intake_attr( $attributes, 'entity_id' );
		$entity_type = dailyos_entity_intake_attr( $attributes, 'entity_type' );
		$file_ref    = dailyos_entity_intake_attr( $attributes, 'file_ref' );

		$validation = dailyos_entity_intake_validate_attrs( $entity_type, $entity_id, $file_ref );
		if ( is_wp_error( $validation ) ) {
			return dailyos_block_error_panel( 'dailyos/entity-intake', $validation->get_error_code() );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_block_error_panel( 'dailyos/entity-intake', 'RuntimeNotPaired' );
		}

		$response = $runtime_client->invoke_ability(
			'entity_intake_render',
			[
				'entityType' => $entity_type,
				'entityId'   => $entity_id,
				'fileRef'    => $file_ref,
			],
			[ 'read.entity_intelligence' ]
		);

		if ( is_wp_error( $response ) ) {
			return dailyos_block_error_panel( 'dailyos/entity-intake', dailyos_entity_intake_error_kind( $response ) );
		}
		if ( isset( $response['ok'] ) && false === $response['ok'] ) {
			return dailyos_block_error_panel( 'dailyos/entity-intake', dailyos_entity_intake_response_error_kind( $response ) );
		}

		return dailyos_entity_intake_render_claims( $response, $attributes );
	}

	/**
	 * Read and trim a string block attribute.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $key Attribute key.
	 * @return string
	 */
	function dailyos_entity_intake_attr( array $attributes, string $key ): string {
		return isset( $attributes[ $key ] ) && is_string( $attributes[ $key ] )
			? trim( $attributes[ $key ] )
			: '';
	}

	/**
	 * Validate durable block attributes before invoking the read ability.
	 *
	 * @param string $entity_type Entity type.
	 * @param string $entity_id Entity id.
	 * @param string $file_ref File reference.
	 * @return true|\WP_Error
	 */
	function dailyos_entity_intake_validate_attrs( string $entity_type, string $entity_id, string $file_ref ) {
		if ( '' === $entity_type || ! preg_match( '/^[a-z][a-z0-9_-]{0,63}$/', $entity_type ) ) {
			return new WP_Error( 'InvalidEntityType' );
		}
		if ( '' === $entity_id || ! preg_match( '/^[A-Za-z0-9:_-]{1,128}$/', $entity_id ) ) {
			return new WP_Error( 'InvalidEntityId' );
		}
		$decoded_file_ref = rawurldecode( $file_ref );
		if ( '' === $file_ref || '/' === substr( $file_ref, 0, 1 ) || false !== strpos( $file_ref, '..' ) || false !== strpos( $decoded_file_ref, '..' ) ) {
			return new WP_Error( 'PathTraversalAttempt' );
		}

		return true;
	}

	/**
	 * Render claim rows from an entity-intake ability response.
	 *
	 * @param array<string, mixed> $response Runtime response.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_entity_intake_render_claims( array $response, array $attributes ): string {
		$data   = isset( $response['data'] ) && is_array( $response['data'] ) ? $response['data'] : $response;
		$claims = isset( $data['claims'] ) && is_array( $data['claims'] ) ? $data['claims'] : [];
		$file   = isset( $attributes['file_ref'] ) ? (string) $attributes['file_ref'] : '';

		$out  = '<section class="wp-block-dailyos-entity-intake dailyos-entity-intake" data-file-ref="' . esc_attr( $file ) . '">';
		$out .= '<div class="dailyos-entity-intake__header">';
		$out .= '<span class="dailyos-entity-intake__label">' . esc_html__( 'Workspace intake', 'dailyos' ) . '</span>';
		$out .= '<span class="dailyos-entity-intake__count">' . esc_html( (string) count( $claims ) ) . '</span>';
		$out .= '</div>';

		if ( [] === $claims ) {
			$out .= '<p class="dailyos-entity-intake__empty">' . esc_html__( 'No claim proposals yet.', 'dailyos' ) . '</p>';
			$out .= '</section>';
			return $out;
		}

		$out .= '<ul class="dailyos-entity-intake__claims">';
		foreach ( $claims as $claim ) {
			if ( ! is_array( $claim ) ) {
				continue;
			}
			$text        = isset( $claim['displayText'] ) && is_string( $claim['displayText'] ) ? $claim['displayText'] : '';
			$claim_id    = isset( $claim['claimId'] ) && is_string( $claim['claimId'] ) ? $claim['claimId'] : '';
			$trust_band  = isset( $claim['trustBand'] ) && is_string( $claim['trustBand'] ) ? $claim['trustBand'] : 'needs_verification';
			$sensitivity = isset( $claim['sensitivity'] ) && is_string( $claim['sensitivity'] ) ? $claim['sensitivity'] : 'internal';
			$out        .= '<li class="dailyos-entity-intake__claim" data-claim-id="' . esc_attr( $claim_id ) . '" data-sensitivity="' . esc_attr( $sensitivity ) . '">';
			$out        .= '<span class="dailyos-entity-intake__claimText">' . esc_html( $text ) . '</span>';
			$out        .= dailyos_trust_band_badge_render_payload(
				[
					'band'    => $trust_band,
					'compact' => true,
				]
			);
			$out        .= '</li>';
		}
		$out .= '</ul></section>';
		return $out;
	}

	/**
	 * Map WP_Error codes to public block states.
	 *
	 * @param WP_Error $error Error instance.
	 * @return string
	 */
	function dailyos_entity_intake_error_kind( WP_Error $error ): string {
		$code = $error->get_error_code();
		return '' !== $code ? $code : 'IngestionFailed';
	}

	/**
	 * Map runtime envelopes to public block states.
	 *
	 * @param array<string, mixed> $response Runtime response.
	 */
	function dailyos_entity_intake_response_error_kind( array $response ): string {
		$code = isset( $response['error']['code'] ) ? (string) $response['error']['code'] : 'IngestionFailed';
		switch ( $code ) {
			case 'InvalidEntityType':
			case 'InvalidEntityId':
			case 'EntityNotFound':
			case 'InvalidCategorySlug':
			case 'CategoryNotAllowed':
			case 'FileNotFound':
			case 'PathTraversalAttempt':
				return $code;
			case 'session_requires_repair':
			case 'session_not_found':
			case 'session_expired':
			case 'scope_denied':
			case 'auth_missing':
			case 'signature_invalid':
			case 'runtime_unavailable':
				return 'RuntimeNotPaired';
			default:
				return 'IngestionFailed';
		}
	}
}
