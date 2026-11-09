//! Archives with 7-Zip. Converting between archive types means unpacking, then packing again.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::core::formats::Format;
use crate::core::fsutil;
use crate::core::options::Opts;
use crate::core::run::{Cmd, Ctx};

pub async fn convert(ctx: &Ctx, input: &Path, from: &Format, to: &Format, o: &Opts, out: &Path) -> Result<()> {
    let extracted = ctx.tmp("extracted");
    extract(ctx, input, from, &extracted, o.str("inPassword")).await?;
    if fsutil::entries(&extracted).is_empty() {
        bail!("The archive is empty.");
    }
    if to.id == "folder" {
        std::fs::rename(&extracted, out)?;
        return Ok(());
    }
    create(ctx, &extracted, to, o, out).await
}

async fn extract(ctx: &Ctx, input: &Path, from: &Format, dir: &Path, password: Option<String>) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let seven = ctx.engines.sevenzip();
    let pw = format!("-p{}", password.unwrap_or_default());
    let run = |src: PathBuf, dest: PathBuf| {
        Cmd::new(&seven).args(["x", "-y", "-bso0", "-bsp0", "-aou"]).arg(&pw).arg(format!("-o{}", dest.display())).arg(src)
    };
    let result = ctx.run(run(input.to_path_buf(), dir.to_path_buf())).await;
    if let Err(e) = result {
        let msg = e.to_string();
        if msg.contains("Wrong password") || msg.contains("encrypted") {
            bail!("This archive is protected with a password. Enter it under More options.");
        }
        return Err(e);
    }

    // .tar.gz and friends unpack to a single .tar first.
    if ["tgz", "txz", "tbz2", "gz", "xz", "bz2"].contains(&from.id) {
        let items = fsutil::entries(dir);
        if items.len() == 1 && items[0].extension().is_some_and(|e| e.eq_ignore_ascii_case("tar")) {
            let tar = items[0].clone();
            let inner = ctx.tmp("extracted-tar");
            std::fs::create_dir_all(&inner)?;
            ctx.run(run(tar.clone(), inner.clone())).await?;
            std::fs::remove_file(&tar)?;
            for item in fsutil::entries(&inner) {
                std::fs::rename(&item, dir.join(item.file_name().unwrap()))?;
            }
        }
    }
    Ok(())
}

async fn create(ctx: &Ctx, src_dir: &Path, to: &Format, o: &Opts, out: &Path) -> Result<()> {
    let seven = ctx.engines.sevenzip();
    let level = o.str_or("level", "5");
    let stream_level = if level == "0" { "1".to_string() } else { level.clone() };
    let all = src_dir.join("*");
    let add = |kind: &str| Cmd::new(&seven).args(["a", "-y", "-bso0", "-bsp0"]).arg(format!("-t{kind}"));
    match to.id {
        "zip" | "7z" => {
            let mut cmd = add(to.id).arg(format!("-mx={level}"));
            if let Some(pw) = o.str("password") {
                cmd = cmd.arg(format!("-p{pw}"));
                cmd = if to.id == "7z" { cmd.arg("-mhe=on") } else { cmd.arg("-mem=AES256") };
            }
            ctx.run(cmd.arg(out).arg(&all)).await?;
        }
        "tar" => {
            ctx.run(add("tar").arg(out).arg(&all)).await?;
        }
        "tgz" | "txz" | "tbz2" => {
            let tar = ctx.tmp("packed.tar");
            ctx.run(add("tar").arg(&tar).arg(&all)).await?;
            let kind = match to.id {
                "tgz" => "gzip",
                "txz" => "xz",
                _ => "bzip2",
            };
            ctx.run(add(kind).arg(format!("-mx={stream_level}")).arg(out).arg(&tar)).await?;
        }
        other => bail!("Can't create {other} archives."),
    }
    Ok(())
}
