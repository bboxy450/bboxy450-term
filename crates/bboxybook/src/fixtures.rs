//! JSON exported from the frozen web page's component inputs, including FIX
//! from fixtures.js. Bounds describe the lab fixture, not component internals.
use crate::{Result, gallery::Case, modules::*};
use bbt_ui_model::{draw::*, layout::Rect, *};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct Capture {
    version: u32,
    calls: Vec<Call>,
}
#[derive(Debug, Deserialize)]
struct Call {
    name: String,
    args: Vec<Value>,
    rect: Bounds,
}
#[derive(Debug, Deserialize)]
struct Bounds {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
impl Bounds {
    fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }
}
fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
fn array(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn ident(v: &str) -> u128 {
    v.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    }) as u128
}
fn id(v: &Value) -> SeatId {
    SeatId::from_u128(ident(s(v, "key")))
}
fn role(v: &Value) -> Option<Role> {
    let name = v.as_str().or_else(|| v["name"].as_str())?;
    Some(Role {
        name: name.into(),
        class: if v["class"] == "oracle" || name == "oracle" {
            RoleClass::Oracle
        } else {
            RoleClass::Worker
        },
    })
}
fn color(v: &Value, theme: Theme) -> Hsl {
    let c = s(v, "color");
    if let Some(raw) = c.strip_prefix("hsl(") {
        let nums: Vec<f32> = raw
            .trim_end_matches(')')
            .split_whitespace()
            .filter_map(|n| n.trim_end_matches('%').parse().ok())
            .collect();
        if nums.len() == 3 {
            return Hsl {
                hue: nums[0],
                saturation: nums[1],
                lightness: nums[2],
            };
        }
    }
    if c.starts_with('#') {
        let h = Color::hex(c).to_hsl();
        return Hsl {
            hue: h[0],
            saturation: h[1] * 100.,
            lightness: h[2] * 100.,
        };
    }
    SeatColor::Derived.resolve(s(v, "key"), theme)
}
pub fn seat(v: &Value, swarm: SwarmId, theme: Theme) -> Seat {
    let mut out = Seat::new(id(v), swarm, [0; 32], s(v, "name"), s(v, "harness"));
    out.title = s(v, "title").into();
    out.role = role(&v["role"]);
    out.activity = match s(v, "state") {
        "active" => Activity::Active,
        "errored" => Activity::Errored,
        "waiting" => Activity::Waiting(match s(v, "waitKind") {
            "permission" => WaitKind::Permission,
            "auth" => WaitKind::Auth,
            _ => WaitKind::Question,
        }),
        _ => Activity::Idle,
    };
    out.grade = match s(v, "grade") {
        "shim" => Grade::Shim,
        "inferred" => Grade::Inferred,
        _ => Grade::Reported,
    };
    out.where_ = match s(v, "where") {
        "ssh" => Where::Ssh {
            host: s(v, "host").into(),
        },
        "cloud" => Where::Cloud {
            provider: s(v, "provider").into(),
            host: s(v, "host").into(),
        },
        _ => Where::Local,
    };
    out.muted = v["muted"].as_bool().unwrap_or(false);
    if let Some(n) = v["unread"].as_u64().filter(|n| *n > 0) {
        out.unread = Some((
            n as u32,
            role(&v["unreadRole"]).unwrap_or(Role {
                name: "executor".into(),
                class: RoleClass::Worker,
            }),
        ));
    }
    out.color = SeatColor::Override(color(v, theme));
    out
}
pub fn tree(v: &Value) -> Result<SplitNode> {
    if !s(v, "key").is_empty() {
        return Ok(SplitNode::Leaf(id(v)));
    }
    if v["dir"] != "v" && v["dir"] != "h" {
        return Err("fixture has no split direction or seat key".into());
    }
    Ok(SplitNode::split(
        if v["dir"] == "v" {
            Axis::Row
        } else {
            Axis::Column
        },
        v["ratio"].as_f64().unwrap_or(0.5) as f32,
        tree(&v["a"])?,
        tree(&v["b"])?,
    ))
}
fn leaves(v: &Value) -> Vec<&Value> {
    if v["key"].is_string() {
        vec![v]
    } else {
        [leaves(&v["a"]), leaves(&v["b"])].concat()
    }
}
fn tab_seats(v: &Value) -> Vec<&Value> {
    v["seats"]
        .as_array()
        .map(|a| a.iter().collect())
        .unwrap_or_else(|| leaves(&v["tree"]))
}
fn contains(v: &Value, key: &str) -> bool {
    array(v).iter().any(|v| v.as_str() == Some(key))
}

