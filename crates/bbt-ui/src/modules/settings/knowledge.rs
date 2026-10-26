//! Mounted-root presentation. Choosing a provider emits an intent; this module never opens one.
use super::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pending {
    pub to: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Root {
    pub name: String,
    pub path: String,
    pub provider: String,
    pub records: Option<u64>,
    pub answering: String,
    pub digests: Option<String>,
    pub refused: Option<String>,
    pub pending: Option<Pending>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Knowledge {
    pub roots: Vec<Root>,
    pub offered: Vec<String>,
}

impl Settings {
    pub(super) fn knowledge(&self, b: &Build) -> c::Element {
        let knowledge = &self.values.knowledge;
        let mut children = vec![b.note("knowledge/note", "A root is a place records live; each names the provider that reads it. Swapping the provider moves no file: the new one reads the same records, or refuses with its reason.", true).into()];
        for (i, root) in knowledge.roots.iter().enumerate() {
            let key = format!("card0/root{i}");
            let header = row(
                &format!("{key}/header"),
                plain(None, [0.; 4], 0.),
                8.,
                vec![
                    natural(icon(
                        &format!("{key}/icon"),
                        "book",
                        14.,
                        ui::Color::Foreground,
                    )),
                    natural(b.text(&format!("{key}/name"), &root.name, 10., 600, true, false)),
                    natural(b.text(&format!("{key}/path"), &root.path, 10., 400, true, true)),
                    fill(c::Element::Spacer),
                    natural(b.text(
                        &format!("{key}/records"),
                        root.records.map_or_else(
                            || "record count unavailable".into(),
                            |count| format!("{count} records"),
                        ),
                        10.,
                        400,
                        false,
                        true,
                    )),
                ],
            );
            let bundled = knowledge.offered.first() == Some(&root.provider);
            let picker = b.select(
                &format!("{key}/provider"),
                &knowledge.offered,
                &root.provider,
                false,
                false,
            );
            let picker = if root.pending.is_some() {
                faded(picker, 0.4)
            } else {
                picker
            };
            let hint = root.pending.as_ref().map_or_else(
                || {
                    if bundled {
                        "bundled, in process".into()
                    } else {
                        "a trusted plugin over stdio".into()
                    }
                },
                |pending| format!("swapping to {}…", pending.to),
            );
            let mut answering = vec![
                b.text(
                    &format!("{key}/answering-label"),
                    "answering: ",
                    10.,
                    400,
                    false,
                    true,
                )
                .into(),
                b.text(
                    &format!("{key}/answering-value"),
                    &root.answering,
                    10.,
                    600,
                    false,
                    false,
                )
                .into(),
            ];
            if let Some(digests) = &root.digests {
                answering.push(
                    b.text(
                        &format!("{key}/digests"),
                        format!(" · {digests}"),
                        10.,
                        400,
                        false,
                        true,
                    )
                    .into(),
                );
            }
            let mut parts = vec![
                header.into(),
                b.setting_row(
                    &format!("card0/r{i}"),
                    "Provider",
                    Some(&hint),
                    picker,
                    !bundled,
                    false,
                    false,
                )
                .into(),
                row(
                    &format!("{key}/answering"),
                    plain(None, [0., 0., 0., 4.], 0.),
                    0.,
                    answering,
                )
                .into(),
            ];
            if let Some(reason) = &root.refused {
                parts.push(plugins::alert(&format!("{key}/refused"), reason).into());
            }
            let mut style = border(plain(None, [12., 0., 14., 0.], 0.), hair(b.ctx), false);
            style.overrides.border_sides = Some([true, false, false, false]);
            children.push(col(&key, style, 0., parts).into());
        }
        if knowledge.roots.is_empty() {
            children.push(
                b.note(
                    "knowledge/empty",
                    "Knowledge roots are unavailable in the current runtime snapshot",
                    true,
                )
                .into(),
            );
        }
        children.push(b.note("knowledge/check", "A second provider is offered only after it passes bboxy450-term plugin check and the contract’s conformance fixtures.", true).into());
        b.card("card0", "Knowledge roots", children, [4., 14., 10., 14.])
    }
}
