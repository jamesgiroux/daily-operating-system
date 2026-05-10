use crate::abilities::trust::SurfaceClass;
use crate::types::ClaimSensitivity;

use super::sensitivity_aware_filtering;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn sensitivity_factor_passes_when_target_surface_is_none() {
    assert_close(
        sensitivity_aware_filtering(&ClaimSensitivity::UserOnly, None),
        1.0,
    );
}

#[test]
fn sensitivity_factor_passes_for_public_claim_on_public_surface() {
    assert_close(
        sensitivity_aware_filtering(&ClaimSensitivity::Public, Some(SurfaceClass::Public)),
        1.0,
    );
}

#[test]
fn sensitivity_factor_blocks_internal_claim_on_public_surface() {
    assert_close(
        sensitivity_aware_filtering(&ClaimSensitivity::Internal, Some(SurfaceClass::Public)),
        0.0,
    );
}

#[test]
fn sensitivity_factor_blocks_confidential_claim_on_public_surface() {
    assert_close(
        sensitivity_aware_filtering(&ClaimSensitivity::Confidential, Some(SurfaceClass::Public)),
        0.0,
    );
}

#[test]
fn sensitivity_factor_blocks_user_only_claim_on_public_surface() {
    assert_close(
        sensitivity_aware_filtering(&ClaimSensitivity::UserOnly, Some(SurfaceClass::Public)),
        0.0,
    );
}

#[test]
fn sensitivity_factor_passes_for_internal_claim_on_internal_surface() {
    assert_close(
        sensitivity_aware_filtering(&ClaimSensitivity::Internal, Some(SurfaceClass::Internal)),
        1.0,
    );
}

#[test]
fn sensitivity_factor_blocks_confidential_claim_on_internal_surface() {
    assert_close(
        sensitivity_aware_filtering(
            &ClaimSensitivity::Confidential,
            Some(SurfaceClass::Internal),
        ),
        0.0,
    );
}

#[test]
fn sensitivity_factor_passes_for_confidential_claim_on_confidential_surface() {
    assert_close(
        sensitivity_aware_filtering(
            &ClaimSensitivity::Confidential,
            Some(SurfaceClass::Confidential),
        ),
        1.0,
    );
}

#[test]
fn sensitivity_factor_blocks_user_only_claim_on_confidential_surface() {
    assert_close(
        sensitivity_aware_filtering(
            &ClaimSensitivity::UserOnly,
            Some(SurfaceClass::Confidential),
        ),
        0.0,
    );
}

#[test]
fn sensitivity_factor_passes_user_only_claim_on_user_only_surface() {
    assert_close(
        sensitivity_aware_filtering(&ClaimSensitivity::UserOnly, Some(SurfaceClass::UserOnly)),
        1.0,
    );
}
