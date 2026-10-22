use bbt_render::chrome::ChromeRenderer;
use bbt_ui_model::{
    draw::{Color, DrawList, Prim},
    layout::Rect,
};

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
