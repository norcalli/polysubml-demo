use compiler_lib::{CompilationResult, State};
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const REGRESSION_DIR: &str = "tests/regression";
const BASELINE_DIR: &str = "tests/regression/baselines";

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// Write directly to stderr, bypassing test harness capture.
fn stderr() -> fs::File {
    fs::OpenOptions::new().write(true).open("/dev/stderr").unwrap()
}

fn load_baseline(path: &Path) -> Option<CompilationResult> {
    let content = fs::read_to_string(path).ok()?;
    let mut lines = content.splitn(2, '\n');
    let status = lines.next()?;
    let body = lines.next().unwrap_or("");
    match status {
        "SUCCESS" => Some(CompilationResult::Success(body.to_string())),
        "ERROR" => Some(CompilationResult::Error(body.to_string())),
        _ => None,
    }
}

fn discover_test_dirs() -> Vec<PathBuf> {
    let regression_dir = workspace_root().join(REGRESSION_DIR);
    let baseline_dir_name = Path::new(BASELINE_DIR).file_name().unwrap();

    let mut dirs: Vec<PathBuf> = fs::read_dir(&regression_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.file_name().unwrap() != baseline_dir_name)
        .collect();
    dirs.sort();
    dirs
}

fn discover_tests(test_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut tests = Vec::new();
    for dir in test_dirs {
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "ml"))
            .collect();
        entries.sort();
        tests.extend(entries);
    }
    tests
}

fn show_diff(out: &mut fs::File, expected: &str, actual: &str) {
    let diff = TextDiff::from_lines(expected, actual);
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => write!(out, "    \x1b[31m-{}\x1b[0m", change).unwrap(),
            ChangeTag::Insert => write!(out, "    \x1b[32m+{}\x1b[0m", change).unwrap(),
            ChangeTag::Equal => write!(out, "     {}", change).unwrap(),
        }
    }
    if !expected.ends_with('\n') || !actual.ends_with('\n') {
        writeln!(out).unwrap();
    }
}

/// Set UPDATE_BASELINES=1 to regenerate all .expected files.
#[test]
fn regression_tests() {
    let mut out = stderr();
    let baseline_dir = workspace_root().join(BASELINE_DIR);
    let test_dirs = discover_test_dirs();
    let tests = discover_tests(&test_dirs);

    let update = std::env::var("UPDATE_BASELINES").is_ok_and(|v| v == "1");

    if update {
        fs::create_dir_all(&baseline_dir).unwrap();
        for test_path in &tests {
            let name = test_path.file_stem().unwrap().to_str().unwrap();
            let source = fs::read_to_string(test_path).unwrap();
            let result = State::new().process(&source);
            let baseline_path = baseline_dir.join(format!("{}.expected", name));
            fs::write(&baseline_path, result.to_string()).unwrap();
            writeln!(out, "  updated {}", name).unwrap();
        }
        writeln!(out, "Updated baselines for {} tests", tests.len()).unwrap();
        return;
    }

    let mut passed = 0;
    let mut skipped = 0;
    let mut failed = Vec::new();

    for test_path in &tests {
        let name = test_path.file_stem().unwrap().to_str().unwrap();
        let baseline_path = baseline_dir.join(format!("{}.expected", name));

        let expected = match load_baseline(&baseline_path) {
            Some(r) => r,
            None => {
                skipped += 1;
                continue;
            }
        };

        let source = fs::read_to_string(test_path).unwrap();
        let actual = State::new().process(&source);

        if actual == expected {
            passed += 1;
            writeln!(out, "  \x1b[32m✓\x1b[0m {}", name).unwrap();
        } else {
            writeln!(out, "\n  FAIL: {}", name).unwrap();
            show_diff(&mut out, &expected.to_string(), &actual.to_string());
            failed.push(name.to_string());
        }
    }

    writeln!(
        out,
        "\n{} passed, {} failed, {} skipped ({} total)",
        passed,
        failed.len(),
        skipped,
        tests.len()
    )
    .unwrap();

    if !failed.is_empty() {
        panic!(
            "{} regression tests failed:\n  {}",
            failed.len(),
            failed.join("\n  ")
        );
    }

    assert!(passed > 0, "No regression tests found");
}
