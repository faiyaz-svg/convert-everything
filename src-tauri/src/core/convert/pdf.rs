//! PDF pages to images and PDF compression with Ghostscript; page tools with qpdf.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use base64::Engine as _;

use crate::core::formats::Format;
use crate::core::fsutil;
use crate::core::options::Opts;
use crate::core::run::{Cmd, Ctx, Progress};

pub async fn page_count(ctx: &Ctx, input: &Path) -> Result<u32> {
    let out = ctx.run(Cmd::new(ctx.engines.qpdf()).arg("--show-npages").arg(input).capture()).await?;
    out.trim().parse().map_err(|_| anyhow!("Could not count the pages in this PDF."))
}

fn gs(ctx: &Ctx) -> Cmd {
    Cmd::new(ctx.engines.gs()).args(["-dSAFER", "-dBATCH", "-dNOPAUSE"])
}

/// Checks a page list such as "1-3, 5" and returns it in Ghostscript's form ("1-3,5").
fn page_list(s: &str) -> Result<String> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() || cleaned.eq_ignore_ascii_case("all") {
        return Ok(String::new());
    }
    let ok = cleaned.split(',').all(|part| {
        let mut nums = part.split('-');
        let a = nums.next().unwrap_or("");
        let b = nums.next();
        nums.next().is_none() && !a.is_empty() && a.chars().all(|c| c.is_ascii_digit()) && b.is_none_or(|b| b.is_empty() || b.chars().all(|c| c.is_ascii_digit()))
    });
    if !ok {
        bail!("The page list \"{s}\" is not valid. Use numbers and ranges such as 1-3, 5.");
    }
    Ok(cleaned)
}

/// Draws PDF pages as images. One page gives `<stem>.<ext>`; several give a folder.
pub async fn render(ctx: &Ctx, input: &Path, to: &Format, o: &Opts, out_dir: &Path, stem: &str) -> Result<()> {
    let (device, ext) = match to.id {
        "jpg" => ("jpeg", "jpg"),
        "tiff" => ("tiff24nc", "tif"),
        _ => ("png16m", "png"),
    };
    let dpi = o.str_or("dpi", "150");
    let pages = page_list(&o.str_or("pages", ""))?;
    let total = page_count(ctx, input).await.unwrap_or(0);
    let pages_dir = ctx.tmp("pages");
    std::fs::create_dir_all(&pages_dir)?;

    let mut cmd = gs(ctx)
        .arg(format!("-sDEVICE={device}"))
        .arg(format!("-r{dpi}"))
        .args(["-dTextAlphaBits=4", "-dGraphicsAlphaBits=4"]);
    if device == "jpeg" {
        cmd = cmd.arg(format!("-dJPEGQ={}", o.num("quality").unwrap_or(90.0).clamp(1.0, 100.0) as u32));
    }
    if !pages.is_empty() {
        cmd = cmd.arg(format!("-sPageList={pages}"));
    }
    let pattern = pages_dir.join(format!("page-%03d.{ext}"));
    cmd = cmd.arg(format!("-sOutputFile={}", pattern.display())).arg(input).progress(Progress::Pages { total });
    ctx.run(cmd).await?;

    let files = fsutil::entries(&pages_dir);
    match files.len() {
        0 => bail!("Ghostscript did not draw any pages. Check the page numbers."),
        1 => std::fs::rename(&files[0], out_dir.join(format!("{stem}.{}", to.exts[0])))?,
        _ => std::fs::rename(&pages_dir, out_dir.join(format!("{stem} (pages)")))?,
    }
    Ok(())
}

pub async fn compress(ctx: &Ctx, input: &Path, o: &Opts, out: &Path) -> Result<()> {
    let level = match o.str_or("level", "ebook").as_str() {
        "screen" => "screen",
        "printer" => "printer",
        _ => "ebook",
    };
    let total = page_count(ctx, input).await.unwrap_or(0);
    let cmd = gs(ctx)
        .args(["-sDEVICE=pdfwrite", "-dCompatibilityLevel=1.7", "-dDetectDuplicateImages=true", "-dCompressFonts=true"])
        .arg(format!("-dPDFSETTINGS=/{level}"))
        .arg(format!("-sOutputFile={}", out.display()))
        .arg(input)
        .progress(Progress::Pages { total });
    ctx.run(cmd).await?;
    Ok(())
}

