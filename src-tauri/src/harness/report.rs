use super::classifier::{RegressionClass, Severity};
use super::types::AbilityCategory;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::Path;

const UNBLOCKED_BUNDLES: &[u32] = &[1, 2, 3, 4, 6, 7, 8];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessReport {
    pub run_id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub fixtures: Vec<FixtureRunSummary>,
    pub bundle_coverage: BundleCoverage,
    #[serde(serialize_with = "serialize_sorted_string_map")]
    pub regression_class_counts: HashMap<String, u32>,
    #[serde(serialize_with = "serialize_sorted_string_map")]
    pub category_counts: HashMap<String, CategorySummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureRunSummary {
    pub fixture_dir: String,
    pub bundle: Option<u32>,
    pub scenario_id: String,
    pub category: AbilityCategory,
    pub passed: bool,
    pub continuous_score: Option<f64>,
    pub regression: Option<(RegressionClass, Severity)>,
    pub diff_count: usize,
    pub runtime_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleCoverage {
    pub bundles_run: Vec<u32>,
    pub bundles_passed: Vec<u32>,
    pub bundles_failed: Vec<u32>,
    pub bundles_unblocked: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategorySummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
}

impl HarnessReport {
    pub fn new() -> Self {
        let now = Utc::now();

        Self {
            run_id: format!("harness-{}", now.format("%Y%m%dT%H%M%S%.6fZ")),
            started_at: now,
            finished_at: now,
            fixtures: Vec::new(),
            bundle_coverage: BundleCoverage {
                bundles_unblocked: UNBLOCKED_BUNDLES.to_vec(),
                ..BundleCoverage::default()
            },
            regression_class_counts: empty_regression_class_counts(),
            category_counts: empty_category_counts(),
        }
    }

    pub fn add_fixture_summary(&mut self, summary: FixtureRunSummary) {
        self.fixtures.push(summary);
    }

    pub fn finalize(&mut self) {
        self.finished_at = Utc::now();
        self.regression_class_counts = empty_regression_class_counts();
        self.category_counts = empty_category_counts();

        let mut bundle_status: BTreeMap<u32, bool> = BTreeMap::new();
        for summary in &self.fixtures {
            let category = category_label(summary.category).to_string();
            let category_summary = self.category_counts.entry(category).or_default();
            category_summary.total += 1;
            if summary.passed {
                category_summary.passed += 1;
            } else {
                category_summary.failed += 1;
            }

            if let Some((regression_class, _)) = &summary.regression {
                let key = regression_class_label(regression_class).to_string();
                *self.regression_class_counts.entry(key).or_insert(0) += 1;
            }

            if let Some(bundle) = summary.bundle {
                bundle_status
                    .entry(bundle)
                    .and_modify(|passed| *passed &= summary.passed)
                    .or_insert(summary.passed);
            }
        }

        let bundles_run = bundle_status.keys().copied().collect::<Vec<_>>();
        let bundles_passed = bundle_status
            .iter()
            .filter_map(|(bundle, passed)| passed.then_some(*bundle))
            .collect::<Vec<_>>();
        let bundles_failed = bundle_status
            .iter()
            .filter_map(|(bundle, passed)| (!passed).then_some(*bundle))
            .collect::<Vec<_>>();

        self.bundle_coverage = BundleCoverage {
            bundles_run,
            bundles_passed,
            bundles_failed,
            bundles_unblocked: UNBLOCKED_BUNDLES.to_vec(),
        };
    }

    pub fn write_json(&self, path: &Path) -> Result<(), io::Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

impl Default for HarnessReport {
    fn default() -> Self {
        Self::new()
    }
}

fn empty_regression_class_counts() -> HashMap<String, u32> {
    [
        RegressionClass::InputChange,
        RegressionClass::PromptChange,
        RegressionClass::CanonicalizationBug,
        RegressionClass::ProviderDrift,
        RegressionClass::LogicChange,
    ]
    .into_iter()
    .map(|class| (regression_class_label(&class).to_string(), 0))
    .collect()
}

fn empty_category_counts() -> HashMap<String, CategorySummary> {
    [
        AbilityCategory::Read,
        AbilityCategory::Transform,
        AbilityCategory::Maintenance,
        AbilityCategory::Publish,
    ]
    .into_iter()
    .map(|category| {
        (
            category_label(category).to_string(),
            CategorySummary::default(),
        )
    })
    .collect()
}

fn regression_class_label(class: &RegressionClass) -> &'static str {
    match class {
        RegressionClass::InputChange => "InputChange",
        RegressionClass::PromptChange => "PromptChange",
        RegressionClass::CanonicalizationBug => "CanonicalizationBug",
        RegressionClass::ProviderDrift => "ProviderDrift",
        RegressionClass::LogicChange => "LogicChange",
    }
}

fn category_label(category: AbilityCategory) -> &'static str {
    match category {
        AbilityCategory::Read => "Read",
        AbilityCategory::Transform => "Transform",
        AbilityCategory::Maintenance => "Maintenance",
        AbilityCategory::Publish => "Publish",
    }
}

fn serialize_sorted_string_map<T, S>(
    value: &HashMap<String, T>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    value
        .iter()
        .collect::<BTreeMap<_, _>>()
        .serialize(serializer)
}
