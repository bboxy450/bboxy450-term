use bbt_render::chrome::ChromeRenderer;
use bbt_ui_model::{
    draw::{Color, DrawList, Prim},
    layout::Rect,
};

// keeps: spec renderer.md §4.5
#[test]
fn positioned_style_moves_only_its_shadow_in_the_existing_chrome_pass() {
    use bbt_ui::{self as ui, components as c};
    let ctx = ui::Context::new(ui::Theme::Light, ui::Density::Cozy);
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let mut style = ui::Style::new(ui::Preset::Row);
    style.overrides.height = Some(ui::Length::Px(20.));
    style.overrides.padding = Some([ui::Length::Px(0.); 4]);
    style.overrides.radii = Some([ui::Length::Px(0.); 4]);
    style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba([1., 0., 0., 1.])));
    style.overrides.shadow = Some(ui::Shadow::Positioned {
        offset: [0., 12.],
        blur: 4.,
        color: [0., 0., 0., 1.],
    });
    let layout = c::Element::Box(c::Box::new(style, vec![]))
        .layout(
            Rect::new(20., 20., 20., 0.),
            ctx,
            &bboxybook::components::Metrics,
        )
        .unwrap();
    let pixels = bboxybook::components::execute(
        &mut renderer,
        c::paint(&layout, ctx),
        [255, 255, 255, 255].repeat(80 * 80),
        [80, 80],
        ctx,
    )
    .unwrap();
    let pixel = |x: usize, y: usize| &pixels[(y * 80 + x) * 4..(y * 80 + x) * 4 + 4];
    assert_eq!(pixel(30, 30), [255, 0, 0, 255]);
    assert!(
        pixel(30, 47)[0] < 100,
        "shifted shadow below the unchanged fill"
    );
    assert_eq!(pixel(30, 13), [255, 255, 255, 255]);
}

// keeps: spec renderer.md §4.5
#[test]
fn css_shadow_uses_gaussian_mask_sigma_half_blur_and_knocks_out_the_box() {
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let mut list = DrawList::default();
    list.rect(Rect::new(0., 0., 180., 100.), 0., Color::WHITE);
    list.push(Prim::Rect {
        rect: Rect::new(40., 20., 80., 20.),
        radii: [0.; 4],
        fill: Color::TRANSPARENT,
        stroke: None,
        inner_highlight: None,
        shadow: Some((10., 20., Color::BLACK)),
    });
    let image = renderer.render_to_rgba(&list, 180, 100, 1.).unwrap();
    // Independent normal-CDF values from Python math.erf, for pixel centres.
    // CSS blur 20 -> sigma 10; the unshifted border-box is a knockout.
    for (x, y, expected) in [
        (40, 45, 174),
        (80, 45, 99),
        (80, 60, 218),
        (130, 45, 232),
        (80, 35, 255),
    ] {
        let pixel = &image[(y * 180 + x) * 4..][..4];
        for &v in &pixel[..3] {
            assert!(
                v.abs_diff(expected) <= 1,
                "({x},{y}) {pixel:?}, expected {expected}"
            );
        }
        assert_eq!(pixel[3], 255);
    }
}
