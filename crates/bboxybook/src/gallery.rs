use crate::{Result, modules};
use bbt_ui_model::{
    Density, Theme,
    draw::{Color, DrawCtx, DrawList},
    layout::Rect,
};
use serde::{Deserialize, Serialize};

pub const DESIGN: &str = include_str!("../fixtures/design-system-v22.json");
pub const DENSITIES: [Density; 4] = [
    Density::Touch,
    Density::Comfortable,
    Density::Cozy,
    Density::Compact,
];
pub const THEMES: [Theme; 2] = [Theme::Dark, Theme::Light];
pub fn density_name(d: Density) -> &'static str {
    match d {
        Density::Touch => "touch",
        Density::Comfortable => "comfortable",
        Density::Cozy => "cozy",
        Density::Compact => "compact",
    }
}
pub fn theme_name(t: Theme) -> &'static str {
    match t {
        Theme::Dark => "dark",
        Theme::Light => "light",
    }
}
pub fn density(s: &str) -> Density {
    match s {
        "touch" => Density::Touch,
        "comfortable" => Density::Comfortable,
        "compact" => Density::Compact,
        _ => Density::Cozy,
    }
}
pub fn theme(s: &str) -> Theme {
    if s == "light" {
        Theme::Light
    } else {
        Theme::Dark
    }
}
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub states: Vec<String>,
    pub page: String,
    pub selector: String,
    pub checker: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub component: String,
    pub state: String,
    pub slug: String,
    pub density: String,
    pub theme: String,
    pub page: String,
    pub selector: String,
    pub width: u32,
    pub height: u32,
    pub checker: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture: Option<serde_json::Value>,
}
impl Case {
    pub fn relative(&self) -> String {
        format!(
            "{}/{}-{}-{}.png",
            self.component, self.slug, self.density, self.theme
        )
    }
}
pub fn slug(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_lowercase());
        } else if !result.ends_with('-') {
            result.push('-');
        }
    }
    result.trim_matches('-').to_owned()
}
fn entry(id: &str, page: &str, states: &[&str], checker: bool) -> Entry {
    Entry {
        id: id.into(),
        page: page.into(),
        selector: ".card > .body".into(),
        states: states.iter().map(|s| (*s).into()).collect(),
        checker,
    }
}
/// Entries are explicit implementations. The test rejects a newly built design entry
/// until it is deliberately added here; it never auto-labels a fallback as coverage.
pub fn entries() -> Vec<Entry> {
    let mut out = vec![
        entry(
            "tokens",
            "tokens.html",
            &[
                "palette",
                "state + role",
                "seat colours",
                "density",
                "sizes + motion",
            ],
            false,
        ),
        entry(
            "glyph",
            "glyph.html",
            &[
                "active",
                "idle",
                "waiting",
                "errored",
                "all four in a row",
                "grades: reported · shim · inferred",
                "waiting kinds: permission · question · auth",
            ],
            true,
        ),
        entry(
            "harness-icon",
            "harness-icon.html",
            &["claude", "codex", "pi", "omp", "zsh", "gear", "sidebarfox"],
            true,
        ),
        entry(
            "seat_row",
            "seat-row.html",
            &[
                "active",
                "idle",
                "waiting",
                "errored",
                "focused × state",
                "unread badge",
                "subtitle opt-in",
                "zsh seat",
                "long names",
            ],
            false,
        ),
        entry(
            "mini_grid",
            "mini-grid.html",
            &[
                "single",
                "V",
                "H",
                "V(a, H(b,c))",
                "H(V(a,b), c)",
                "depth 3",
                "focused cell",
                "waiting + errored cells",
            ],
            true,
        ),
        entry(
            "tab_row",
            "tab-row.html",
            &[
                "1 seat collapsed",
                "3 seats collapsed",
                "8 seats collapsed",
                "expanded",
                "expanded with focus",
            ],
            false,
        ),
        entry(
            "band",
            "space-header.html",
            &[
                "collapsed small",
                "collapsed large",
                "collapsed with waiting",
                "expanded",
            ],
            false,
        ),
        entry(
            "sidebar",
            "sidebar.html",
            &["10 seats", "100 seats", "1000 seats", "phone sheet"],
            false,
        ),
        entry(
            "title_strip",
            "title-strip.html",
            &[
                "idle",
                "active",
                "waiting",
                "errored",
                "focused vs unfocused",
                "row + pane breathe together",
                "three seats, one focused",
            ],
            false,
        ),
        entry(
            "launcher",
            "picker.html",
            &[
                "open",
                "typed \"c\"",
                "typed \"co\"",
                "typed \"z\"",
                "no match",
                "rerolled",
            ],
            false,
        ),
        entry(
            "message-row",
            "message-row.html",
            &[
                "oracle",
                "executor",
                "creativity",
                "unsigned",
                "reply nested",
                "stream",
                "arrival ripple",
            ],
            false,
        ),
        entry("logo", "@component", &["identity png"], true),
        entry(
            "toolbar-icon",
            "toolbar-icon.html",
            &[
                "sidebar toggle · closed",
                "sidebar toggle · open",
                "sizes ladder",
                "in our title bar",
                "in the menubar",
            ],
            true,
        ),
        entry(
            "settings_tab",
            "settings.html",
            &[
                "Account",
                "Cloud",
                "Machines",
                "Roles",
                "Harnesses",
                "new harness",
                "Appearance",
                "Shortcuts",
                "in a tab",
            ],
            false,
        ),
    ];
    // These previews reuse the lab's real exported components through the capture adapter.
    for id in [
        "tile",
        "badge",
        "kbd",
        "role_mark",
        "swatch",
        "hairline",
        "pill",
        "crumb",
        "chevron",
        "bumper",
        "close_dot",
        "mute_toggle",
        "muted_mark",
        "ghost",
        "drop_zone",
        "divider",
        "cursor",
        "where_mark",
        "artifact_row",
        "responsible_chip",
        "touched_count",
        "wikilink_chip",
        "title_bar",
        "swarm_card",
        "new_seat_row",
        "closed_strip",
        "new_swarm_button",
        "stream_dock",
        "pane",
        "banner",
        "glow_canvas",
        "task_row",
        "epic_group",
        "graph_tab",
        "provenance_card",
        "subscriptions_card",
        "external_lane",
    ] {
        out.push(entry(
            id,
            "@component",
            &["normal", "focused", "unfocused"],
            false,
        ));
    }
    out.push(entry(
        "gap_marker",
        "@component",
        &["retained", "unavailable"],
        false,
    ));
    out.push(entry(
        "tick_ladder",
        "@component",
        &[
            "sender-stored",
            "recipient-stored",
            "native-queued",
            "native-written",
            "native-unknown",
            "application-acknowledged",
        ],
        false,
    ));
    out.push(entry(
        "nested_subagent_rows",
        "@component",
        &["claude", "codex", "overflow"],
        false,
    ));
    for scene in ["3", "15"] {
        let mut e = entry(
            &format!("showcase-{scene}"),
            &format!("showcase.html?scene={scene}"),
            &["normal"],
            false,
        );
        e.selector = "body".into();
        out.push(e);
    }
    out
}
/// Current accepted layer matrices, with the routing needed for native geometry.
/// `cases()` remains the original registry for the separate legacy report.
pub fn parity_cases() -> Vec<Case> {
    let mut out = crate::materials::cases();
    out.extend(crate::styles::cases());
    out.extend(crate::atoms::cases());
    out.extend(crate::components::cases());
    out.extend(crate::components::mask_cases());
    out.extend(crate::widgets::cases());
    out.extend(crate::overlays::cases());
    // A surface-only shell and the complete module share inputs, but must not
    // overwrite one another's PNGs or geometry in the combined report.
    out.extend(crate::overlays::surface_cases().into_iter().map(|mut case| {
        case.component.push_str("-surface");
        case
    }));
    out.extend(crate::seat_rows::cases());
    out.extend(crate::tabs::cases());
    out.extend(crate::swarms::cases());
    out.extend(crate::panes::cases());
    out.extend(crate::title_bars::cases());
    out.extend(crate::launchers::cases());
    out.extend(crate::streams::cases());
    out.extend(crate::graphs::cases());
    out.extend(crate::settings::cases());
    out.extend(crate::perf::cases());
    out.extend(crate::documents::cases());
    out.extend(crate::paths::cases());
    out
}

