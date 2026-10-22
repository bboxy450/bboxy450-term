use bboxybook::gallery::Case;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    case: Case,
}

impl Fixture {
    fn new(historical: bool, pixel_review: bool) -> Result<Self> {
        let thread = std::thread::current();
        let test_name = thread.name().unwrap_or("fixture");
        let root = std::env::temp_dir().join(format!(
            "bboxybook-parity-gate-{}-{}-{test_name}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&root)?;
        let mut case = bboxybook::components::cases().remove(0);
        case.width = 1;
        case.height = 1;
        case.fixture = Some(serde_json::json!({
            "layer": "components",
            "reference": if historical { "baseline" } else { "current" },
            "component": {"tree": {"kind": "text"}}
        }));
        let fixture = Self { root, case };
        for (side, pixel) in [
            ("web", [0, 0, 0, 255]),
            (
                "native",
                if pixel_review {
                    [255; 4]
                } else {
                    [0, 0, 0, 255]
                },
            ),
        ] {
            let path = fixture.root.join(side).join(fixture.case.relative());
            fs::create_dir_all(path.parent().unwrap())?;
            bbt_render::write_png(&path, &pixel, 1, 1)?;
        }
        fs::write(
            fixture.root.join("cases.json"),
            serde_json::to_vec(&[&fixture.case])?,
        )?;
        Ok(fixture)
    }

    fn geometry(&self, rect_error: f64, complete: bool) -> Result {
        let baseline = if complete {
            serde_json::json!({"native": 1., "web": 1.})
        } else {
            serde_json::Value::Null
        };
        fs::write(
            self.root.join("native/layouts.json"),
            serde_json::to_vec(&serde_json::json!([{
                "key": self.case.relative(),
                "tree": {
                    "kind": "text",
                    "native": [0., 0., 1., 1.],
                    "web": [rect_error, 0., 1., 1.],
                    "baseline": baseline,
                    "children": []
                }
            }]))?,
        )?;
        Ok(())
    }

    fn compare(&self, strict: bool, exit: i32, status: &str, summary: &str) -> Result {
        let mut command = Command::new(env!("CARGO_BIN_EXE_bboxybook"));
        command
            .arg("compare")
            .arg("--out")
            .arg(&self.root)
            .arg("--manifest")
            .arg(self.root.join("cases.json"));
        if strict {
            command.arg("--strict-geometry");
        }
        let output = command.output()?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.status.code(), Some(exit), "{text}");
        assert!(text.contains(summary), "{text}");
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(self.root.join("report.json"))?)?;
        assert_eq!(report[0]["status"], status);
        assert!(self.root.join("index.html").exists());
        assert!(self.root.join("report.md").exists());
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn strict_geometry_review_exits_one_after_writing_the_report() -> Result {
    let fixture = Fixture::new(false, false)?;
    fixture.geometry(1., true)?;
    fixture.compare(
        true,
        1,
        "review geometry",
        "parity gate: 1 geometry failures (fail)",
    )
}

#[test]
fn pixel_only_review_exits_zero_and_is_counted_as_advisory() -> Result {
    let fixture = Fixture::new(false, true)?;
    fixture.geometry(0., true)?;
    fixture.compare(
        true,
        0,
        "review pixels",
        "parity gate: pass (1 pixel reviews advisory)",
    )
}

#[test]
fn historical_geometry_review_exits_zero() -> Result {
    let fixture = Fixture::new(true, false)?;
    fixture.geometry(1., true)?;
    fixture.compare(
        true,
        0,
        "review geometry",
        "parity gate: pass (0 pixel reviews advisory)",
    )
}

#[test]
fn missing_or_incomplete_geometry_exits_one() -> Result {
    for missing in [true, false] {
        let fixture = Fixture::new(false, false)?;
        if !missing {
            fixture.geometry(0., false)?;
        }
        fixture.compare(
            true,
            1,
            "unproven geometry",
            "parity gate: 1 geometry failures (fail)",
        )?;
    }
    Ok(())
}

#[test]
fn missing_reference_does_not_hide_missing_geometry() -> Result {
    let fixture = Fixture::new(false, false)?;
    fs::remove_file(fixture.root.join("web").join(fixture.case.relative()))?;
    fixture.compare(
        true,
        1,
        "missing web reference",
        "parity gate: 1 geometry failures (fail)",
    )
}

#[test]
fn geometry_review_without_strict_flag_remains_advisory() -> Result {
    let fixture = Fixture::new(false, false)?;
    fixture.geometry(1., true)?;
    fixture.compare(
        false,
        0,
        "review geometry",
        "1 geometry failures (advisory; enable --strict-geometry)",
    )
}
