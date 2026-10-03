use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=icon.svg");
    println!("cargo:rerun-if-changed=ui/main.slint");

    let cfg = slint_build::CompilerConfiguration::new().with_style("fluent-light".into());
    slint_build::compile_with_config("ui/main.slint", cfg).unwrap();

    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        let out = out_dir();
        let ico = out.join("app.ico");
        write_ico(&ico);
        // Raw 64px premultiplied RGBA for the runtime tray icon, so the
        // exe doesn't carry an SVG renderer just for this.
        std::fs::write(out.join("icon-64.rgba"), render_rgba(64)).expect("must write icon-64.rgba");
        let mut res = winres::WindowsResource::new();
        res.set_icon(ico.to_str().expect("out dir must be UTF-8"));
        res.compile().expect("winres must embed the icon");
    }
}

fn out_dir() -> PathBuf {
    PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set"))
}

/// Rasterize icon.svg at several sizes into a multi-image .ico.
fn write_ico(dest: &PathBuf) {
    const SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in SIZES {
        let mut rgba = render_rgba(size);
        unpremultiply(&mut rgba);
        let image = ico::IconImage::from_rgba_data(size, size, rgba);
        dir.add_entry(ico::IconDirEntry::encode(&image).expect("ico entry must encode"));
    }
    let file = std::fs::File::create(dest).expect("must create app.ico");
    dir.write(file).expect("must write app.ico");
}

/// Rasterize icon.svg to premultiplied RGBA at `size`px.
fn render_rgba(size: u32) -> Vec<u8> {
    const SVG: &[u8] = include_bytes!("icon.svg");
    let tree = resvg::usvg::Tree::from_data(SVG, &resvg::usvg::Options::default())
        .expect("icon.svg must parse");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("icon size sane");
    let scale = size as f32 / tree.size().width().max(1.0);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.take()
}

/// ICO PNG entries store straight alpha; tiny-skia pixmaps are premultiplied.
fn unpremultiply(px: &mut [u8]) {
    let (chunks, _rest) = px.as_chunks_mut::<4>();
    for p in chunks {
        let a = p[3] as u32;
        if a == 0 {
            p[0] = 0;
            p[1] = 0;
            p[2] = 0;
        } else if a < 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 / a).min(255)) as u8;
            }
        }
    }
}
