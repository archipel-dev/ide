//! Test results: the model behind the test runner tree.
//!
//! Results come from JUnit XML reports (Gradle `build/test-results`, Maven
//! `target/surefire-reports`, Bazel `bazel-testlogs/**/test.xml`) read after a test
//! task finishes, and from live console output of `cargo test` (libtest) and
//! `go test -v`.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestOutcome {
    Passed,
    Failed { message: String, details: String },
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    pub name: String,
    pub duration: Option<Duration>,
    pub outcome: TestOutcome,
    /// Captured stdout/stderr of the test, when reported.
    pub output: String,
}

impl TestCase {
    pub fn failed(&self) -> bool {
        matches!(self.outcome, TestOutcome::Failed { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSuite {
    /// Class (JUnit), module path (libtest) or package (Go).
    pub name: String,
    pub cases: Vec<TestCase>,
}

impl TestSuite {
    pub fn duration(&self) -> Option<Duration> {
        let durations: Vec<Duration> = self.cases.iter().filter_map(|case| case.duration).collect();
        (!durations.is_empty()).then(|| durations.iter().sum())
    }

    pub fn failed(&self) -> bool {
        self.cases.iter().any(TestCase::failed)
    }
}

/// A test that can be selected for a re-run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestId {
    pub suite: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestResults {
    suites: Vec<TestSuite>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TestCounts {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

impl TestCounts {
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped
    }
}

impl TestResults {
    pub fn suites(&self) -> &[TestSuite] {
        &self.suites
    }

    pub fn is_empty(&self) -> bool {
        self.suites.iter().all(|suite| suite.cases.is_empty())
    }

    pub fn counts(&self) -> TestCounts {
        let mut counts = TestCounts::default();
        for case in self.suites.iter().flat_map(|suite| &suite.cases) {
            match case.outcome {
                TestOutcome::Passed => counts.passed += 1,
                TestOutcome::Failed { .. } => counts.failed += 1,
                TestOutcome::Skipped => counts.skipped += 1,
            }
        }
        counts
    }

    pub fn duration(&self) -> Option<Duration> {
        let durations: Vec<Duration> = self.suites.iter().filter_map(TestSuite::duration).collect();
        (!durations.is_empty()).then(|| durations.iter().sum())
    }

    pub fn failed_tests(&self) -> Vec<TestId> {
        self.suites
            .iter()
            .flat_map(|suite| {
                suite
                    .cases
                    .iter()
                    .filter(|case| case.failed())
                    .map(|case| TestId {
                        suite: suite.name.clone(),
                        name: case.name.clone(),
                    })
            })
            .collect()
    }

    /// Adds a case, creating its suite on first use (suites keep arrival order).
    pub fn push(&mut self, suite: &str, case: TestCase) {
        match self
            .suites
            .iter_mut()
            .find(|existing| existing.name == suite)
        {
            Some(existing) => {
                // A re-reported case (e.g. libtest failure details) replaces the old entry.
                match existing.cases.iter_mut().find(|c| c.name == case.name) {
                    Some(previous) => *previous = case,
                    None => existing.cases.push(case),
                }
            }
            None => self.suites.push(TestSuite {
                name: suite.to_owned(),
                cases: vec![case],
            }),
        }
    }

    pub fn merge(&mut self, other: TestResults) {
        for suite in other.suites {
            for case in suite.cases {
                self.push(&suite.name, case);
            }
        }
    }

    /// Parses one JUnit XML report (`<testsuite>` or `<testsuites>` root).
    pub fn parse_junit(xml: &str) -> Result<Self, String> {
        let mut reader = Reader::from_str(xml);
        let mut results = TestResults::default();
        let mut suite_name = String::new();
        let mut case: Option<(String, String, TestCase)> = None;
        let mut text_target: Option<TextTarget> = None;
        loop {
            let event = reader.read_event().map_err(|error| {
                format!(
                    "invalid JUnit XML at byte {}: {error}",
                    reader.buffer_position()
                )
            })?;
            match event {
                Event::Start(start) | Event::Empty(start)
                    if matches!(event_kind(&start), "testsuite") =>
                {
                    suite_name = attribute(&start, "name").unwrap_or_default();
                }
                Event::Start(ref start) | Event::Empty(ref start) => {
                    let empty = matches!(event, Event::Empty(_));
                    match event_kind(start) {
                        "testcase" => {
                            let class = attribute(start, "classname")
                                .filter(|class| !class.is_empty())
                                .unwrap_or_else(|| suite_name.clone());
                            let name = attribute(start, "name").unwrap_or_default();
                            let duration = attribute(start, "time")
                                .and_then(|time| time.trim().parse::<f64>().ok())
                                .filter(|secs| secs.is_finite() && *secs >= 0.0)
                                .map(Duration::from_secs_f64);
                            let parsed = TestCase {
                                name: name.clone(),
                                duration,
                                outcome: TestOutcome::Passed,
                                output: String::new(),
                            };
                            if empty {
                                results.push(&class, parsed);
                            } else {
                                case = Some((class, name, parsed));
                            }
                        }
                        kind @ ("failure" | "error") => {
                            if let Some((_, _, current)) = case.as_mut() {
                                current.outcome = TestOutcome::Failed {
                                    message: attribute(start, "message")
                                        .unwrap_or_else(|| kind.to_owned()),
                                    details: String::new(),
                                };
                                if !empty {
                                    text_target = Some(TextTarget::Details);
                                }
                            }
                        }
                        "skipped" => {
                            if let Some((_, _, current)) = case.as_mut() {
                                current.outcome = TestOutcome::Skipped;
                            }
                        }
                        "system-out" | "system-err" if case.is_some() && !empty => {
                            text_target = Some(TextTarget::Output);
                        }
                        "system-out" | "system-err" if !empty => {
                            // Suite-level output (Bazel's test.log) belongs to its case.
                            text_target = Some(TextTarget::SuiteOutput);
                        }
                        _ => {}
                    }
                }
                Event::Text(text) => {
                    if let Some(target) = text_target {
                        let text = text.unescape().map_err(|error| error.to_string())?;
                        results.append_text(target, case.as_mut().map(|(_, _, c)| c), &text);
                    }
                }
                Event::CData(data) => {
                    if let Some(target) = text_target {
                        let text = data.decode().map_err(|error| error.to_string())?;
                        results.append_text(target, case.as_mut().map(|(_, _, c)| c), &text);
                    }
                }
                Event::End(end) => match end.local_name().as_ref() {
                    b"testcase" => {
                        if let Some((class, _, parsed)) = case.take() {
                            results.push(&class, parsed);
                        }
                        text_target = None;
                    }
                    b"failure" | b"error" | b"system-out" | b"system-err" => text_target = None,
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
        }
        Ok(results)
    }

    /// Reads JUnit reports under `root` written at or after `since` (bounded walk,
    /// following the `bazel-testlogs` convenience symlink).
    pub fn collect_reports(root: &Path, since: SystemTime) -> (Self, Vec<(PathBuf, String)>) {
        let mut results = TestResults::default();
        let mut errors = Vec::new();
        let mut reports = Vec::new();
        let walker = walkdir::WalkDir::new(root)
            .max_depth(12)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                !(entry.depth() > 0
                    && (name.starts_with('.') || name == "node_modules" || name == "src"))
            });
        for entry in walker.flatten() {
            let path = entry.path();
            let is_report = is_junit_report(path);
            if is_report {
                reports.push(path.to_owned());
            }
        }
        // Bazel keeps results behind a symlink at the workspace root.
        let testlogs = root.join("bazel-testlogs");
        if testlogs.is_dir() {
            for entry in walkdir::WalkDir::new(&testlogs)
                .follow_links(true)
                .max_depth(20)
                .into_iter()
                .flatten()
            {
                if entry.file_name() == "test.xml" {
                    reports.push(entry.path().to_owned());
                }
            }
        }
        reports.sort();
        reports.dedup();
        for path in reports {
            let fresh = path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .is_ok_and(|modified| modified >= since);
            if !fresh {
                continue;
            }
            match std::fs::read_to_string(&path)
                .map_err(|error| error.to_string())
                .and_then(|xml| Self::parse_junit(&xml))
            {
                Ok(mut parsed) => {
                    // Bazel names suites after the target; use the full label instead.
                    if let Some(label) = bazel_label(&testlogs, &path) {
                        let target = label.rsplit(':').next().unwrap_or(&label).to_owned();
                        for suite in &mut parsed.suites {
                            // The single case is named after the target (`lib/x_test`).
                            for case in &mut suite.cases {
                                if case.name == suite.name {
                                    case.name = target.clone();
                                }
                            }
                            suite.name = label.clone();
                        }
                    }
                    results.merge(parsed)
                }
                Err(error) => errors.push((path, error)),
            }
        }
        (results, errors)
    }
}

/// `bazel-testlogs/lib/greeter_test/test.xml` → `//lib:greeter_test`.
fn bazel_label(testlogs: &Path, report: &Path) -> Option<String> {
    let relative = report.strip_prefix(testlogs).ok()?;
    let target_dir = relative.parent()?;
    let name = target_dir.file_name()?.to_string_lossy();
    let package = target_dir
        .parent()
        .map(|dir| dir.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    Some(format!("//{package}:{name}"))
}

fn is_junit_report(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    if !(name.starts_with("TEST-") && name.ends_with(".xml")) {
        return false;
    }
    let parent = path.to_string_lossy().replace('\\', "/");
    parent.contains("/build/test-results/")
        || parent.contains("/target/surefire-reports/")
        || parent.contains("/target/failsafe-reports/")
}

#[derive(Clone, Copy)]
enum TextTarget {
    Details,
    Output,
    SuiteOutput,
}

impl TestResults {
    fn append_text(&mut self, target: TextTarget, case: Option<&mut TestCase>, text: &str) {
        match (target, case) {
            (TextTarget::SuiteOutput, _) => {
                if let Some(last) = self.suites.last_mut().and_then(|s| s.cases.last_mut()) {
                    last.output.push_str(text.trim_start_matches('\n'));
                }
            }
            (target, Some(case)) => append(target, case, text),
            (_, None) => {}
        }
    }
}

fn append(target: TextTarget, case: &mut TestCase, text: &str) {
    match target {
        TextTarget::Details => {
            if let TestOutcome::Failed { details, .. } = &mut case.outcome {
                details.push_str(text);
            }
        }
        TextTarget::Output | TextTarget::SuiteOutput => case.output.push_str(text),
    }
}

fn event_kind<'a>(start: &'a BytesStart<'_>) -> &'a str {
    std::str::from_utf8(start.local_name().into_inner()).unwrap_or_default()
}

fn attribute(start: &BytesStart<'_>, name: &str) -> Option<String> {
    start
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.local_name().as_ref() == name.as_bytes())
        .and_then(|attribute| attribute.unescape_value().ok())
        .map(|value| value.into_owned())
}

/// Live parser for `cargo test` (libtest) and `go test -v` console output.
#[derive(Debug, Clone, Default)]
pub struct ConsoleTestParser {
    /// libtest prints failure details after all results: `---- name stdout ----`.
    failure_details_for: Option<(String, String)>,
    /// Go suite (package) of the most recent `=== RUN`.
    go_package: Option<String>,
    pending_go: Vec<(String, TestCase)>,
}

impl ConsoleTestParser {
    pub fn on_line(&mut self, line: &str, results: &mut TestResults) {
        // libtest: `test tests::adds ... ok` / `FAILED` / `ignored`
        if let Some(rest) = line.strip_prefix("test ")
            && let Some((name, status)) = rest.rsplit_once(" ... ")
        {
            let outcome = match status.trim() {
                "ok" => Some(TestOutcome::Passed),
                "FAILED" => Some(TestOutcome::Failed {
                    message: "test failed".into(),
                    details: String::new(),
                }),
                status if status.starts_with("ignored") => Some(TestOutcome::Skipped),
                _ => None,
            };
            if let Some(outcome) = outcome {
                let (suite, test) = split_rust_path(name.trim());
                results.push(
                    &suite,
                    TestCase {
                        name: test,
                        duration: None,
                        outcome,
                        output: String::new(),
                    },
                );
            }
            self.failure_details_for = None;
            return;
        }
        if let Some(name) = line
            .strip_prefix("---- ")
            .and_then(|rest| rest.strip_suffix(" stdout ----"))
        {
            let (suite, test) = split_rust_path(name.trim());
            self.failure_details_for = Some((suite, test));
            return;
        }
        if line.starts_with("failures:") || line.starts_with("test result:") {
            self.failure_details_for = None;
            return;
        }
        if let Some((suite, test)) = &self.failure_details_for {
            if let Some(case) = results
                .suites
                .iter_mut()
                .find(|s| &s.name == suite)
                .and_then(|s| s.cases.iter_mut().find(|c| &c.name == test))
                && let TestOutcome::Failed { message, details } = &mut case.outcome
            {
                if message == "test failed" && line.contains("panicked at") {
                    *message = line.trim().to_owned();
                }
                details.push_str(line);
                details.push('\n');
            }
            return;
        }
        // Bazel summary: `//:app_test   (cached) PASSED in 0.0s` / `FAILED in 0.1s`.
        if (line.starts_with("//") || line.starts_with('@'))
            && let Some((label, status)) = line.split_once(char::is_whitespace)
        {
            let status = status.trim().trim_start_matches("(cached)").trim();
            let outcome = if status.starts_with("PASSED") {
                Some(TestOutcome::Passed)
            } else if status.starts_with("FAILED") || status.starts_with("TIMEOUT") {
                Some(TestOutcome::Failed {
                    message: status.split(" in ").next().unwrap_or(status).to_owned(),
                    details: String::new(),
                })
            } else if status.starts_with("NO STATUS") || status.starts_with("SKIPPED") {
                Some(TestOutcome::Skipped)
            } else {
                None
            };
            if let Some(outcome) = outcome {
                let duration = status
                    .rsplit_once(" in ")
                    .and_then(|(_, time)| time.strip_suffix('s'))
                    .and_then(|secs| secs.parse::<f64>().ok())
                    .filter(|secs| secs.is_finite() && *secs >= 0.0)
                    .map(Duration::from_secs_f64);
                let name = label.rsplit(':').next().unwrap_or(label).to_owned();
                // Do not downgrade a richer report already merged for this target.
                let known = results
                    .suites
                    .iter()
                    .any(|suite| suite.name == label && suite.cases.iter().any(|c| c.name == name));
                if !known {
                    results.push(
                        label,
                        TestCase {
                            name,
                            duration,
                            outcome,
                            output: String::new(),
                        },
                    );
                }
                return;
            }
        }
        // go test -v: `--- PASS: TestAdd (0.00s)` then `ok  	example.com/pkg	0.003s`
        let trimmed = line.trim_start();
        for (prefix, outcome) in [
            ("--- PASS: ", TestOutcome::Passed),
            ("--- SKIP: ", TestOutcome::Skipped),
            (
                "--- FAIL: ",
                TestOutcome::Failed {
                    message: "test failed".into(),
                    details: String::new(),
                },
            ),
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let (name, duration) = match rest.rsplit_once(" (") {
                    Some((name, time)) => (
                        name.to_owned(),
                        time.trim_end_matches(')')
                            .strip_suffix('s')
                            .and_then(|secs| secs.parse::<f64>().ok())
                            .filter(|secs| secs.is_finite() && *secs >= 0.0)
                            .map(Duration::from_secs_f64),
                    ),
                    None => (rest.to_owned(), None),
                };
                self.pending_go.push((
                    name.clone(),
                    TestCase {
                        name,
                        duration,
                        outcome,
                        output: String::new(),
                    },
                ));
                return;
            }
        }
        for prefix in ["ok  \t", "FAIL\t", "ok \t", "ok\t"] {
            if let Some(rest) = line.strip_prefix(prefix) {
                let package = rest.split_whitespace().next().unwrap_or("go").to_owned();
                self.go_package = Some(package.clone());
                for (_, case) in self.pending_go.drain(..) {
                    results.push(&package, case);
                }
                return;
            }
        }
    }

    /// Flushes Go results whose package summary never arrived.
    pub fn finish(&mut self, results: &mut TestResults) {
        let package = self.go_package.clone().unwrap_or_else(|| "go test".into());
        for (_, case) in self.pending_go.drain(..) {
            results.push(&package, case);
        }
    }
}

/// `tests::math::adds` → (`tests::math`, `adds`); a bare name goes to `(top level)`.
fn split_rust_path(path: &str) -> (String, String) {
    match path.rsplit_once("::") {
        Some((suite, name)) => (suite.to_owned(), name.to_owned()),
        None => ("(top level)".to_owned(), path.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRADLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuite name="com.example.AppTest" tests="3" skipped="1" failures="1" errors="0" time="0.05">
  <properties/>
  <testcase name="greets()" classname="com.example.AppTest" time="0.021"/>
  <testcase name="fails()" classname="com.example.AppTest" time="0.004">
    <failure message="org.opentest4j.AssertionFailedError: expected: &lt;1&gt; but was: &lt;2&gt;" type="org.opentest4j.AssertionFailedError">org.opentest4j.AssertionFailedError: expected: &lt;1&gt; but was: &lt;2&gt;
	at com.example.AppTest.fails(AppTest.java:14)
</failure>
    <system-out><![CDATA[debug line
]]></system-out>
  </testcase>
  <testcase name="later()" classname="com.example.AppTest" time="0.0">
    <skipped/>
  </testcase>
  <system-out><![CDATA[]]></system-out>
</testsuite>
"#;

    #[test]
    fn parses_gradle_junit_report() {
        let results = TestResults::parse_junit(GRADLE_XML).unwrap();
        assert_eq!(results.suites().len(), 1);
        let suite = &results.suites()[0];
        assert_eq!(suite.name, "com.example.AppTest");
        assert_eq!(suite.cases.len(), 3);
        assert_eq!(suite.cases[0].duration, Some(Duration::from_millis(21)));
        let TestOutcome::Failed { message, details } = &suite.cases[1].outcome else {
            panic!("expected failure");
        };
        assert_eq!(
            message,
            "org.opentest4j.AssertionFailedError: expected: <1> but was: <2>"
        );
        assert!(details.contains("AppTest.java:14"), "{details}");
        assert_eq!(suite.cases[1].output, "debug line\n");
        assert_eq!(suite.cases[2].outcome, TestOutcome::Skipped);
        assert_eq!(
            results.counts(),
            TestCounts {
                passed: 1,
                failed: 1,
                skipped: 1
            }
        );
        assert_eq!(
            results.failed_tests(),
            [TestId {
                suite: "com.example.AppTest".into(),
                name: "fails()".into()
            }]
        );
    }

    #[test]
    fn parses_bazel_testsuites_root_and_rejects_garbage() {
        let xml = r#"<testsuites><testsuite name="//:app_test" tests="1"><testcase name="//:app_test" status="run" duration="1" time="1.5"><error message="exited with error code 1"></error></testcase></testsuite></testsuites>"#;
        let results = TestResults::parse_junit(xml).unwrap();
        assert_eq!(results.suites()[0].name, "//:app_test");
        assert!(results.suites()[0].cases[0].failed());
        assert!(
            TestResults::parse_junit("<testsuite><testcase").is_err()
                || TestResults::parse_junit("<testsuite><testcase")
                    .unwrap()
                    .is_empty()
        );
    }

    #[test]
    fn bazel_reports_use_labels_and_suite_output() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-bazel-logs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("bazel-testlogs/lib/greeter_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("test.xml"),
            r#"<testsuites><testsuite name="lib/greeter_test" tests="1"><testcase name="lib/greeter_test" status="run" time="0"><error message="exited with error code 1"></error></testcase>
      <system-out>
<![CDATA[expected 2, got 3]]>
      </system-out></testsuite></testsuites>"#,
        )
        .unwrap();
        let (results, errors) = TestResults::collect_reports(&root, SystemTime::UNIX_EPOCH);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(results.suites()[0].name, "//lib:greeter_test");
        assert!(
            results.suites()[0].cases[0]
                .output
                .contains("expected 2, got 3")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn libtest_console_output_with_failure_details() {
        let mut results = TestResults::default();
        let mut parser = ConsoleTestParser::default();
        for line in [
            "running 3 tests",
            "test tests::adds ... ok",
            "test tests::subtracts ... FAILED",
            "test slow ... ignored, takes a minute",
            "",
            "failures:",
            "",
            "---- tests::subtracts stdout ----",
            "thread 'tests::subtracts' panicked at src/lib.rs:12:9:",
            "assertion `left == right` failed",
            "",
            "failures:",
            "    tests::subtracts",
            "test result: FAILED. 1 passed; 1 failed; 1 ignored",
        ] {
            parser.on_line(line, &mut results);
        }
        assert_eq!(
            results.counts(),
            TestCounts {
                passed: 1,
                failed: 1,
                skipped: 1
            }
        );
        let case = &results.suites()[0].cases[1];
        let TestOutcome::Failed { message, details } = &case.outcome else {
            panic!("expected failure");
        };
        assert!(message.contains("panicked at src/lib.rs:12:9"), "{message}");
        assert!(details.contains("assertion `left == right` failed"));
        assert_eq!(results.suites()[1].name, "(top level)");
    }

    #[test]
    fn bazel_summary_lines_include_cached_results() {
        let mut results = TestResults::default();
        let mut parser = ConsoleTestParser::default();
        for line in [
            "//:app_test                                                     (cached) PASSED in 0.0s",
            "//lib:flaky_test                                                FAILED in 0.2s",
            "  /root/.cache/bazel/.../testlogs/lib/flaky_test/test.log",
            "Executed 1 out of 2 tests: 1 test passes and 1 fails locally.",
        ] {
            parser.on_line(line, &mut results);
        }
        assert_eq!(results.suites()[0].name, "//:app_test");
        assert_eq!(results.suites()[0].cases[0].name, "app_test");
        assert_eq!(results.counts().failed, 1);
        assert_eq!(results.failed_tests()[0].suite, "//lib:flaky_test");
        assert_eq!(
            results.suites()[1].cases[0].duration,
            Some(Duration::from_millis(200))
        );
    }

    #[test]
    fn go_test_verbose_output() {
        let mut results = TestResults::default();
        let mut parser = ConsoleTestParser::default();
        for line in [
            "=== RUN   TestAdd",
            "--- PASS: TestAdd (0.01s)",
            "=== RUN   TestSub",
            "    calc_test.go:9: wrong",
            "--- FAIL: TestSub (0.00s)",
            "FAIL",
            "FAIL\texample.com/calc\t0.004s",
        ] {
            parser.on_line(line, &mut results);
        }
        parser.finish(&mut results);
        assert_eq!(results.suites()[0].name, "example.com/calc");
        assert_eq!(results.counts().failed, 1);
        assert_eq!(
            results.suites()[0].cases[0].duration,
            Some(Duration::from_millis(10))
        );
    }

    #[test]
    fn collects_only_fresh_reports() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-test-reports-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("app/build/test-results/test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("TEST-com.example.AppTest.xml"), GRADLE_XML).unwrap();
        std::fs::write(
            dir.join("TEST-broken.xml"),
            "<testsuite><testcase name='x'><failure",
        )
        .unwrap();
        std::fs::write(dir.join("unrelated.xml"), GRADLE_XML).unwrap();
        let (results, errors) = TestResults::collect_reports(&root, SystemTime::UNIX_EPOCH);
        assert_eq!(results.counts().total(), 3);
        assert_eq!(errors.len(), 1, "{errors:?}");
        let future = SystemTime::now() + Duration::from_secs(3600);
        assert!(TestResults::collect_reports(&root, future).0.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