/// Conversion is strict about identity/tree ownership via Model::new. The web
/// key is fixture identity only; it is never a credential or a real seat grant.
pub fn model(data: &Value, opts: &Value, theme: Theme) -> Result<Model> {
    let mut seats = BTreeMap::new();
    let mut swarms = Vec::new();
    for sp in array(&data["spaces"]) {
        let swarm = SwarmId::from_u128(ident(s(sp, "key")));
        let mut tabs = Vec::new();
        for (n, t) in array(&sp["tabs"]).iter().enumerate() {
            for value in tab_seats(t) {
                seats.insert(id(value), seat(value, swarm, theme));
            }
            let mut tab = Tab::new(
                TabId::from_u128(ident(&format!("{}/{n}", s(sp, "key")))),
                s(t, "name"),
                tree(&t["tree"])?,
            );
            tab.shortcut = s(t, "shortcut").trim_start_matches('⌘').parse().ok();
            tab.expanded = contains(
                &opts["openTabs"],
                &format!("{}/{}", s(sp, "key"), s(t, "name")),
            );
            tabs.push(tab);
        }
        let mut out = Swarm::new(swarm, s(sp, "name"), tabs);
        out.color = SeatColor::Override(color(sp, theme));
        for ph in array(&sp["participants"]) {
            let st = seat(ph, swarm, theme);
            out.shadows.push(Shadow {
                id: ShadowId::from_u128(ident(s(ph, "key"))),
                name: st.name,
                harness: st.harness,
                role: st.role,
                activity: st.activity,
                grade: st.grade,
                owner: s(ph, "owner").into(),
            });
        }
        swarms.push(out);
    }
    let mut model = Model::new(swarms, seats.into_values().collect())?;
    model.window.theme = theme;
    model.window.sidebar.open = array(&data["spaces"])
        .iter()
        .filter(|sp| contains(&opts["openSpaces"], s(sp, "key")))
        .map(|sp| SwarmId::from_u128(ident(s(sp, "key"))))
        .collect();
    let focused = SeatId::from_u128(ident(s(opts, "focusedKey")));
    if let Some((si, ti)) = model.tab_of(focused) {
        model.window.stage = Stage {
            swarm: Some(model.window.sidebar.swarms[si].id),
            tab: Some(model.window.sidebar.swarms[si].tabs[ti].id),
            focused: Some(focused),
        };
    }
    Ok(model)
}
fn mini(
    v: &Value,
    r: Rect,
    focus: &str,
    big: bool,
    ctx: &DrawCtx,
    out: &mut DrawList,
) -> Result<()> {
    crate::tabs::paint_mini_model(v, r, focus, big, ctx, out)
}
fn tab(
    v: &Value,
    r: Rect,
    expanded: bool,
    focused: &str,
    ctx: &DrawCtx,
    out: &mut DrawList,
) -> Result<()> {
    crate::tabs::paint_tab_model(v, r, expanded, focused, ctx, out)
}

fn sidebar(call: &Call, ctx: &DrawCtx, out: &mut DrawList) -> Result<()> {
    crate::swarms::paint_sidebar_model(
        call.args.first().ok_or("sidebar model missing")?,
        call.args.get(1).unwrap_or(&Value::Null),
        call.rect.rect(),
        ctx,
        out,
    )
}

