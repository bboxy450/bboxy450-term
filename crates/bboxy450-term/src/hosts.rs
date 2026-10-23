//! Pure host-to-view projection. L8 owns launcher/settings input and hosting.
use bbt_proto::{GuiHost, HostState, Where};
use bbt_ui::{
    self as ui, components as c,
    modules::{
        launcher::Item,
        link::{Cloud, LinkState, MachineState},
        seat_row::SeatRow,
        settings::Records,
        swarm_card::SwarmCard,
    },
};
use bbt_ui_model::{Activity, TOKENS, layout::Axis};

pub fn link(state: &HostState) -> LinkState {
    match state {
        HostState::Offline => LinkState::Offline,
        HostState::Connecting => LinkState::Connecting,
        HostState::Connected { rtt_ms } => LinkState::Connected { rtt_ms: *rtt_ms },
        HostState::Degraded { rtt_ms } => LinkState::Degraded { rtt_ms: *rtt_ms },
        HostState::Disconnected { reason } => LinkState::Disconnected {
            reason: reason.clone(),
        },
    }
}
pub fn key(location: &Where) -> String {
    match location {
        Where::Local => "local".into(),
        Where::Ssh { host } => format!("ssh:{host}"),
        Where::Cloud { provider, host } => format!("cloud:{}:{provider}{host}", provider.len()),
    }
}

/// Read-only module records projected from the current provider snapshot.
pub fn settings(hosts: &[GuiHost]) -> Records {
    let mut records = Records::default();
    let mut clouds: std::collections::BTreeMap<String, Vec<&GuiHost>> = Default::default();
    for host in hosts {
        match &host.location {
            Where::Local => {}
            Where::Ssh { host: name } => records.machines.push((
                host.label.clone(),
                host.address.clone().unwrap_or_else(|| name.clone()),
                String::new(),
                String::new(),
                MachineState::Link(link(&host.state)),
            )),
            Where::Cloud { provider, .. } => clouds.entry(provider.clone()).or_default().push(host),
        }
    }
    for (name, hosts) in clouds {
        let state = hosts
            .iter()
            .max_by_key(|host| match &host.state {
                HostState::Disconnected { .. } => (5, 0),
                HostState::Offline => (4, 0),
                HostState::Connecting => (3, 0),
                HostState::Degraded { rtt_ms } => (2, *rtt_ms),
                HostState::Connected { rtt_ms } => (1, *rtt_ms),
            })
            .map(|host| link(&host.state));
        let regions: std::collections::BTreeSet<_> =
            hosts.iter().filter_map(|h| h.region.as_deref()).collect();
        records.cloud.push(Cloud(
            name,
            regions.into_iter().collect::<Vec<_>>().join(" · "),
            true,
            "#2E6BFF".into(),
            state,
        ));
    }
    records
}

/// Host branches; L8 attaches its harness/placement choices to available hosts.
pub fn launcher_items(hosts: &[GuiHost]) -> Vec<Item> {
    let rows = |cloud: bool| {
        hosts
            .iter()
            .filter(|host| {
                matches!(host.location, Where::Cloud { .. }) == cloud
                    && !matches!(host.location, Where::Local)
            })
            .map(|host| {
                let state = link(&host.state);
                let down = state.down();
                Item {
                    id: key(&host.location),
                    label: host.label.clone(),
                    sub: host
                        .region
                        .clone()
                        .or(host.address.clone())
                        .unwrap_or_default(),
                    harness: if cloud { "cloud" } else { "ssh" }.into(),
                    kind: if cloud { "Region" } else { "Machine" }.into(),
                    link: Some(state),
                    down,
                    children: (!down).then(Vec::new),
                    ..Default::default()
                }
            })
            .collect()
    };
    vec![
        Item {
            id: "ssh".into(),
            label: "SSH".into(),
            sub: "pick a machine, then a harness".into(),
            harness: "ssh".into(),
            kind: "Remote".into(),
            section: Some("Remote".into()),
            children: Some(rows(false)),
            ..Default::default()
        },
        Item {
            id: "cloud".into(),
            label: "Cloud".into(),
            sub: "pick a region, then a harness".into(),
            harness: "cloud".into(),
            kind: "Remote".into(),
            section: Some("Remote".into()),
            children: Some(rows(true)),
            ..Default::default()
        },
    ]
}

fn label(text: impl Into<String>, size: f32, color: ui::Color) -> c::Element {
    let mut t = c::Text::new(
        text,
        ui::Text {
            size: Some(size),
            paint: ui::Paint::solid(color),
            line_height: 1.4,
            ..Default::default()
        },
    );
    t.wrap = false;
    t.ellipsis = true;
    c::Element::Text(t)
}
fn row(host: &GuiHost) -> c::Element {
    let mut style = ui::Style::new(ui::Preset::Card);
    style.overrides.padding = Some([6., 8., 6., 8.].map(ui::Length::Px));
    style.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
    style.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    let state = link(&host.state);
    let mut box_ = c::Box::new(
        style,
        vec![
            c::Element::Icon {
                name: if matches!(host.location, Where::Ssh { .. }) {
                    "ssh"
                } else {
                    "cloud"
                }
                .into(),
                size: ui::Length::Px(14.),
                paint: ui::Paint::solid(ui::Color::Muted),
            }
            .into(),
            c::Item {
                element: label(&host.label, 13., ui::Color::Foreground),
                extent: c::Extent::Fill,
            },
            c::Item {
                element: state.element(11., true),
                extent: c::Extent::Content {
                    grow: 0.,
                    shrink: 1.,
                    min: 0.,
                    max: None,
                    max_fraction: Some(0.55),
                },
            },
        ],
    );
    box_.direction = Axis::Row;
    box_.align = c::Alignment::Center;
    box_.gap = ui::Length::Px(6.);
    c::Element::Named {
        name: format!("provider/{}", key(&host.location)),
        content: Box::new(c::Element::Box(box_)),
    }
}
pub fn sidebar_cards(hosts: &[GuiHost]) -> Vec<SwarmCard> {
    let mut cards = Vec::new();
    for (cloud, name) in [(false, "Hosts"), (true, "Cloud")] {
        let hosts: Vec<_> = hosts
            .iter()
            .filter(|h| {
                matches!(h.location, Where::Cloud { .. }) == cloud
                    && !matches!(h.location, Where::Local)
            })
            .collect();
        if hosts.is_empty() {
            continue;
        }
        let seats = hosts
            .iter()
            .map(|host| {
                let mut seat = SeatRow::new(
                    key(&host.location),
                    &host.label,
                    if cloud { "cloud" } else { "ssh" },
                );
                seat.activity = match host.state {
                    HostState::Connected { .. } => Activity::Active,
                    HostState::Degraded { .. } => {
                        Activity::Waiting(bbt_ui_model::WaitKind::Question)
                    }
                    HostState::Disconnected { .. } | HostState::Offline => Activity::Errored,
                    _ => Activity::Idle,
                };
                seat
            })
            .collect();
        cards.push(SwarmCard {
            key: format!("providers/{name}"),
            name: name.into(),
            color: bbt_ui_model::draw::Color::hex(if cloud {
                TOKENS.color.dark.accent
            } else {
                "#374151"
            })
            .0,
            seats,
            expanded: true,
            closed: false,
            unfocused: false,
            tabs: vec![],
            body: true,
            new_seat: false,
            extra: hosts.iter().map(|host| row(host)).collect(),
            shadows: vec![],
        });
    }
    cards
}