pub fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    for e in entries() {
        for state in e.states {
            for d in DENSITIES {
                for t in THEMES {
                    let showcase = e.id.starts_with("showcase");
                    out.push(Case {
                        component: e.id.clone(),
                        slug: slug(&state),
                        state: state.clone(),
                        density: density_name(d).into(),
                        theme: theme_name(t).into(),
                        page: e.page.clone(),
                        selector: e.selector.clone(),
                        width: if showcase { 1440 } else { 1246 },
                        height: if showcase {
                            868
                        } else {
                            modules::height(&e.id, &state, d)
                        },
                        checker: e.checker,
                        fixture: None,
                    });
                }
            }
        }
    }
    out
}
pub fn draw(case: &Case) -> Result<DrawList> {
    let ctx = DrawCtx::new(theme(&case.theme), density(&case.density));
    let mut list = DrawList::default();
    let bounds = Rect::new(0.0, 0.0, case.width as f32, case.height as f32);
    let ground = Color::hex(if case.component.starts_with("showcase") {
        ctx.theme.palette().bg
    } else {
        ctx.theme.palette().surface
    });
    list.rect(bounds, 0.0, ground);
    if case.checker {
        for y in (0..case.height).step_by(8) {
            for x in (0..case.width).step_by(8) {
                if (x / 8 + y / 8) % 2 == 1 {
                    list.rect(
                        Rect::new(x as f32, y as f32, 8.0, 8.0),
                        0.0,
                        Color::hex(ctx.theme.palette().surface_2),
                    );
                }
            }
        }
    }
    let rect = if case.component.starts_with("showcase") {
        bounds
    } else {
        Rect::new(
            14.0,
            14.0,
            (bounds.w - 28.0).max(1.0),
            (bounds.h - 28.0).max(1.0),
        )
    };
    if !crate::fixtures::draw(case, &ctx, &mut list)? {
        modules::draw(&case.component, &case.state, rect, &ctx, &mut list)?;
    }
    list.validate()?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    #[test]
    fn current_parity_matrix_has_unique_paths_and_geometry_routing() {
        let all = parity_cases();
        let paths: BTreeSet<_> = all.iter().map(Case::relative).collect();
        assert_eq!(paths.len(), all.len());
        let mut layers = BTreeSet::new();
        for case in &all {
            assert_ne!(case.fixture.as_ref().map(|f| &f["reference"]), Some(&serde_json::json!("baseline")));
            if matches!(case.component.as_str(), "materials" | "style") {
                layers.insert(case.component.as_str());
            } else {
                layers.insert(case.fixture.as_ref().and_then(|f| f["layer"].as_str()).expect("current case needs geometry routing"));
            }
        }
        assert_eq!(layers, BTreeSet::from([
            "materials", "style", "atoms", "components", "floating-surfaces",
            "seat-row", "tabs", "swarms", "panes", "titlebars", "launchers",
            "streams", "graphs", "settings", "perf", "documents", "paths",
        ]));
        assert!(paths.contains("components/button-cozy-dark.png"));
        assert!(paths.contains("components/edge-fade-cozy-dark.png"));
        assert!(paths.contains("style/modal-cozy-dark.png"));
        assert!(!paths.contains("settings_tab/new-harness-cozy-dark.png"));
    }
    #[test]
    fn registry_covers_exactly_the_built_design_entries() {
        let design: serde_json::Value = serde_json::from_str(DESIGN).unwrap();
        assert_eq!(
            design["version"], 22,
            "coverage baseline must be updated deliberately"
        );
        let registry: BTreeSet<_> = entries().into_iter().map(|e| e.id).collect();
        let mut missing = vec![];
        for group in ["elements", "modules"] {
            for (id, v) in design[group].as_object().unwrap() {
                let status = v["status"].as_str().unwrap_or("built");
                if !status.contains("not built")
                    && !status.contains("deferred")
                    && !registry.contains(id)
                {
                    missing.push(id);
                }
            }
        }
        assert!(
            missing.is_empty(),
            "built entries absent from gallery: {missing:?}"
        );
    }
    #[test]
    fn matrix_paths_are_unique_and_every_entry_builds_a_balanced_list() {
        let all = cases();
        let paths: BTreeSet<_> = all.iter().map(Case::relative).collect();
        assert_eq!(paths.len(), all.len());
        for e in entries() {
            let mut c = all.iter().find(|c| c.component == e.id).unwrap().clone();
            c.width = 800;
            c.height = 600;
            let list = draw(&c).unwrap();
            assert!(list.primitives.len() > 1, "{}", e.id);
        }
    }
}