fn card(r: Rect, title: &str, rows: &[String], ctx: &DrawCtx, out: &mut DrawList) {
    panel(out, r, ctx);
    text(
        out,
        Rect::new(r.x + 16., r.y + 12., r.w - 32., 22.),
        title,
        16.,
        ctx.fg(),
        500,
    );
    for (i, line) in rows.iter().enumerate() {
        let y = r.y + 46. + i as f32 * 26.;
        if y + 20. <= r.bottom() {
            text(
                out,
                Rect::new(r.x + 16., y, r.w - 32., 20.),
                line.as_str(),
                12.,
                ctx.muted(),
                400,
            );
        }
    }
}
fn graph(v: &Value, r: Rect, ctx: &DrawCtx, out: &mut DrawList) {
    out.rect(r, 0., Color::hex(ctx.theme.palette().bg));
    let mut y = r.y + 14.;
    let x = r.x + 16.;
    let w = r.w - 32.;
    let agents: Vec<_> = array(&v["seats"])
        .iter()
        .map(|st| {
            format!(
                "{}   {}   {}",
                st["name"].as_str().unwrap_or(""),
                st["role"].as_str().unwrap_or(""),
                array(&st["tasks"])
                    .iter()
                    .map(|t| format!("{} · {}", s(t, "title"), s(t, "state")))
                    .collect::<Vec<_>>()
                    .join(" · ")
            )
        })
        .collect();
    let h = 66. + agents.len() as f32 * 26.;
    card(Rect::new(x, y, w, h), "Agents", &agents, ctx, out);
    y += h + 14.;
    let mut work = Vec::new();
    for e in array(&v["epics"]) {
        work.push(format!("EPIC  {}", s(e, "title")));
        for t in array(&e["items"]) {
            work.push(format!(
                "    {}   {}   {}",
                s(t, "state"),
                s(t, "title"),
                s(t, "assignee")
            ));
            for a in array(&t["artifacts"]) {
                work.push(format!(
                    "        {}   {}   {}",
                    s(a, "kind"),
                    s(a, "ref"),
                    s(a, "digest")
                ));
            }
        }
    }
    let h = 66. + work.len() as f32 * 26.;
    card(Rect::new(x, y, w, h), "Work tree", &work, ctx, out);
    y += h + 14.;
    if !v["subscriptions"].is_null() {
        let rows = subscriptions(&v["subscriptions"]);
        let h = 66. + rows.len() as f32 * 26.;
        card(Rect::new(x, y, w, h), "Subscriptions", &rows, ctx, out);
        y += h + 14.;
    }
    if !v["provenance"].is_null() {
        let rows = provenance(&v["provenance"]);
        let h = 66. + rows.len() as f32 * 26.;
        card(Rect::new(x, y, w, h), "Provenance", &rows, ctx, out);
        y += h + 14.;
    }
    let rows: Vec<_> = array(&v["history"])
        .iter()
        .map(|a| {
            format!(
                "{}   {}   {}   {}   {}",
                s(a, "time"),
                s(a, "seat"),
                s(a, "event"),
                s(a, "ref"),
                s(a, "grade")
            )
        })
        .collect();
    card(
        Rect::new(x, y, w, 66. + rows.len() as f32 * 26.),
        "Artifact history",
        &rows,
        ctx,
        out,
    );
}
fn subscriptions(v: &Value) -> Vec<String> {
    array(v)
        .iter()
        .map(|r| {
            format!(
                "{}   {}",
                s(r, "seat"),
                array(&r["patterns"])
                    .iter()
                    .map(|p| format!(
                        "{}{}",
                        s(p, "pattern"),
                        if p["auto"] == true { "  AUTO" } else { "" }
                    ))
                    .collect::<Vec<_>>()
                    .join("   ")
            )
        })
        .collect()
}
fn provenance(v: &Value) -> Vec<String> {
    array(&v["items"])
        .iter()
        .flat_map(|i| {
            [
                format!(
                    "{}  {}  created by {} · updated by {}",
                    s(i, "type"),
                    s(i, "title"),
                    s(i, "created_by"),
                    s(i, "updated_by")
                ),
                s(i, "derived_from").into(),
                format!("{}  {}", s(&i["digest"], "content"), s(&i["digest"], "by")),
            ]
        })
        .chain(array(&v["log"]).iter().map(|l| {
            format!(
                "{}   {}   {}{}",
                s(l, "hash"),
                s(l, "time"),
                s(l, "text"),
                if l["checkpoint"] == true {
                    "signed daily checkpoint"
                } else {
                    ""
                }
            )
        }))
        .collect()
}

