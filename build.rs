use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=icon.svg");
    println!("cargo:rerun-if-changed=ui/main.slint");

    let cfg = slint_build::CompilerConfiguration::new().with_style("fluent-light".into());
    slint_build::compile_with_config("ui/main.slint", cfg).unwrap();

    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        let ico = out_dir().join("app.ico");
        write_ico(&ico);
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
    const SVG: &[u8] = include_bytes!("icon.svg");
    let tree = resvg::usvg::Tree::from_data(SVG, &resvg::usvg::Options::default())
        .expect("icon.svg must parse");
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16u32, 24, 32, 48, 64, 128, 256] {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("icon size sane");
        let scale = size as f32 / tree.size().width().max(1.0);
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        let mut rgba = pixmap.take();
        unpremultiply(&mut rgba);
        let image = ico::IconImage::from_rgba_data(size, size, rgba);
        dir.add_entry(ico::IconDirEntry::encode(&image).expect("ico entry must encode"));
    }
    let file = std::fs::File::create(dest).expect("must create app.ico");
    dir.write(file).expect("must write app.ico");
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
