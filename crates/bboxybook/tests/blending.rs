use bbt_render::chrome::ChromeRenderer;
use bbt_ui_model::{
    draw::{Color, DrawList, TextStyle, TypeScale},
    layout::Rect,
};

// keeps: spec renderer.md §1a
#[test]
fn chrome_blends_in_gamma_while_text_keeps_corrected_linear_coverage() {
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let mut list = DrawList::default();
    list.rect(Rect::new(0., 0., 200., 100.), 0., Color::BLACK);
    list.rect(Rect::new(0., 0., 20., 20.), 0., Color::WHITE.alpha(0.5));
    list.text(
        Rect::new(30., 10., 150., 80.),
        "H",
        TextStyle {
            scale: TypeScale::Custom(64.),
            weight: 700,
            ..Default::default()
        },
        Color::hex("#808080").alpha(0.5),
    );
    let image = renderer.render_to_rgba(&list, 200, 100, 1.).unwrap();
    let p = &image[(10 * 200 + 10) * 4..][..4];
    assert_eq!(
        p,
        &[128, 128, 128, 255],
        "CSS 50% white over black is 128, not linear-light 188"
    );
    let text_max = (10..90)
        .flat_map(|y| (30..180).map(move |x| (y * 200 + x) * 4))
        .map(|i| image[i])
        .max()
        .unwrap();
    assert!(
        (62..=65).contains(&text_max),
        "linear text coverage must be corrected to native weight, got {text_max}"
    );
    // A chrome primitive after the text must retain painter order across views.
    list.rect(Rect::new(30., 10., 150., 80.), 0., Color::hex("#102040"));
    let image = renderer.render_to_rgba(&list, 200, 100, 1.).unwrap();
    assert_eq!(&image[(40 * 200 + 50) * 4..][..4], &[16, 32, 64, 255]);
    // Captured CSS background edges snap to device pixels before corner AA.
    list.rect(Rect::new(0., 25.28125, 20., 10.), 0., Color::WHITE);
    let image = renderer.render_to_rgba(&list, 200, 100, 1.).unwrap();
    assert_eq!(&image[(25 * 200 + 10) * 4..][..4], &[255, 255, 255, 255]);
    assert_eq!(&image[(24 * 200 + 10) * 4..][..4], &[0, 0, 0, 255]);
}