pub fn draw(case: &Case, ctx: &DrawCtx, out: &mut DrawList) -> Result<bool> {
    let Some(value) = &case.fixture else {
        return Ok(false);
    };
    let capture: Capture = serde_json::from_value(value.clone())?;
    if capture.version != 1 {
        return Err("unsupported web fixture version".into());
    }
    let mut handled = false;
    if case.component == "sidebar" {
        let phone = case.state == "phone sheet";
        let frame = Rect::new(
            if phone {
                (case.width as f32 - 320.) / 2.
            } else {
                14.
            },
            14.,
            if phone { 320. } else { case.width as f32 - 28. },
            case.height as f32 - 28.,
        );
        out.push(Prim::Rect {
            rect: frame,
            radii: [if phone { 28. } else { 0. }; 4],
            fill: Color::hex(ctx.theme.palette().navy),
            stroke: Some((1., Color::hex(ctx.theme.palette().line))),
            inner_highlight: None,
            shadow: None,
        });
    }
    if case.component.starts_with("showcase") {
        titlebar(
            Rect::new(0., 0., case.width as f32, STYLE.spacing.title_bar as f32),
            ctx,
            out,
        );
    }
    let names: &[&str] = match case.component.as_str() {
        "sidebar" | "showcase-3" | "showcase-15" => &["sidebar", "pane"],
        "seat_row" => &["seatRow"],
        "tab_row" => &["tabRow"],
        "mini_grid" => &["miniGrid"],
        "pane" | "title_strip" => &["pane", "seatRow"],
        "graph_tab" => &["graphPage"],
        "swarm_card" => &["sidebar"],
        "provenance_card" => &["provenanceCard"],
        "subscriptions_card" => &["subscriptionsCard"],
        _ => &[],
    };
    let dim = if case.state == "unfocused" {
        ctx.dimmed()
    } else {
        *ctx
    };
    for call in capture
        .calls
        .iter()
        .filter(|c| names.contains(&c.name.as_str()))
    {
        let v = call.args.first().unwrap_or(&Value::Null);
        let opts = call.args.get(1).unwrap_or(&Value::Null);
        let r = call.rect.rect();
        match call.name.as_str() {
            "sidebar" => sidebar(call, &dim, out)?,
            "pane" => pane(
                &seat(v, SwarmId::from_u128(1), ctx.theme),
                r,
                opts["focused"] == true,
                &dim,
                out,
            ),
            "spaceHeader" => crate::swarms::paint_header_model(v, opts, r, &dim, out)?,
            "seatRow" => seat_row(
                &seat(v, SwarmId::from_u128(1), ctx.theme),
                r,
                opts["focused"] == true,
                s(opts, "shortcut").trim_start_matches('⌘').parse().ok(),
                &dim,
                out,
            ),
            "tabRow" => tab(
                v,
                r,
                opts["expanded"] == true,
                s(opts, "focusedKey"),
                &dim,
                out,
            )?,
            "miniGrid" => mini(
                v,
                r,
                call.args.get(1).and_then(Value::as_str).unwrap_or(""),
                call.args.get(2).and_then(Value::as_bool).unwrap_or(false),
                &dim,
                out,
            )?,
            "graphPage" => {
                out.clip(r, [0.; 4]);
                graph(v, r, &dim, out);
                out.unclip();
            }
            "provenanceCard" => card(r, "Provenance", &provenance(v), &dim, out),
            "subscriptionsCard" => card(r, "Subscriptions", &subscriptions(v), &dim, out),
            _ => continue,
        }
        handled = true;
    }
    Ok(handled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    #[test]
    fn web_model_uses_real_keys_trees_ownership_and_observation_axes() {
        let data = serde_json::json!({"spaces":[{"key":"space0","name":"term","tabs":[{"name":"gpu","tree":{"dir":"v","a":{"key":"a"},"b":{"key":"b"}},"seats":[{"key":"a","name":"Nyx","harness":"codex","state":"waiting","grade":"shim","waitKind":"auth","where":"ssh","host":"rig"},{"key":"b","name":"Rook","harness":"codex","state":"active"}]}]}]});
        let opts =
            serde_json::json!({"openSpaces":["space0"],"openTabs":["space0/gpu"],"focusedKey":"b"});
        let m = model(&data, &opts, Theme::Dark).unwrap();
        assert_eq!(m.seats.len(), 2);
        assert_eq!(m.window.stage.focused, Some(SeatId::from_u128(ident("b"))));
        let a = &m.seats[&SeatId::from_u128(ident("a"))];
        assert_eq!(a.grade, Grade::Shim);
        assert_eq!(a.activity, Activity::Waiting(WaitKind::Auth));
        assert_eq!(a.where_, Where::Ssh { host: "rig".into() });
        assert!(m.window.sidebar.swarms[0].tabs[0].expanded);
        assert_eq!(
            m.window.sidebar.swarms[0].tabs[0]
                .tree
                .leaves()
                .into_iter()
                .collect::<BTreeSet<_>>(),
            m.seats.keys().copied().collect()
        );
    }
}
