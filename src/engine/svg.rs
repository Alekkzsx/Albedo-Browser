use resvg::{usvg, tiny_skia};
use slint::{Image, SharedPixelBuffer, Rgba8Pixel};

pub fn rasterize_svg(svg_data: &str, width: f32, height: f32) -> Option<Image> {
    let opt = usvg::Options::default();
    
    let rtree = match usvg::Tree::from_str(svg_data, &opt) {
        Ok(tree) => tree,
        Err(e) => {
            println!("SVG ERROR: Fail to parse SVG: {}", e);
            return None;
        }
    };

    let pixmap_size = rtree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(
        width.round() as u32, 
        height.round() as u32
    ).or_else(|| {
        tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
    })?;

    let render_ts = if width > 0.0 && height > 0.0 {
        let sx = width / rtree.size().width();
        let sy = height / rtree.size().height();
        tiny_skia::Transform::from_scale(sx, sy)
    } else {
        tiny_skia::Transform::default()
    };

    resvg::render(&rtree, render_ts, &mut pixmap.as_mut());

    let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(pixmap.width(), pixmap.height());
    pixel_buffer.make_mut_bytes().copy_from_slice(pixmap.data());

    Some(Image::from_rgba8_premultiplied(pixel_buffer))
}
