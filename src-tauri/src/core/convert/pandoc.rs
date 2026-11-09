//! Text and markup documents with Pandoc.

use std::path::Path;

use anyhow::Result;

use crate::core::formats::{Format, by_id};
use crate::core::run::{Cmd, Ctx};

fn reader(id: &str) -> &'static str {
    match id {
        "html" => "html",
        "docx" => "docx",
        "odt" => "odt",
        "epub" => "epub",
        "rst" => "rst",
        "org" => "org",
        "tex" => "latex",
        _ => "markdown",
    }
}

fn writer(id: &str) -> &'static str {
    match id {
        "html" => "html5",
        "docx" => "docx",
        "odt" => "odt",
        "rst" => "rst",
        "org" => "org",
        "tex" => "latex",
        "epub" => "epub3",
        "txt" => "plain",
        _ => "gfm",
    }
}

/// Converts with Pandoc. `out` must sit in its own folder: pictures pulled out of
/// a Word file or ebook are saved next to it in "<name>_files".
pub async fn convert(ctx: &Ctx, input: &Path, from: &Format, to: &Format, stem: &str, out: &Path) -> Result<()> {
    let out_dir = out.parent().unwrap_or(Path::new("."));
    let in_dir = input.parent().unwrap_or(Path::new("."));
    let mut cmd = Cmd::new(ctx.engines.pandoc())
        .cwd(out_dir)
        .arg(input)
        .arg("-f")
        .arg(reader(from.id))
        .arg("-t")
        .arg(writer(to.id))
        .arg("-o")
        .arg(out)
        .arg(format!("--resource-path={}", in_dir.display()));
    match to.id {
        "html" => {
            cmd = cmd.args(["-s", "--embed-resources"]).arg("--metadata").arg(format!("pagetitle={stem}"));
        }
        "tex" => cmd = cmd.arg("-s"),
        "epub" => cmd = cmd.arg("--metadata").arg(format!("title={stem}")),
        _ => {}
    }
    if ["md", "rst", "org", "txt", "tex"].contains(&to.id) {
        cmd = cmd.arg("--wrap=none");
        if ["docx", "odt", "epub", "html"].contains(&from.id) && to.id != "txt" {
            cmd = cmd.arg(format!("--extract-media={stem}_files"));
        }
    }
    ctx.run(cmd).await?;
    Ok(())
}

/// Markdown and other markup to PDF: Pandoc makes a Word file, LibreOffice prints it.
pub async fn to_pdf(ctx: &Ctx, input: &Path, from: &Format, stem: &str, out: &Path) -> Result<()> {
    let docx_dir = ctx.tmp("pandoc-docx");
    std::fs::create_dir_all(&docx_dir)?;
    let docx = docx_dir.join(format!("{stem}.docx"));
    let docx_fmt = by_id("docx").unwrap();
    convert(ctx, input, from, docx_fmt, stem, &docx).await?;
    super::office::convert(ctx, &docx, docx_fmt, by_id("pdf").unwrap(), out).await
}
