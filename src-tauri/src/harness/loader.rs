use std::{
    fs, io,
    num::ParseIntError,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use super::types::{EvalFixture, ExpectedArtifacts, FixtureMetadata, FixtureRef};

const REQUIRED_FILES: &[&str] = &[
    "clock.txt",
    "seed.txt",
    "state.sql",
    "inputs.json",
    "provider_replay.json",
    "external_replay.json",
    "expected_output.json",
    "expected_provenance.json",
    "metadata.json",
];

const OPTIONAL_EXPECTED_STATE: &str = "expected_state.json";

/// Small library-facing loader for release-gate and integration harness callers.
///
/// The underlying fixture format stays in `tests/fixtures/bundle-{N}`; this
/// wrapper only centralizes discovery/filtering so binaries do not know the
/// directory traversal details.
#[derive(Debug, Clone)]
pub struct BundleLoader {
    roots: Vec<PathBuf>,
}

impl BundleLoader {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    pub fn default_fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    pub fn from_default_fixture_root() -> Self {
        Self::new(vec![Self::default_fixture_root()])
    }

    pub fn discover(&self) -> Result<Vec<FixtureRef>, FixtureLoadError> {
        let roots = self.roots.iter().map(PathBuf::as_path).collect::<Vec<_>>();
        discover_fixtures(&roots)
    }

    pub fn fixtures_for_bundle_names(
        &self,
        bundle_names: &[String],
    ) -> Result<Vec<FixtureRef>, FixtureLoadError> {
        let mut fixtures = self.discover()?;
        fixtures.retain(|fixture| {
            bundle_names
                .iter()
                .any(|bundle| fixture.has_label(bundle.as_str()))
        });
        Ok(fixtures)
    }
}

#[derive(Debug)]
pub enum FixtureLoadError {
    InvalidFixtureDirectory {
        path: PathBuf,
    },
    MissingRequiredFile {
        path: PathBuf,
    },
    ReadFile {
        path: PathBuf,
        source: io::Error,
    },
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    ParseClock {
        path: PathBuf,
        source: chrono::ParseError,
    },
    ParseSeed {
        path: PathBuf,
        source: ParseIntError,
    },
}

impl std::fmt::Display for FixtureLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFixtureDirectory { path } => {
                write!(f, "invalid fixture directory: {}", path.display())
            }
            Self::MissingRequiredFile { path } => {
                write!(f, "missing required fixture file: {}", path.display())
            }
            Self::ReadFile { path, source } => {
                write!(
                    f,
                    "failed to read fixture file {}: {source}",
                    path.display()
                )
            }
            Self::ParseJson { path, source } => {
                write!(
                    f,
                    "failed to parse JSON fixture file {}: {source}",
                    path.display()
                )
            }
            Self::ParseClock { path, source } => {
                write!(
                    f,
                    "failed to parse fixture clock {}: {source}",
                    path.display()
                )
            }
            Self::ParseSeed { path, source } => {
                write!(
                    f,
                    "failed to parse fixture seed {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for FixtureLoadError {}

pub fn load_fixture(fixture_dir: &Path) -> Result<EvalFixture, FixtureLoadError> {
    if !fixture_dir.is_dir() {
        return Err(FixtureLoadError::InvalidFixtureDirectory {
            path: fixture_dir.to_path_buf(),
        });
    }

    for file_name in REQUIRED_FILES {
        let path = fixture_dir.join(file_name);
        if !path.is_file() {
            return Err(FixtureLoadError::MissingRequiredFile { path });
        }
    }

    let metadata = read_json::<FixtureMetadata>(&fixture_dir.join("metadata.json"))?;
    let expected_render_policy = metadata.expected_render_policy.clone();
    let state_sql = read_to_string(&fixture_dir.join("state.sql"))?;
    let inputs_json = read_json(&fixture_dir.join("inputs.json"))?;
    let provider_replay = read_json(&fixture_dir.join("provider_replay.json"))?;
    let external_replay = read_json(&fixture_dir.join("external_replay.json"))?;
    let expected_output = read_json(&fixture_dir.join("expected_output.json"))?;
    let expected_provenance = read_json(&fixture_dir.join("expected_provenance.json"))?;
    let expected_state_path = fixture_dir.join(OPTIONAL_EXPECTED_STATE);
    let expected_state = if expected_state_path.is_file() {
        Some(read_json(&expected_state_path)?)
    } else {
        None
    };

    let clock = parse_clock(&fixture_dir.join("clock.txt"))?;
    let seed = parse_seed(&fixture_dir.join("seed.txt"))?;

    Ok(EvalFixture {
        fixture_dir: fixture_dir.to_path_buf(),
        metadata,
        state_sql,
        inputs_json,
        provider_replay,
        external_replay,
        clock,
        seed,
        expected: ExpectedArtifacts {
            output: expected_output,
            provenance: expected_provenance,
            state: expected_state,
            expected_render_policy,
        },
    })
}

pub fn discover_fixtures(roots: &[&Path]) -> Result<Vec<FixtureRef>, FixtureLoadError> {
    let mut fixtures = Vec::new();

    for root in roots {
        if !root.is_dir() {
            return Err(FixtureLoadError::InvalidFixtureDirectory {
                path: (*root).to_path_buf(),
            });
        }

        discover_bundle_dirs(root, &mut fixtures)?;
    }

    fixtures.sort_by(|left, right| left.fixture_dir.cmp(&right.fixture_dir));
    fixtures.dedup_by(|left, right| left.fixture_dir == right.fixture_dir);

    Ok(fixtures)
}

fn discover_bundle_dirs(
    dir: &Path,
    fixtures: &mut Vec<FixtureRef>,
) -> Result<(), FixtureLoadError> {
    let file_name = dir.file_name().and_then(|name| name.to_str());
    if let Some(label) = file_name.and_then(bundle_label) {
        if dir.join("metadata.json").is_file() {
            fixtures.push(FixtureRef {
                fixture_dir: dir.to_path_buf(),
                labels: vec![label],
            });
            return Ok(());
        }
    }

    for entry in read_dir(dir)? {
        let entry = entry.map_err(|source| FixtureLoadError::ReadFile {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            discover_bundle_dirs(&path, fixtures)?;
        }
    }

    Ok(())
}

fn bundle_label(file_name: &str) -> Option<String> {
    let bundle_id = file_name.strip_prefix("bundle-")?;
    (!bundle_id.is_empty() && bundle_id.chars().all(|ch| ch.is_ascii_digit()))
        .then(|| file_name.to_owned())
}

fn read_json<T>(path: &Path) -> Result<T, FixtureLoadError>
where
    T: serde::de::DeserializeOwned,
{
    let contents = read_to_string(path)?;
    serde_json::from_str(&contents).map_err(|source| FixtureLoadError::ParseJson {
        path: path.to_path_buf(),
        source,
    })
}

fn parse_clock(path: &Path) -> Result<DateTime<Utc>, FixtureLoadError> {
    let contents = read_to_string(path)?;
    DateTime::parse_from_rfc3339(contents.trim())
        .map(|clock| clock.with_timezone(&Utc))
        .map_err(|source| FixtureLoadError::ParseClock {
            path: path.to_path_buf(),
            source,
        })
}

fn parse_seed(path: &Path) -> Result<u64, FixtureLoadError> {
    let contents = read_to_string(path)?;
    contents
        .trim()
        .parse::<u64>()
        .map_err(|source| FixtureLoadError::ParseSeed {
            path: path.to_path_buf(),
            source,
        })
}

fn read_to_string(path: &Path) -> Result<String, FixtureLoadError> {
    fs::read_to_string(path).map_err(|source| FixtureLoadError::ReadFile {
        path: path.to_path_buf(),
        source,
    })
}

fn read_dir(path: &Path) -> Result<fs::ReadDir, FixtureLoadError> {
    fs::read_dir(path).map_err(|source| FixtureLoadError::ReadFile {
        path: path.to_path_buf(),
        source,
    })
}
