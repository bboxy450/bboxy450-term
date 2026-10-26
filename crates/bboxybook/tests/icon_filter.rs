use bbt_render::chrome::{ChromeRenderer, icons::Image};
use bbt_ui_model::{
    draw::{Color, DrawList, Prim},
    layout::Rect,
};

// keeps: spec renderer.md §4.4b
#[test]
fn icon_filter_interpolates_coverage_without_hidden_colour_or_atlas_neighbour_bleed() {
    // Transparent red beside opaque blue: halfway must be half-blue, never purple.
    let mut renderer = ChromeRenderer::new(vec![
        Image {
            name: "edge".into(),
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 0, 0, 0, 255, 255],
        },
        Image {
            name: "neighbour".into(),
            width: 2,
            height: 1,
            rgba: vec![0, 255, 0, 255, 0, 255, 0, 255],
        },
    ])
    .unwrap();
    let mut list = DrawList::default();
    list.rect(Rect::new(0., 0., 16., 16.), 0., Color::BLACK);
    list.push(Prim::Image {
        rect: Rect::new(0.5, 2.5, 8., 8.),
        image: "edge".into(),
        radius: 0.,
    });
    let result = renderer.render_to_rgba(&list, 16, 16, 1.).unwrap();
    let pixel = |x: usize, y: usize| &result[(y * 16 + x) * 4..][..4];
    let middle = pixel(4, 6);
    assert_eq!(&middle[..2], &[0, 0]);
    assert!(
        (126..=129).contains(&middle[2]),
        "expected interpolated half-blue, got {middle:?}"
    );
    assert_eq!(middle[3], 255);
    let edge = pixel(7, 6);
    assert_eq!(
        &edge[..3],
        &[0, 0, 255],
        "clamp within each asset; adjacent atlas green must not bleed"
    );
}
