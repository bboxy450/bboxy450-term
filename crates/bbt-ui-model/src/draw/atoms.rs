use super::*;
use crate::{Activity, Grade, Role, WaitKind, Where, STYLE, TOKENS, layout::Rect};

pub trait Draw { fn draw(&self, rect: Rect, ctx: &DrawCtx, out: &mut DrawList); }
pub struct IconTile<'a> { pub icon: &'a str, pub color: Color }
impl Draw for IconTile<'_> {
    fn draw(&self, rect: Rect, ctx: &DrawCtx, out: &mut DrawList) {
        out.rect(rect, (rect.h * 0.25).min(STYLE.radius.tile as f32), ctx.color(self.color));
        let p = rect.h * 0.18;
        out.push(Prim::Icon { rect: Rect::new(rect.x+p,rect.y+p,rect.w-p*2.0,rect.h-p*2.0), icon: self.icon.into(), color: ctx.color(Color::WHITE) });
    }
}
pub struct Badge<'a> { pub label: &'a str, pub selected: bool, pub mono: bool }
impl Draw for Badge<'_> {
    fn draw(&self, rect: Rect, ctx: &DrawCtx, out: &mut DrawList) {
        let alpha = if self.selected { STYLE.tint.badge_selected_alpha } else { STYLE.tint.badge_alpha } as f32;
        out.rect(rect, STYLE.radius.badge as f32, ctx.color(Color::hex(ctx.theme.palette().fg_dim).alpha(alpha)));
        out.text(Rect::new(rect.x+4.0,rect.y+(rect.h-12.0)*0.5,rect.w-6.0,14.0), self.label, TextStyle { scale: TypeScale::Badge, mono: self.mono, ..Default::default() }, ctx.muted());
    }
}
pub struct StateGlyph { pub activity: Activity, pub grade: Grade, pub color: Color }
impl Draw for StateGlyph {
    fn draw(&self, r: Rect, ctx: &DrawCtx, out: &mut DrawList) {
        let color = match self.activity { Activity::Idle => Color::hex(TOKENS.color.state.idle), Activity::Waiting(_) => Color::hex(TOKENS.color.state.waiting), Activity::Errored => Color::hex(TOKENS.color.state.errored), Activity::Active => self.color };
        let color = ctx.color(color).alpha(if self.grade == Grade::Inferred { 0.72 } else { 1.0 });
        let inset = 2.0; let c = Rect::new(r.x+inset,r.y+inset,(r.w-inset*2.0).max(1.0),(r.h-inset*2.0).max(1.0));
        match self.activity {
            Activity::Idle => out.push(Prim::Rect { rect:c, radii:[c.h;4], fill:Color::TRANSPARENT, stroke:Some((1.25,color)), inner_highlight:None, shadow:None }),
            Activity::Errored => { out.line([c.x,c.y],[c.right(),c.bottom()],1.6,color); out.line([c.right(),c.y],[c.x,c.bottom()],1.6,color); },
            Activity::Waiting(_) => { out.rect(c,c.h,color.alpha(0.35)); out.rect(Rect::new(c.x,c.y,c.w*0.5,c.h),c.h*0.4,color); },
            Activity::Active => out.rect(c,c.h,color),
        }
        if self.grade == Grade::Shim { out.push(Prim::Rect { rect:r, radii:[r.h;4], fill:Color::TRANSPARENT, stroke:Some((1.0,color.alpha(0.55))), inner_highlight:None, shadow:None }); }
        if self.grade == Grade::Inferred {
            for i in 0..8 { let a = i as f32 * std::f32::consts::TAU / 8.0; let b=a+0.42; let rad=r.h*0.55;
                out.line([r.x+r.w*0.5+a.cos()*rad,r.y+r.h*0.5+a.sin()*rad],[r.x+r.w*0.5+b.cos()*rad,r.y+r.h*0.5+b.sin()*rad],1.0,color); }
        }
        if let Activity::Waiting(kind)=self.activity {
            let label = match kind { WaitKind::Permission => "⚿", WaitKind::Question => "?", WaitKind::Auth => "⚷" };
            out.text(Rect::new(r.right()-2.0,r.y-5.0,10.0,9.0),label,TextStyle { scale:TypeScale::Custom(7.0),weight:700,..Default::default() },color);
        }
    }
}
pub struct RoleMark<'a>(pub &'a Role);
impl Draw for RoleMark<'_> {
    fn draw(&self, r:Rect, ctx:&DrawCtx, out:&mut DrawList) {
        let (mark,color)=match self.0.name.as_str() { "oracle" => ("♔",TOKENS.color.role.oracle), "creativity" => ("♝",TOKENS.color.role.creativity), _ => ("♟",TOKENS.color.role.executor) };
        out.text(r,mark,TextStyle { scale:TypeScale::Custom(r.h.min(12.0)),..Default::default() },ctx.color(Color::hex(color)));
    }
}
pub struct WhereMark<'a>(pub &'a Where);
impl Draw for WhereMark<'_> {
    fn draw(&self,r:Rect,ctx:&DrawCtx,out:&mut DrawList) {
        let icon=match self.0 { Where::Local=>return,Where::Ssh{..}=>"ssh",Where::Cloud{..}=>"cloud" };
        out.push(Prim::Icon { rect:r,icon:icon.into(),color:ctx.muted() });
    }
}
pub struct Swatch(pub Color);
impl Draw for Swatch { fn draw(&self,r:Rect,ctx:&DrawCtx,out:&mut DrawList) { out.push(Prim::Rect { rect:r,radii:[r.h;4],fill:ctx.color(self.0),stroke:Some((1.5,ctx.color(Color::WHITE.alpha(0.85)))),inner_highlight:None,shadow:None }); } }
pub struct Chevron;
impl Draw for Chevron { fn draw(&self,r:Rect,ctx:&DrawCtx,out:&mut DrawList) { out.line([r.x+r.w*0.35,r.y+r.h*0.25],[r.x+r.w*0.65,r.y+r.h*0.5],1.3,ctx.muted());out.line([r.x+r.w*0.65,r.y+r.h*0.5],[r.x+r.w*0.35,r.y+r.h*0.75],1.3,ctx.muted()); } }
pub struct Pill<'a>(pub &'a [(&'a str,&'a str)]);
impl Draw for Pill<'_> {
    fn draw(&self,r:Rect,ctx:&DrawCtx,out:&mut DrawList) {
        out.push(Prim::Rect { rect:r,radii:[STYLE.radius.pill as f32;4],fill:ctx.color(Color::hex(ctx.theme.palette().surface).alpha(STYLE.material.panel_alpha as f32)),stroke:Some((1.0,ctx.hairline())),inner_highlight:Some(Color::WHITE.alpha(STYLE.material.inner_highlight_alpha as f32)),shadow:Some((6.0,10.0,Color::BLACK.alpha(0.18))) });
        let w=r.w/self.0.len().max(1) as f32;
        for (i,(label,key)) in self.0.iter().enumerate() {
            let x=r.x+i as f32*w;
            if i>0 { out.line([x,r.y+6.0],[x,r.bottom()-6.0],1.0,ctx.hairline()); }
            out.text(Rect::new(x+10.0,r.y+8.0,w-14.0,16.0),format!("{label}  {key}"),TextStyle { scale:TypeScale::Caption,..Default::default() },ctx.fg());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dim_desaturates_content_and_preserves_identity() {
        let ctx=DrawCtx::new(crate::Theme::Dark,crate::Density::Cozy).dimmed();
        for color in [Color::hex("#FF4FA3"),Color::hex("#2EE6A8"),Color::hex("#FFC53D")] {
            let dim=ctx.color(color); assert!(dim.to_hsl()[1]<=0.10001); assert_eq!(dim.0[3],0.45); assert_eq!(ctx.identity(color),color); assert_ne!(ctx.ground(color),color);
        }
    }
    #[test]
    fn glyphs_encode_grade_and_wait_kind_and_clips_balance() {
        let ctx=DrawCtx::new(crate::Theme::Dark,crate::Density::Cozy);
        let mut list=DrawList::default();
        StateGlyph { activity:Activity::Waiting(WaitKind::Auth),grade:Grade::Inferred,color:Color::WHITE }.draw(Rect::new(0.0,0.0,12.0,12.0),&ctx,&mut list);
        assert!(list.primitives.iter().filter(|p|matches!(p,Prim::Hairline{..})).count()>=8);
        assert!(list.primitives.iter().any(|p|matches!(p,Prim::Text{text,..} if text.as_ref()=="⚷")));
        list.clip(Rect::new(0.0,0.0,5.0,5.0),[0.0;4]);assert!(list.validate().is_err());list.unclip();assert!(list.validate().is_ok());list.unclip();assert!(list.validate().is_err());
    }
}
