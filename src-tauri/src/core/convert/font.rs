//! Fonts with fontTools (in the bundled Python).

use std::path::Path;

use anyhow::Result;

use crate::core::formats::Format;
use crate::core::run::{Cmd, Ctx};

const SCRIPT: &str = include_str!("fontconv.py");

pub async fn convert(ctx: &Ctx, input: &Path, to: &Format, out: &Path) -> Result<()> {
    let script = ctx.tmp("fontconv.py");
    std::fs::write(&script, SCRIPT)?;
    ctx.run(Cmd::new(ctx.engines.python()).arg("-I").arg(&script).arg(input).arg(out).arg(to.id)).await?;
    Ok(())
}
