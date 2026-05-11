use crate::types::ClaimSensitivity;

use super::super::config::{FACTOR_MAX, FACTOR_MIN};
use super::super::types::SurfaceClass;

/// Returns the sensitivity-aware-filtering factor as a soft trust signal.
///
/// The rendering layer remains the policy enforcer. This factor only records
/// whether the claim sensitivity is compatible with the target surface class
/// so the compiler can expose the reason in confidence evidence.
pub fn sensitivity_aware_filtering(
    claim_sensitivity: &ClaimSensitivity,
    target_surface: Option<SurfaceClass>,
) -> f64 {
    match target_surface {
        None | Some(SurfaceClass::UserOnly) => FACTOR_MAX,
        Some(SurfaceClass::Confidential) => match claim_sensitivity {
            ClaimSensitivity::Public
            | ClaimSensitivity::Internal
            | ClaimSensitivity::Confidential => FACTOR_MAX,
            ClaimSensitivity::UserOnly => FACTOR_MIN,
        },
        Some(SurfaceClass::Internal) => match claim_sensitivity {
            ClaimSensitivity::Public | ClaimSensitivity::Internal => FACTOR_MAX,
            ClaimSensitivity::Confidential | ClaimSensitivity::UserOnly => FACTOR_MIN,
        },
        Some(SurfaceClass::Public) => match claim_sensitivity {
            ClaimSensitivity::Public => FACTOR_MAX,
            ClaimSensitivity::Internal
            | ClaimSensitivity::Confidential
            | ClaimSensitivity::UserOnly => FACTOR_MIN,
        },
    }
}

#[cfg(test)]
#[path = "sensitivity_test.rs"]
mod sensitivity_test;
