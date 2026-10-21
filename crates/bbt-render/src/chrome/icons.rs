//! Baked RGBA assets, provided by bboxybook's build step, share one atlas.
use std::collections::HashMap;
use crate::Result;
pub struct Image { pub name:String,pub width:u32,pub height:u32,pub rgba:Vec<u8> }
pub struct IconAtlas { pub size:u32,pub pixels:Vec<u8>,pub rects:HashMap<String,[f32;4]> }
impl IconAtlas {
    pub fn new(images:Vec<Image>)->Result<Self>{
        let size=2048u32;let mut atlas=Self{size,pixels:vec![0;(size*size*4) as usize],rects:HashMap::new()};let(mut x,mut y,mut row)=(0,0,0);
        for image in images {
            if image.width==0||image.height==0||image.width>=size||image.height>=size||image.rgba.len()!=(image.width*image.height*4) as usize{return Err("invalid icon image".into());}
            if x+image.width+1>size {x=0;y+=row;row=0;}
            if y+image.height+1>size{return Err("icon atlas capacity exhausted".into());}
            for dy in 0..image.height {let offset=(((y+dy)*size+x)*4) as usize;let src=(dy*image.width*4) as usize;atlas.pixels[offset..offset+(image.width*4) as usize].copy_from_slice(&image.rgba[src..src+(image.width*4) as usize]);}
            atlas.rects.insert(image.name,[x as f32,y as f32,image.width as f32,image.height as f32]);x+=image.width+1;row=row.max(image.height+1);
        }Ok(atlas)
    }
}
