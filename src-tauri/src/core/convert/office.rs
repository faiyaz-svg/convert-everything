//! Office documents, spreadsheets and presentations with LibreOffice.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};

use crate::core::formats::{Category, Format};
use crate::core::fsutil;
use crate::core::run::{Cmd, Ctx};

const SLOTS: usize = 16;
static IN_USE: Mutex<[bool; SLOTS]> = Mutex::new([false; SLOTS]);

/// One LibreOffice user profile. Two LibreOffice processes can't share a profile,
/// so each running conversion borrows its own. Profiles are kept between runs
/// because the first start with a new profile is slow.
struct Slot(usize);

impl Drop for Slot {
    fn drop(&mut self) {
        IN_USE.lock().unwrap()[self.0] = false;
    }
}

async fn take_slot(ctx: &Ctx) -> Result<Slot> {
    loop {
        {
            let mut used = IN_USE.lock().unwrap();
            if let Some(i) = used.iter().position(|u| !u) {
                used[i] = true;
                return Ok(Slot(i));
            }
        }
        ctx.check_canceled()?;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

fn profile_dir(slot: usize) -> PathBuf {
    std::env::temp_dir().join("convert-everything").join(format!("lo-profile-{slot}"))
}

/// file:///C:/Users/... with unsafe characters percent-encoded.
fn file_url(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file:///");
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/:._-~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn filter(from: &Format, to: &Format) -> String {
    let f = match to.id {
        "pdf" => match from.category {
            Category::Spreadsheet | Category::Data => "pdf:calc_pdf_Export",
            Category::Presentation => "pdf:impress_pdf_Export",
            Category::Image => "pdf:draw_pdf_Export",
            _ if from.id == "html" => "pdf:writer_web_pdf_Export",
            _ => "pdf:writer_pdf_Export",
        },
        "docx" => "docx:MS Word 2007 XML",
        "doc" => "doc:MS Word 97",
        "odt" => "odt:writer8",
        "rtf" => "rtf:Rich Text Format",
        "txt" => "txt:Text (encoded):UTF8",
        "html" if from.category == Category::Spreadsheet => "html:HTML (StarCalc)",
        "html" => "html:HTML (StarWriter)",
        "xlsx" => "xlsx:Calc MS Excel 2007 XML",
        "xls" => "xls:MS Excel 97",
        "ods" => "ods:calc8",
        "csv" => "csv:Text - txt - csv (StarCalc):44,34,76,1",
        "pptx" => "pptx:Impress MS PowerPoint 2007 XML",
        "ppt" => "ppt:MS PowerPoint 97",
        "odp" => "odp:impress8",
        other => other,
    };
    f.to_string()
}

pub async fn convert(ctx: &Ctx, input: &Path, from: &Format, to: &Format, out: &Path) -> Result<()> {
    let slot = take_slot(ctx).await?;
    let profile = profile_dir(slot.0);
    let _ = std::fs::remove_file(profile.join("user").join(".lock"));
    let out_dir = ctx.tmp("office-out");
    std::fs::create_dir_all(&out_dir)?;

    let mut cmd = Cmd::new(ctx.engines.soffice())
        .arg(format!("-env:UserInstallation={}", file_url(&profile)))
        .args(["--headless", "--invisible", "--norestore", "--nologo", "--nodefault", "--nolockcheck"]);
    match from.id {
        "pdf" => cmd = cmd.arg("--infilter=writer_pdf_import"),
        "csv" => cmd = cmd.arg("--infilter=CSV:44,34,76,1"),
        "txt" => cmd = cmd.arg("--infilter=Text (encoded):UTF8"),
        _ => {}
    }
    cmd = cmd.arg("--convert-to").arg(filter(from, to)).arg("--outdir").arg(&out_dir).arg(input);
    ctx.run(cmd).await?;
    drop(slot);

    let produced = fsutil::entries(&out_dir)
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| anyhow!("LibreOffice did not write a file. The document may be damaged or protected with a password."))?;
    std::fs::rename(&produced, out).context("Could not move the converted file")?;
    Ok(())
}
