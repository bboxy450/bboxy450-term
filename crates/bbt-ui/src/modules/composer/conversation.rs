use super::Composer;
use crate::{
    self as ui, components as c,
    modules::{seat_row::named, stream_dock::StreamDock, strip::plain},
};
use bbt_ui_model::{
    STYLE,
    layout::{Axis, Rect},
};

/// Shared inline stream surface. Only rows scroll; the composer stays at its
/// foot. The caller owns the selected stream, offset, focus and effects.
#[derive(Default)]
pub struct Conversation {
    pub stream: StreamDock,
    pub composer: Composer,
    pub scroll: f32,
}
impl Conversation {
    pub fn element(
        &self,
        ctx: ui::Context,
        size: [f32; 2],
        m: &dyn c::Measure,
    ) -> ui::style::Result<c::Element> {
        let sp = &STYLE.spacing;
        let inner = (size[0] - sp.s2 as f32 - sp.s3 as f32).max(1.);
        let composer = self.composer.element(ctx, inner, m)?;
        let h = composer
            .layout(Rect::new(0., 0., inner, 0.), ctx, m)?
            .rect
            .h;
        let height = size[1].max(h + 2. * sp.s3 as f32 + 1.);
        let rows_h = (height - 2. * sp.s3 as f32 - 1. - h).max(0.);
        let rows = c::Element::ScrollView(c::ScrollView {
            viewport: [inner, rows_h],
            offset: [0., self.scroll],
            content_width: Some(inner),
            content: Box::new(self.stream.content(m)),
        });
        let mut style = plain(
            Some(height),
            [sp.s3 as f32, sp.s3 as f32, sp.s3 as f32, sp.s2 as f32],
            0.,
        );
        style.overrides.width = Some(ui::Length::Px(size[0]));
        style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(
            bbt_ui_model::draw::Color::hex(ctx.theme.palette().sidebar).0,
        )));
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: false,
        });
        style.overrides.border_sides = Some([true, false, false, false]);
        let mut b = c::Box::new(
            style,
            vec![named("conversation/rows", rows).into(), composer.into()],
        );
        b.direction = Axis::Column;
        b.gap = ui::Length::Px(0.);
        b.align = c::Alignment::Stretch;
        Ok(named("conversation", c::Element::Box(b)))
    }
}
