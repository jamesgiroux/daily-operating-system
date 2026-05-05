use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

fn lint_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn fixture_root() -> PathBuf {
    repo_root().join("src-tauri/tests/fixtures")
}

fn lint_script() -> PathBuf {
    repo_root().join("src-tauri/scripts/check_fixture_anonymization.sh")
}

fn run_lint() -> Output {
    let script = lint_script();
    assert!(
        script.is_file(),
        "missing DOS-216 fixture anonymization lint script: {}",
        script.display()
    );

    Command::new("bash")
        .arg(script)
        .current_dir(repo_root())
        .output()
        .expect("run DOS-216 fixture anonymization lint")
}

fn output_text(output: &Output) -> String {
    format!(
        "--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn cleanup_stale_temp_lint_dirs() {
    let root = fixture_root();
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if name.starts_with("_temp_lint_test_") {
            let _ = fs::remove_dir_all(path);
        }
    }
}

struct TempFixtureDir {
    path: PathBuf,
}

impl TempFixtureDir {
    fn new(test_name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = fixture_root().join(format!(
            "_temp_lint_test_{}_{}_{}",
            test_name,
            std::process::id(),
            now
        ));
        fs::create_dir_all(&path).expect("create temp fixture dir");
        Self { path }
    }

    fn write(&self, file_name: &str, contents: &str) {
        fs::write(self.path.join(file_name), contents).expect("write synthetic fixture");
    }
}

impl Drop for TempFixtureDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn lint_passes_on_current_committed_fixtures() {
    let _guard = lint_lock().lock().expect("lock fixture lint test");
    cleanup_stale_temp_lint_dirs();

    let output = run_lint();
    assert!(
        output.status.success(),
        "lint must pass on committed fixtures\n{}",
        output_text(&output)
    );
}

#[test]
fn lint_blocks_non_example_email_in_synthesized_fixture() {
    let _guard = lint_lock().lock().expect("lock fixture lint test");
    cleanup_stale_temp_lint_dirs();
    let temp = TempFixtureDir::new("email");
    temp.write("inputs.json", r#"{"owner_email":"test@gmail.com"}"#);

    let output = run_lint();
    let text = output_text(&output);
    assert!(
        !output.status.success(),
        "lint must reject non-example.com email\n{text}"
    );
    assert!(
        text.contains("non-example-email") && text.contains("_temp_lint_test_email"),
        "lint output must identify the email violation\n{text}"
    );
}

#[test]
fn lint_blocks_redacted_literal_in_synthesized_fixture() {
    let _guard = lint_lock().lock().expect("lock fixture lint test");
    cleanup_stale_temp_lint_dirs();
    let temp = TempFixtureDir::new("redacted");
    temp.write("expected_output.json", r#"{"summary":"REDACTED"}"#);

    let output = run_lint();
    let text = output_text(&output);
    assert!(
        !output.status.success(),
        "lint must reject REDACTED scrub artifacts\n{text}"
    );
    assert!(
        text.contains("redacted-literal") && text.contains("_temp_lint_test_redacted"),
        "lint output must identify the REDACTED violation\n{text}"
    );
}

#[test]
fn lint_allows_lint_allow_escape_hatch() {
    let _guard = lint_lock().lock().expect("lock fixture lint test");
    cleanup_stale_temp_lint_dirs();
    let temp = TempFixtureDir::new("allow");
    temp.write(
        "allow.txt",
        "owner=test@gmail.com // LINT-ALLOW: fixture-anonymization (justification: test data)\n",
    );

    let output = run_lint();
    assert!(
        output.status.success(),
        "lint must allow a justified per-line escape hatch\n{}",
        output_text(&output)
    );
}
