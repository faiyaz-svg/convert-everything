//! Ebooks with Calibre's ebook-convert.

use std::path::Path;

use anyhow::Result;

use crate::core::formats::Format;
use crate::core::options::Opts;
use crate::core::run::{Cmd, Ctx, Progress};

pub async fn convert(ctx: &Ctx, input: &Path, from: &Format, to: &Format, o: &Opts, out: &Path) -> Result<()> {
    let config = ctx.tmp("calibre-config");
    let temp = ctx.tmp("calibre-temp");
    std::fs::create_dir_all(&config)?;
    std::fs::create_dir_all(&temp)?;

    // Calibre picks the input type from the extension, so give it a copy with a
    // name it knows (Markdown files must end in .md, comics in .cbz, and so on).
    let named = ctx.tmp(&format!("input.{}", from.exts[0]));
    std::fs::copy(input, &named)?;

    let mut cmd = Cmd::new(ctx.engines.ebook_convert())
        .arg(&named)
        .arg(out)
        .env("CALIBRE_CONFIG_DIRECTORY", &config)
        .env("CALIBRE_TEMP_DIR", &temp)
        .env("CALIBRE_NO_NATIVE_FILEDIALOGS", "1")
        .progress(Progress::Percent);
    if from.id == "md" {
        cmd = cmd.args(["--formatting-type", "markdown"]);
    }
    if to.id == "pdf" {
        cmd = cmd.arg("--paper-size").arg(o.str_or("paper", "letter")).arg("--pdf-page-numbers");
    }
    ctx.run(cmd).await?;
    Ok(())
}
