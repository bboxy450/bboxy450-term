//! One ordered, instanced chrome pass on the caller's device; no window ownership.
pub mod icons;
pub mod diff;
pub mod pipeline;
pub mod text;
use crate::{Result,RenderError,atlas::{GlyphAtlas,AtlasRect},device::DeviceContext};
use bbt_ui_model::{draw::{DrawList,Prim,Color},layout::Rect};
use std::{collections::HashMap,sync::mpsc,time::Duration};
use wgpu::util::DeviceExt;
use pipeline::Instance;

pub struct ChromeRenderer {pub context:DeviceContext,atlas:GlyphAtlas,text:HashMap<String,(AtlasRect,f32)>,icons:icons::IconAtlas,pipelines:HashMap<wgpu::TextureFormat,wgpu::RenderPipeline>}
impl ChromeRenderer {
    pub fn new(images:Vec<icons::Image>)->Result<Self>{Self::with_context(pollster::block_on(DeviceContext::new())?,images)}
    pub fn with_context(context:DeviceContext,images:Vec<icons::Image>)->Result<Self>{Ok(Self{context,atlas:GlyphAtlas::new(512,8192)?,text:HashMap::new(),icons:icons::IconAtlas::new(images)?,pipelines:HashMap::new()})}
    pub fn draw(&mut self,list:&DrawList,width:u32,height:u32,scale:f32,target:&wgpu::TextureView,format:wgpu::TextureFormat,encoder:&mut wgpu::CommandEncoder)->Result<()> {
        list.validate().map_err(RenderError::from)?;
        if width==0||height==0||!scale.is_finite()||!(0.5..=4.0).contains(&scale){return Err("invalid chrome viewport".into());}
        let mut instances=Vec::new();let mut clips=vec![(Rect::new(0.0,0.0,width as f32/scale,height as f32/scale),[0.0;4])];
        let make=|r:Rect,c:Color,kind:f32,clip:(Rect,[f32;4])|Instance{rect:[r.x*scale,r.y*scale,r.w*scale,r.h*scale],fill:c.0,params:[kind,0.0,0.0,0.0],clip:[clip.0.x*scale,clip.0.y*scale,clip.0.w*scale,clip.0.h*scale],clip_radii:clip.1.map(|v|v*scale),..Default::default()};
        for prim in &list.primitives {let clip=*clips.last().unwrap();match prim {
            Prim::Clip{rect,radii}=>{let x=rect.x.max(clip.0.x);let y=rect.y.max(clip.0.y);clips.push((Rect::new(x,y,(rect.right().min(clip.0.right())-x).max(0.0),(rect.bottom().min(clip.0.bottom())-y).max(0.0)),*radii));},
            Prim::Unclip=>{clips.pop();},
            Prim::Rect{rect,radii,fill,stroke,inner_highlight,shadow}=>{
                if let Some((offset,blur,color))=shadow {let mut item=make(Rect::new(rect.x,rect.y+offset,rect.w,rect.h),*color,4.0,clip);item.radii=radii.map(|v|v*scale);item.params[2]=blur*scale;instances.push(item);}
                let mut item=make(*rect,*fill,0.0,clip);item.radii=radii.map(|v|v*scale);if let Some((width,color))=stroke {item.params[1]=width*scale;item.stroke=color.0;}if let Some(c)=inner_highlight {item.params[3]=c.0[3];}instances.push(item);
            },
            Prim::Gloss{rect,radii,color}=>{let mut item=make(*rect,*color,1.0,clip);item.radii=radii.map(|v|v*scale);item.params[3]=0.18;instances.push(item);},
            Prim::Hairline{from,to,width,color}=>{let pad=width*0.5;let r=Rect::new(from[0].min(to[0])-pad,from[1].min(to[1])-pad,(from[0]-to[0]).abs()+width,(from[1]-to[1]).abs()+width);let mut item=make(r,*color,5.0,clip);item.params[1]=width*scale;item.uv=[from[0]*scale,from[1]*scale,to[0]*scale,to[1]*scale];instances.push(item);},
            Prim::Text{origin,text,style,max_width,color}=>{
                if text.is_empty(){continue;}
                let key=format!("{text}\0{style:?}\0{max_width:?}\0{scale}");
                let (atlas,advance)=if let Some(cached)=self.text.get(&key){*cached}else{let line=text::rasterize(text,*style,*max_width,scale)?;let rect=self.atlas.insert(line.width,line.height,&line.mask)?;let value=(rect,line.advance);self.text.insert(key,value);value};
                if advance<=0.0 {continue;}
                let r=Rect::new(origin[0]-2.0/scale,origin[1]+(style.pixels()*1.4-atlas.height as f32/scale)*0.5,atlas.width as f32/scale,atlas.height as f32/scale);
                let mut item=make(r,*color,2.0,clip);item.uv=[atlas.x as f32,atlas.y as f32,atlas.width as f32,atlas.height as f32];instances.push(item);
            },
            Prim::Icon{rect,icon,color}=>{let uv=self.icons.rects.get(icon.as_ref()).ok_or_else(||RenderError(format!("unbaked icon: {icon}")))?;let mut item=make(*rect,*color,3.0,clip);item.uv=*uv;instances.push(item);},
            Prim::Image{rect,image,radius}=>{let uv=self.icons.rects.get(image.as_ref()).ok_or_else(||RenderError(format!("unbaked image: {image}")))?;let mut item=make(*rect,Color::WHITE,3.0,clip);item.uv=*uv;item.radii=[radius*scale;4];instances.push(item);},
        }}
        if instances.is_empty(){return Ok(());}
        if instances.len()>65_536{return Err("chrome instance budget exceeded".into());}
        let device=&self.context.device;let queue=&self.context.queue;
        let pipeline=self.pipelines.entry(format).or_insert_with(||pipeline::create(device,format));
        let uniform=device.create_buffer_init(&wgpu::util::BufferInitDescriptor{label:Some("chrome view"),contents:bytemuck::cast_slice(&[width as f32,height as f32,scale,0.0]),usage:wgpu::BufferUsages::UNIFORM});
        let items=device.create_buffer_init(&wgpu::util::BufferInitDescriptor{label:Some("retained chrome list"),contents:bytemuck::cast_slice(&instances),usage:wgpu::BufferUsages::STORAGE});
        let mask=upload(device,queue,self.atlas.size(),self.atlas.pixels(),wgpu::TextureFormat::R8Unorm,1);
        let icons=upload(device,queue,self.icons.size,&self.icons.pixels,wgpu::TextureFormat::Rgba8Unorm,4);
        let mask=mask.create_view(&Default::default());let icons=icons.create_view(&Default::default());
        let bind=device.create_bind_group(&wgpu::BindGroupDescriptor{label:Some("chrome bindings"),layout:&pipeline.get_bind_group_layout(0),entries:&[
            wgpu::BindGroupEntry{binding:0,resource:uniform.as_entire_binding()},wgpu::BindGroupEntry{binding:1,resource:items.as_entire_binding()},wgpu::BindGroupEntry{binding:2,resource:wgpu::BindingResource::TextureView(&mask)},wgpu::BindGroupEntry{binding:3,resource:wgpu::BindingResource::TextureView(&icons)}]});
        let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor{label:Some("chrome"),color_attachments:&[Some(wgpu::RenderPassColorAttachment{view:target,depth_slice:None,resolve_target:None,ops:wgpu::Operations{load:wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),store:wgpu::StoreOp::Store}})],..Default::default()});
        pass.set_pipeline(pipeline);pass.set_bind_group(0,&bind,&[]);pass.draw(0..6,0..instances.len() as u32);Ok(())
    }
    pub fn render_to_rgba(&mut self,list:&DrawList,width:u32,height:u32,scale:f32)->Result<Vec<u8>> {
        let format=wgpu::TextureFormat::Rgba8UnormSrgb;
        if width==0||height==0||width>8192||height>8192{return Err("invalid chrome target size".into());}
        let texture=self.context.device.create_texture(&wgpu::TextureDescriptor{label:Some("chrome headless"),size:wgpu::Extent3d{width,height,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format,usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::COPY_SRC,view_formats:&[]});
        let view=texture.create_view(&Default::default());let mut encoder=self.context.device.create_command_encoder(&Default::default());self.draw(list,width,height,scale,&view,format,&mut encoder)?;
        let stride=(width*4).div_ceil(256)*256;let buffer=self.context.device.create_buffer(&wgpu::BufferDescriptor{label:Some("chrome readback"),size:u64::from(stride)*u64::from(height),usage:wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::MAP_READ,mapped_at_creation:false});
        encoder.copy_texture_to_buffer(wgpu::TexelCopyTextureInfo{texture:&texture,mip_level:0,origin:wgpu::Origin3d::ZERO,aspect:wgpu::TextureAspect::All},wgpu::TexelCopyBufferInfo{buffer:&buffer,layout:wgpu::TexelCopyBufferLayout{offset:0,bytes_per_row:Some(stride),rows_per_image:Some(height)}},wgpu::Extent3d{width,height,depth_or_array_layers:1});
        let index=self.context.queue.submit([encoder.finish()]);let(tx,rx)=mpsc::sync_channel(1);buffer.slice(..).map_async(wgpu::MapMode::Read,move|r|{let _=tx.send(r);});self.context.device.poll(wgpu::PollType::Wait{submission_index:Some(index),timeout:Some(Duration::from_secs(20))}).map_err(|e|RenderError(e.to_string()))?;rx.recv_timeout(Duration::from_secs(20)).map_err(|e|RenderError(e.to_string()))?.map_err(|e|RenderError(e.to_string()))?;
        let mapped=buffer.slice(..).get_mapped_range();let mut rgba=Vec::with_capacity((width*height*4) as usize);for row in mapped.chunks(stride as usize).take(height as usize){rgba.extend_from_slice(&row[..(width*4) as usize]);}drop(mapped);buffer.unmap();Ok(rgba)
    }
}
fn upload(device:&wgpu::Device,queue:&wgpu::Queue,size:u32,pixels:&[u8],format:wgpu::TextureFormat,bpp:u32)->wgpu::Texture {
    let texture=device.create_texture(&wgpu::TextureDescriptor{label:Some("chrome atlas"),size:wgpu::Extent3d{width:size,height:size,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format,usage:wgpu::TextureUsages::COPY_DST|wgpu::TextureUsages::TEXTURE_BINDING,view_formats:&[]});queue.write_texture(wgpu::TexelCopyTextureInfo{texture:&texture,mip_level:0,origin:wgpu::Origin3d::ZERO,aspect:wgpu::TextureAspect::All},pixels,wgpu::TexelCopyBufferLayout{offset:0,bytes_per_row:Some(size*bpp),rows_per_image:Some(size)},wgpu::Extent3d{width:size,height:size,depth_or_array_layers:1});texture
}
