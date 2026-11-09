//! Raster images with ImageMagick, SVG input with resvg, tracing to SVG with VTracer.

use std::ffi::OsString;
use std::path::Path;

use anyhow::Result;

use crate::core::formats::Format;
use crate::core::options::Opts;
use crate::core::run::{Cmd, Ctx};

const LOSSY: &[&str] = &["jpg", "webp", "avif", "jxl"];

/// A `magick` command with the environment it needs (Ghostscript for EPS and PDF).
pub fn magick(ctx: &Ctx) -> Cmd {
    let gs_bin = ctx.engines.gs_bin();
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut new_path = OsString::from(&gs_bin);
    new_path.push(";");
    new_path.push(path);
    Cmd::new(ctx.engines.magick())
        .env("MAGICK_TEMPORARY_PATH", ctx.work.join("tmp"))
        .env("MAGICK_GHOSTSCRIPT_PATH", &gs_bin)
        .env("PATH", new_path)
}

/// Index of the largest picture in an ICO file.
async fn largest_ico_frame(ctx: &Ctx, input: &Path) -> Option<usize> {
    let out = ctx.run(magick(ctx).arg("identify").arg("-format").arg("%w\n").arg(input).capture()).await.ok()?;
    out.lines()
        .filter_map(|l| l.trim().parse::<u32>().ok())
        .enumerate()
        .max_by_key(|(_, w)| *w)
        .map(|(i, _)| i)
}

pub async fn convert(ctx: &Ctx, input: &Path, from: &Format, to: &Format, o: &Opts, out: &Path) -> Result<()> {
    std::fs::create_dir_all(ctx.work.join("tmp"))?;
    let multi_frame_out = ["gif", "tiff", "pdf"].contains(&to.id);
    let mut cmd = magick(ctx);

    if from.id == "eps" {
        cmd = cmd.args(["-density", "300"]);
    }
    let mut src = input.as_os_str().to_os_string();
    match from.id {
        "psd" => src.push("[0]"),
        "ico" => {
            if let Some(i) = largest_ico_frame(ctx, input).await {
                src.push(format!("[{i}]"));
            }
        }
        "gif" | "tiff" if !multi_frame_out => src.push("[0]"),
        _ => {}
    }
    cmd = cmd.arg(src);

    let animated = from.id == "gif" && to.id == "gif";
    if animated {
        cmd = cmd.arg("-coalesce");
    }
    cmd = cmd.arg("-auto-orient");
    cmd = add_image_ops(cmd, to, o, true);
    if animated {
        cmd = cmd.args(["-layers", "Optimize"]);
    }
    ctx.run(cmd.arg(out)).await?;
    Ok(())
}

/// Resize, quality, size target, background and metadata options.
fn add_image_ops(mut cmd: Cmd, to: &Format, o: &Opts, allow_resize: bool) -> Cmd {
    if allow_resize {
        let w = o.num("maxWidth").filter(|v| *v >= 1.0).map(|v| (v as u32).to_string()).unwrap_or_default();
        let h = o.num("maxHeight").filter(|v| *v >= 1.0).map(|v| (v as u32).to_string()).unwrap_or_default();
        if !w.is_empty() || !h.is_empty() {
            cmd = cmd.arg("-resize").arg(format!("{w}x{h}>"));
        }
    }
    if ["jpg", "bmp"].contains(&to.id) {
        cmd = cmd.args(["-background", "white", "-alpha", "remove", "-alpha", "off"]);
    }
    if LOSSY.contains(&to.id)
        && let Some(q) = o.num("quality")
    {
        cmd = cmd.arg("-quality").arg((q.clamp(1.0, 100.0) as u32).to_string());
    }
    if let Some(kb) = o.num("targetKb").filter(|v| *v >= 1.0) {
        match to.id {
            "jpg" => cmd = cmd.arg("-define").arg(format!("jpeg:extent={}KB", kb as u64)),
            "webp" => cmd = cmd.arg("-define").arg(format!("webp:target-size={}", (kb * 1024.0) as u64)),
            _ => {}
        }
    }
    if o.bool("strip", false) {
        cmd = cmd.arg("-strip");
    }
    if to.id == "ico" {
        cmd = cmd.args(["-define", "icon:auto-resize=256,128,64,48,32,16"]);
    }
    cmd
}

/// SVG to a raster format: resvg draws it, ImageMagick converts if needed.
pub async fn from_svg(ctx: &Ctx, input: &Path, to: &Format, o: &Opts, out: &Path) -> Result<()> {
    let png = if to.id == "png" { out.to_path_buf() } else { ctx.tmp("drawn.png") };
    let mut cmd = Cmd::new(ctx.engines.resvg());
    if let Some(w) = o.num("width").filter(|v| *v >= 1.0) {
        cmd = cmd.arg("-w").arg((w as u32).to_string());
    }
    ctx.run(cmd.arg(input).arg(&png)).await?;
    if to.id != "png" {
        let cmd = add_image_ops(magick(ctx).arg(&png), to, o, false);
        ctx.run(cmd.arg(out)).await?;
    } else if o.bool("strip", false) {
        // resvg writes no metadata, nothing to strip.
    }
    Ok(())
}

/// Raster to SVG by tracing shapes with VTracer.
pub async fn trace(ctx: &Ctx, input: &Path, from: &Format, o: &Opts, out: &Path) -> Result<()> {
    let png = ctx.tmp("trace-input.png");
    let mut src = input.as_os_str().to_os_string();
    if ["gif", "tiff", "psd", "ico"].contains(&from.id) {
        src.push("[0]");
    }
    ctx.run(magick(ctx).arg(src).arg("-auto-orient").arg(&png)).await?;

    let (speckle, precision) = match o.str_or("detail", "medium").as_str() {
        "high" => ("2", "8"),
        "low" => ("10", "4"),
        _ => ("4", "6"),
    };
    let mut cmd = Cmd::new(ctx.engines.vtracer()).arg("--input").arg(&png).arg("--output").arg(out);
    if o.str_or("colors", "color") == "binary" {
        cmd = cmd.args(["--preset", "bw"]);
    }
    cmd = cmd.args(["--mode", "spline", "--filter-speckle", speckle, "--color-precision", precision]);
    ctx.run(cmd).await?;
    Ok(())
}

/// Several images into one PDF, one picture per page, in the given order.
pub async fn images_to_pdf(ctx: &Ctx, inputs: &[std::path::PathBuf], out: &Path) -> Result<()> {
    let mut cmd = magick(ctx);
    for p in inputs {
        let mut s = p.as_os_str().to_os_string();
        let lower = p.to_string_lossy().to_lowercase();
        if lower.ends_with(".gif") || lower.ends_with(".psd") {
            s.push("[0]");
        }
        cmd = cmd.arg(s);
    }
    cmd = cmd.args(["-auto-orient", "-background", "white", "-alpha", "remove", "-alpha", "off", "-quality", "92"]);
    ctx.run(cmd.arg(out)).await?;
    Ok(())
}