/// Turns any images in the list into one-page PDFs so qpdf can join everything.
async fn as_pdfs(ctx: &Ctx, inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for (i, p) in inputs.iter().enumerate() {
        let is_pdf = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
        if is_pdf {
            out.push(p.clone());
        } else {
            let pdf = ctx.tmp(&format!("image-{i}.pdf"));
            super::image::images_to_pdf(ctx, std::slice::from_ref(p), &pdf).await?;
            out.push(pdf);
        }
    }
    Ok(out)
}

/// Joins PDFs (and images) in the given order.
pub async fn merge(ctx: &Ctx, inputs: &[PathBuf], out: &Path) -> Result<()> {
    if inputs.is_empty() {
        bail!("Add at least one file to merge.");
    }
    let pdfs = as_pdfs(ctx, inputs).await?;
    let mut cmd = Cmd::new(ctx.engines.qpdf()).args(["--empty", "--pages"]);
    for (i, p) in pdfs.iter().enumerate() {
        cmd = cmd.arg(p).arg("1-z");
        ctx.report(Some(i as f64 / pdfs.len() as f64 * 0.5));
    }
    ctx.run(cmd.arg("--").arg(out)).await?;
    Ok(())
}

/// Splits a PDF into one file per page, or one file per range ("1-3, 4-6").
pub async fn split(ctx: &Ctx, input: &Path, ranges: &str, out_dir: &Path, stem: &str) -> Result<()> {
    let dir = out_dir.join(format!("{stem} (split)"));
    std::fs::create_dir_all(&dir)?;
    let qpdf = ctx.engines.qpdf();
    let cleaned = page_list(ranges)?;
    if cleaned.is_empty() {
        let pattern = dir.join(format!("{stem} page %d.pdf"));
        ctx.run(Cmd::new(&qpdf).arg("--split-pages").arg(input).arg(pattern)).await?;
    } else {
        let parts: Vec<&str> = cleaned.split(',').collect();
        for (i, r) in parts.iter().enumerate() {
            let range = if r.ends_with('-') { format!("{r}z") } else { r.to_string() };
            let out = dir.join(format!("{stem} pages {}.pdf", r.trim_end_matches('-')));
            ctx.run(Cmd::new(&qpdf).arg(input).arg("--pages").arg(".").arg(&range).arg("--").arg(out)).await?;
            ctx.report(Some((i + 1) as f64 / parts.len() as f64));
        }
    }
    Ok(())
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct PagePlan {
    /// 1-based page number in the original file.
    pub page: u32,
    /// Extra clockwise rotation: 0, 90, 180 or 270.
    #[serde(default)]
    pub rotate: i32,
}

/// Writes a new PDF with pages in a new order, some rotated, some left out.
pub async fn arrange(ctx: &Ctx, input: &Path, plan: &[PagePlan], out: &Path) -> Result<()> {
    if plan.is_empty() {
        bail!("Keep at least one page.");
    }
    let order: Vec<String> = plan.iter().map(|p| p.page.to_string()).collect();
    let mut cmd = Cmd::new(ctx.engines.qpdf()).args(["--empty", "--pages"]).arg(input).arg(order.join(",")).arg("--");
    for angle in [90, 180, 270] {
        let positions: Vec<String> = plan.iter().enumerate().filter(|(_, p)| p.rotate.rem_euclid(360) == angle).map(|(i, _)| (i + 1).to_string()).collect();
        if !positions.is_empty() {
            cmd = cmd.arg(format!("--rotate=+{angle}:{}", positions.join(",")));
        }
    }
    ctx.run(cmd.arg(out)).await?;
    Ok(())
}

/// Small pictures of each page, as data URLs, for the reorder tool.
pub async fn thumbnails(ctx: &Ctx, input: &Path) -> Result<Vec<String>> {
    let dir = ctx.tmp("thumbs");
    std::fs::create_dir_all(&dir)?;
    let cmd = gs(ctx)
        .args(["-q", "-sDEVICE=png16m", "-r24", "-dTextAlphaBits=4", "-dGraphicsAlphaBits=4"])
        .arg(format!("-sOutputFile={}", dir.join("t-%04d.png").display()))
        .arg(input);
    ctx.run(cmd).await?;
    let engine = base64::engine::general_purpose::STANDARD;
    fsutil::entries(&dir)
        .iter()
        .map(|p| Ok(format!("data:image/png;base64,{}", engine.encode(std::fs::read(p)?))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::page_list;

    #[test]
    fn page_lists() {
        assert_eq!(page_list(" 1-3, 5 ").unwrap(), "1-3,5");
        assert_eq!(page_list("all").unwrap(), "");
        assert_eq!(page_list("4-").unwrap(), "4-");
        assert!(page_list("1-2-3").is_err());
        assert!(page_list("one").is_err());
    }
}
