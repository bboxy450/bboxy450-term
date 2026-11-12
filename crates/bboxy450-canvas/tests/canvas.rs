// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxy450_canvas::*;
use sha2::{Digest, Sha256};
fn fixture() -> Vec<Op> {
    load_log(include_bytes!("fixtures/c0.log.jsonl").as_slice()).unwrap()
}
fn op(seq: u64, action: Action) -> Op {
    Op {
        author: "seat:canvas-c0".into(),
        seq,
        at: 0,
        action,
    }
}
fn element(id: &str) -> Element {
    Element {
        id: id.into(),
        tags: vec![],
        kind: None,
        emphasis: false,
        status: Status::Normal,
        style: None,
        content: Content::Shape {
            bounds: Bounds {
                origin: [0.; 2],
                size: [10.; 2],
            },
            shape: Shape::Box,
            label: id.into(),
        },
    }
}
// State: empty id map and next sequence=1, one owner. Each op is one atomic
// action. Safety: strict sequence, existing references, bounded elements, stable
// digest. Finite replay terminates without a scheduler/fairness assumption.
#[test]
fn canvas_replay_jsonl_and_record_roundtrip() {
    let ops = fixture();
    let a = Canvas::replay(&ops).unwrap();
    assert_eq!(a.elements().count(), 13);
    assert_eq!(a.digest(), Canvas::replay(&ops).unwrap().digest());
    assert_eq!(a.get("frame-right").unwrap().tags, ["conversation"]);
    let mut bytes = vec![];
    save_log(&ops, &mut bytes).unwrap();
    assert_eq!(ops, load_log(bytes.as_slice()).unwrap());
    let record = CanvasRecord {
        title: "C0".into(),
        swarm: "swarm".into(),
        tags: vec!["tag".into()],
        creator: "seat".into(),
        task: Some("task".into()),
    };
    assert_eq!(
        record,
        serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap()
    );
    assert!(
        load_log(b"\nnot-json\n".as_slice())
            .unwrap_err()
            .to_string()
            .starts_with("line 2:")
    );
}
#[test]
fn canvas_rejects_duplicate_reordered_missing_and_empty_author_ops() {
    let ops = fixture();
    let mut duplicate = ops.clone();
    duplicate.insert(1, ops[0].clone());
    assert_eq!(
        Canvas::replay(&duplicate),
        Err(ReplayError::Sequence {
            expected: 2,
            actual: 1
        })
    );
    let mut reordered = ops.clone();
    reordered.swap(0, 1);
    assert_eq!(
        Canvas::replay(&reordered),
        Err(ReplayError::Sequence {
            expected: 1,
            actual: 2
        })
    );
    let mut missing = ops.clone();
    missing.remove(1);
    assert!(matches!(
        Canvas::replay(&missing),
        Err(ReplayError::Sequence { .. })
    ));
    let mut author = ops.clone();
    author[1].author = "another-seat".into();
    assert_eq!(
        Canvas::replay(&author).unwrap().digest(),
        Canvas::replay(&ops).unwrap().digest()
    );
    author[1].author.clear();
    assert_eq!(Canvas::replay(&author), Err(ReplayError::Author));
    // All permutations of three additions: exactly the original sequence wins.
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        assert_eq!(
            Canvas::replay(&order.map(|i| ops[i].clone())).is_ok(),
            order == [0, 1, 2]
        );
    }
}
#[test]
fn canvas_canonical_digest_and_broken_unsorted_control() {
    let first = vec![
        op(
            1,
            Action::Add {
                element: element("z"),
            },
        ),
        op(
            2,
            Action::Add {
                element: element("a"),
            },
        ),
    ];
    let reversed = vec![
        op(1, first[1].action.clone()),
        op(2, first[0].action.clone()),
    ];
    let canvas = Canvas::replay(&first).unwrap();
    assert_eq!(canvas.digest(), Canvas::replay(&reversed).unwrap().digest());
    // Broken control: serialize the very same JSON object in insertion order,
    // bypassing BTreeMap canonical order. Same fields, changed ordering only.
    let unsorted = format!(
        "{{\"z\":{},\"a\":{}}}",
        serde_json::to_string(canvas.get("z").unwrap()).unwrap(),
        serde_json::to_string(canvas.get("a").unwrap()).unwrap()
    );
    let broken_digest = format!("{:x}", Sha256::digest(unsorted));
    assert_ne!(
        canvas.digest(),
        broken_digest,
        "unsorted control must be detected"
    );
    let mut tag_ops = first;
    tag_ops.push(op(
        3,
        Action::Tag {
            id: "a".into(),
            tag: "x".into(),
        },
    ));
    tag_ops.push(op(
        4,
        Action::Tag {
            id: "a".into(),
            tag: "x".into(),
        },
    ));
    assert_eq!(
        Canvas::replay(&tag_ops).unwrap().get("a").unwrap().tags,
        ["x"]
    );
}
#[test]
fn canvas_references_changes_removal_invalid_geometry_and_cap() {
    for action in [
        Action::Change {
            element: element("missing"),
        },
        Action::Remove {
            id: "missing".into(),
        },
        Action::Tag {
            id: "missing".into(),
            tag: "x".into(),
        },
    ] {
        assert_eq!(
            Canvas::replay(&[op(1, action)]),
            Err(ReplayError::UnknownId("missing".into()))
        );
    }
    for target in ["edge0", "frame-left"] {
        let mut refs = fixture();
        for op in &mut refs {
            if let Action::Add { element } = &mut op.action
                && element.id == target
            {
                match &mut element.content {
                    Content::Connector { from, .. } => *from = "missing".into(),
                    Content::Frame { members, .. } => members.push("missing".into()),
                    _ => unreachable!(),
                }
            }
        }
        assert_eq!(
            Canvas::replay(&refs),
            Err(ReplayError::UnknownId("missing".into()))
        );
    }
    let mut ops = fixture();
    let count = ops.len() as u64;
    ops.push(op(count + 1, Action::Remove { id: "box".into() }));
    assert_eq!(
        Canvas::replay(&ops),
        Err(ReplayError::ReferencedId("box".into()))
    );
    ops.pop();
    let mut changed = element("box");
    changed.tags = vec!["z".into(), "a".into(), "z".into()];
    ops.push(op(count + 1, Action::Change { element: changed }));
    assert_eq!(
        Canvas::replay(&ops).unwrap().get("box").unwrap().tags,
        ["a", "z"]
    );
    let add = op(
        1,
        Action::Add {
            element: element("a"),
        },
    );
    assert_eq!(
        Canvas::replay(&[add.clone(), op(2, add.action.clone())]),
        Err(ReplayError::DuplicateId("a".into()))
    );
    assert_eq!(
        Canvas::replay(&[add, op(2, Action::Remove { id: "a".into() })])
            .unwrap()
            .digest(),
        Canvas::default().digest()
    );
    let mut bad = element("bad");
    bad.content = Content::Shape {
        bounds: Bounds {
            origin: [f64::NAN, 0.],
            size: [1.; 2],
        },
        shape: Shape::Box,
        label: String::new(),
    };
    assert_eq!(
        Canvas::replay(&[op(1, Action::Add { element: bad })]),
        Err(ReplayError::InvalidElement("bad".into()))
    );
    let mut ops: Vec<_> = (0..MAX_ELEMENTS)
        .map(|i| {
            op(
                i as u64 + 1,
                Action::Add {
                    element: element(&i.to_string()),
                },
            )
        })
        .collect();
    assert_eq!(
        Canvas::replay(&ops).unwrap().elements().count(),
        MAX_ELEMENTS
    );
    ops.push(op(
        MAX_ELEMENTS as u64 + 1,
        Action::Add {
            element: element("over-cap"),
        },
    ));
    assert_eq!(
        Canvas::replay(&ops),
        Err(ReplayError::ElementCap { cap: MAX_ELEMENTS })
    );
}
#[test]
fn canvas_camera_roundtrips_extreme_zoom_and_large_centers() {
    for center in [[0.; 2], [1e12, -1e12]] {
        for zoom in [1e-9, 1., 1e9] {
            let camera = Camera { center, zoom };
            let world = [center[0] + 123.25, center[1] - 456.5];
            let restored = camera
                .screen_to_world(camera.world_to_screen(world).unwrap())
                .unwrap();
            for i in 0..2 {
                assert!((world[i] - restored[i]).abs() <= 1e-3, "{camera:?}");
            }
            // A nearby point retains fractional detail because subtraction
            // precedes the renderer's f32 conversion (a broken absolute cast loses it).
            let screen = camera
                .world_to_screen([center[0] + 0.25, center[1]])
                .unwrap();
            assert!((screen[0] / zoom - 0.25).abs() < 1e-12);
        }
    }
    let mut camera = Camera::default();
    let anchor = camera.screen_to_world([20., -30.]).unwrap();
    camera.zoom_at(1e9, [20., -30.]).unwrap();
    let after = camera.screen_to_world([20., -30.]).unwrap();
    assert_eq!(anchor, after);
    camera.zoom_at(1e-9, [20., -30.]).unwrap();
    assert!((camera.zoom - 1.).abs() < f64::EPSILON);
    assert_eq!(anchor, camera.screen_to_world([20., -30.]).unwrap());
    camera.zoom_at(1e9, [20., -30.]).unwrap();
    camera.pan([5., -10.]).unwrap();
    assert!((camera.world_to_screen(anchor).unwrap()[0] - 25.).abs() < 1e-5);
    let before = camera;
    for factor in [0., -1., f64::INFINITY, f64::NAN] {
        assert!(camera.zoom_at(factor, [0.; 2]).is_err());
        assert_eq!(before, camera);
    }
    assert!(camera.pan([f64::NAN, 0.]).is_err());
    assert_eq!(camera, before);
    assert!(
        Camera {
            center: [0.; 2],
            zoom: 0.
        }
        .world_to_screen([0.; 2])
        .is_err()
    );
}
#[test]
fn canvas_camera_fit_content_and_invalid_viewports() {
    let bounds = [
        Bounds {
            origin: [-100., -20.],
            size: [50., 40.],
        },
        Bounds {
            origin: [150., 100.],
            size: [50., 80.],
        },
    ];
    let camera = Camera::fit(bounds, [640., 480.], 20.).unwrap();
    assert_eq!(camera.center, [50., 80.]);
    assert_eq!(camera.zoom, 2.);
    for point in [[-100., -20.], [200., 180.]] {
        let screen = camera.world_to_screen(point).unwrap();
        assert!(screen[0].abs() <= 300. && screen[1].abs() <= 220.);
    }
    assert_eq!(Camera::fit([], [640., 480.], 20.), Ok(Camera::default()));
    for viewport in [[0., 480.], [640., f64::NAN], [40., 480.]] {
        assert!(Camera::fit(bounds, viewport, 20.).is_err());
    }
    for padding in [-1., f64::INFINITY, f64::NAN] {
        assert!(Camera::fit(bounds, [640., 480.], padding).is_err());
    }
    for size in [[0., 10.], [f64::INFINITY, 10.]] {
        assert!(
            Camera::fit(
                [Bounds {
                    origin: [0.; 2],
                    size
                }],
                [640., 480.],
                20.
            )
            .is_err()
        );
    }
    // Finite bounds whose union overflows still cannot manufacture an invalid camera.
    assert!(
        Camera::fit(
            [
                Bounds {
                    origin: [-f64::MAX, 0.],
                    size: [1., 1.]
                },
                Bounds {
                    origin: [f64::MAX / 2., 0.],
                    size: [1., 1.]
                },
            ],
            [640., 480.],
            0.
        )
        .is_err()
    );
}
#[test]
fn canvas_semantic_zoom_thresholds() {
    for size in [0., 0.999, f64::NAN] {
        assert_eq!(semantic_zoom(size), Detail::Culled);
    }
    for size in [1., 12., 23.999] {
        assert_eq!(semantic_zoom(size), Detail::Collapsed);
    }
    for size in [24., 1000.] {
        assert_eq!(semantic_zoom(size), Detail::Full);
    }
}
// State: a shape, a path and a frame; actions patch one field at a time.
// Safety: a patch never changes identity or references, an invalid style or
// geometry is refused whole, and a path's bounds are the box of its points.
#[test]
fn canvas_patch_style_path_and_extent() {
    let mut ops = vec![op(
        1,
        Action::Add {
            element: element("a"),
        },
    )];
    let mut path = element("mark");
    path.content = Content::Path {
        points: vec![[100., 100.], [140., 100.]],
        closed: false,
        smooth: true,
        arrowheads: [false, true],
    };
    ops.push(op(2, Action::Add { element: path }));
    let canvas = Canvas::replay(&ops).unwrap();
    // A horizontal stroke still has a positive extent.
    assert_eq!(
        canvas.get("mark").unwrap().content.bounds(),
        Some(Bounds {
            origin: [100., 100.],
            size: [40., 1.]
        })
    );
    assert_eq!(
        canvas.extent(),
        Some(Bounds {
            origin: [0., 0.],
            size: [140., 101.]
        })
    );
    assert_eq!(
        canvas.extent_of(["a", "missing"]),
        canvas.get("a").unwrap().content.bounds()
    );
    let before = canvas.digest();
    ops.push(op(
        3,
        Action::Patch {
            id: "a".into(),
            bounds: Some(Bounds {
                origin: [10., 10.],
                size: [20., 20.],
            }),
            label: Some("Renamed".into()),
            style: Some(Style {
                fill: Some("#123456".into()),
                glow: Some("accent".into()),
                ..Default::default()
            }),
            status: Some(Status::Done),
            emphasis: Some(true),
        },
    ));
    // A path moves and scales with its box.
    ops.push(op(
        4,
        Action::Patch {
            id: "mark".into(),
            bounds: Some(Bounds {
                origin: [0., 0.],
                size: [80., 1.],
            }),
            label: None,
            style: None,
            status: None,
            emphasis: None,
        },
    ));
    let patched = Canvas::replay(&ops).unwrap();
    let a = patched.get("a").unwrap();
    assert_eq!(a.content.label(), Some("Renamed"));
    assert_eq!(a.content.bounds().unwrap().origin, [10., 10.]);
    assert_eq!(a.status, Status::Done);
    assert!(a.emphasis);
    assert_eq!(a.style.as_ref().unwrap().fill.as_deref(), Some("#123456"));
    assert!(
        matches!(&patched.get("mark").unwrap().content, Content::Path { points, .. } if points == &vec![[0., 0.], [80., 0.]])
    );
    assert_ne!(patched.digest(), before);
    // An empty style clears it, so the digest returns to the unstyled form.
    ops.push(op(
        5,
        Action::Patch {
            id: "a".into(),
            bounds: None,
            label: None,
            style: Some(Style::default()),
            status: None,
            emphasis: None,
        },
    ));
    assert!(
        Canvas::replay(&ops)
            .unwrap()
            .get("a")
            .unwrap()
            .style
            .is_none()
    );
    for (id, bad) in [
        (
            "a",
            Style {
                fill: Some("blurple".into()),
                ..Default::default()
            },
        ),
        (
            "a",
            Style {
                opacity: Some(1.5),
                ..Default::default()
            },
        ),
        (
            "a",
            Style {
                weight: Some(50),
                ..Default::default()
            },
        ),
        (
            "a",
            Style {
                stroke_width: Some(f64::NAN),
                ..Default::default()
            },
        ),
    ] {
        let mut bad_ops = ops.clone();
        bad_ops.push(op(
            6,
            Action::Patch {
                id: id.into(),
                bounds: None,
                label: None,
                style: Some(bad),
                status: None,
                emphasis: None,
            },
        ));
        assert_eq!(
            Canvas::replay(&bad_ops).unwrap_err(),
            ReplayError::InvalidElement(id.into())
        );
    }
    let mut bad_ops = ops.clone();
    bad_ops.push(op(
        6,
        Action::Patch {
            id: "mark".into(),
            bounds: None,
            label: Some("no".into()),
            style: None,
            status: None,
            emphasis: None,
        },
    ));
    assert_eq!(
        Canvas::replay(&bad_ops).unwrap_err(),
        ReplayError::InvalidElement("mark".into())
    );
    let mut bad_ops = ops.clone();
    bad_ops.push(op(
        6,
        Action::Patch {
            id: "ghost".into(),
            bounds: None,
            label: None,
            style: None,
            status: None,
            emphasis: None,
        },
    ));
    assert_eq!(
        Canvas::replay(&bad_ops).unwrap_err(),
        ReplayError::UnknownId("ghost".into())
    );
    let mut one_point = element("dot");
    one_point.content = Content::Path {
        points: vec![[0., 0.]],
        closed: false,
        smooth: false,
        arrowheads: [false; 2],
    };
    assert_eq!(
        Canvas::replay(&[op(1, Action::Add { element: one_point })]).unwrap_err(),
        ReplayError::InvalidElement("dot".into())
    );
    let mut bad_label_at = element("edge");
    bad_label_at.content = Content::Connector {
        from: "a".into(),
        to: "mark".into(),
        label: None,
        arrowheads: [false; 2],
        dashed: false,
        route: Route::Curved,
        from_side: Some(Side::Right),
        to_side: None,
        waypoints: vec![],
        label_at: Some(2.),
    };
    let mut edge_ops = ops.clone();
    edge_ops.push(op(
        6,
        Action::Add {
            element: bad_label_at.clone(),
        },
    ));
    assert_eq!(
        Canvas::replay(&edge_ops).unwrap_err(),
        ReplayError::InvalidElement("edge".into())
    );
    // Old logs without the new fields replay and digest exactly as before.
    let legacy = Canvas::replay(&fixture()).unwrap();
    let serialized = serde_json::to_string(legacy.get("edge0").unwrap()).unwrap();
    assert!(
        !serialized.contains("route") && !serialized.contains("style"),
        "{serialized}"
    );
    for color in ["#abc", "#aabbcc", "#aabbccdd", "accent", "pink"] {
        assert!(valid_color(color), "{color}");
    }
    for color in ["abc", "#ab", "#ggg", "Accent", ""] {
        assert!(!valid_color(color), "{color}");
    }
}
