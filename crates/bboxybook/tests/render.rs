use bboxybook::{Result, gallery};
use bbt_render::chrome::ChromeRenderer;
// keeps: spec renderer.md §4.5
#[test]
fn every_registered_component_renders_on_metal() -> Result<()> {
    let mut renderer = ChromeRenderer::new(bboxybook::images()?)?;
    let cases = gallery::cases();
    let mut rendered = 0;
    for entry in gallery::entries() {
        let case = cases
            .iter()
            .find(|c| c.component == entry.id && c.density == "cozy" && c.theme == "dark")
            .unwrap();
        let list = gallery::draw(case)?;
        let rgba = renderer.render_to_rgba(&list, case.width, case.height, 1.0)?;
        assert_eq!(rgba.len(), (case.width * case.height * 4) as usize);
        assert!(
            rgba.as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[0] != rgba[0] || p[1] != rgba[1] || p[2] != rgba[2]),
            "{} rendered only ground",
            entry.id
        );
        rendered += 1;
    }
    eprintln!(
        "{rendered} registered component previews rendered on {:?}",
        renderer.context.adapter_info
    );
    Ok(())
}
