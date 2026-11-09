//! Audio, video, GIF and subtitles with FFmpeg.

use std::path::Path;

use anyhow::{Result, bail};

use crate::core::formats::{Category, Format};
use crate::core::options::{Opts, parse_time};
use crate::core::run::{Cmd, Ctx, Progress};

/// Length of a media file in seconds, from ffprobe.
pub async fn duration(ctx: &Ctx, input: &Path) -> Option<f64> {
    let cmd = Cmd::new(ctx.engines.ffprobe())
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(input)
        .capture();
    ctx.run(cmd).await.ok()?.lines().find_map(|l| l.trim().parse::<f64>().ok())
}

fn quality_index(o: &Opts) -> usize {
    match o.str_or("quality", "medium").as_str() {
        "high" => 0,
        "low" => 2,
        _ => 1,
    }
}

pub async fn convert(ctx: &Ctx, input: &Path, from: &Format, to: &Format, o: &Opts, out: &Path) -> Result<()> {
    let start = o.str("trimStart").and_then(|s| parse_time(&s));
    let end = o.str("trimEnd").and_then(|s| parse_time(&s));
    if let (Some(s), Some(e)) = (start, end)
        && e <= s
    {
        bail!("The end time must be after the start time.");
    }

    let full = if to.category == Category::Subtitle { 0.0 } else { duration(ctx, input).await.unwrap_or(0.0) };
    let length = match (start, end) {
        (_, Some(e)) => e - start.unwrap_or(0.0),
        (Some(s), None) => (full - s).max(0.0),
        (None, None) => full,
    };

    let mut cmd = Cmd::new(ctx.engines.ffmpeg()).args(["-hide_banner", "-nostdin", "-y", "-loglevel", "error", "-nostats", "-progress", "pipe:1"]);
    if let Some(s) = start {
        cmd = cmd.arg("-ss").arg(format!("{s:.3}"));
    }
    cmd = cmd.arg("-i").arg(input);
    if end.is_some() {
        cmd = cmd.arg("-t").arg(format!("{length:.3}"));
    }

    cmd = if to.category == Category::Subtitle {
        cmd
    } else if to.id == "gif" {
        gif_args(cmd, o)
    } else if to.category == Category::Audio {
        audio_args(cmd.arg("-vn"), to, o)
    } else {
        video_args(cmd, from, to, o, length)?
    };

    ctx.run(cmd.arg(out).progress(Progress::Ffmpeg { duration: length })).await?;
    Ok(())
}

fn gif_args(cmd: Cmd, o: &Opts) -> Cmd {
    let fps = o.str_or("fps", "15");
    let width = o.str_or("width", "480");
    let scale = if width == "keep" { String::new() } else { format!(",scale={width}:-1:flags=lanczos") };
    let filter = format!("fps={fps}{scale},split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse=dither=bayer:bayer_scale=4");
    cmd.arg("-vf").arg(filter).args(["-loop", "0"])
}

fn audio_args(mut cmd: Cmd, to: &Format, o: &Opts) -> Cmd {
    let default_br = if to.id == "opus" { "128" } else { "192" };
    let br = format!("{}k", o.str_or("bitrate", default_br));
    cmd = match to.id {
        "mp3" => cmd.args(["-c:a", "libmp3lame", "-b:a", &br]),
        "m4a" => cmd.args(["-c:a", "aac", "-b:a", &br]),
        "alac" => cmd.args(["-c:a", "alac"]),
        "flac" => cmd.args(["-c:a", "flac"]),
        "wav" => cmd.args(["-c:a", "pcm_s16le"]),
        "aiff" => cmd.args(["-c:a", "pcm_s16be"]),
        "ogg" => cmd.args(["-c:a", "libvorbis", "-b:a", &br]),
        "opus" => cmd.args(["-c:a", "libopus", "-b:a", &br]),
        _ => cmd,
    };
    if to.id == "alac" {
        cmd = cmd.args(["-f", "ipod"]);
    }
    let sr = o.str_or("sampleRate", "keep");
    if sr != "keep" {
        cmd = cmd.arg("-ar").arg(sr);
    }
    let ch = o.str_or("channels", "keep");
    if ch != "keep" {
        cmd = cmd.arg("-ac").arg(ch);
    }
    if o.bool("normalize", false) {
        cmd = cmd.args(["-af", "loudnorm=I=-16:TP=-1.5:LRA=11"]);
    }
    cmd
}

/// The codec a container gets when the user did not pick one (or there is no choice).
fn video_codec(to: &str, o: &Opts) -> String {
    let fixed = match to {
        "avi" => Some("mpeg4"),
        "wmv" => Some("wmv2"),
        "mpeg" => Some("mpeg2"),
        "flv" | "3gp" => Some("h264"),
        _ => None,
    };
    if let Some(c) = fixed {
        return c.into();
    }
    let default = if to == "webm" { "vp9" } else { "h264" };
    let c = o.str_or("codec", default);
    // Guard against a codec that does not fit the container.
    match (to, c.as_str()) {
        ("webm", "vp9" | "av1") => c,
        ("webm", _) => "vp9".into(),
        ("mov", "vp9" | "av1") => "h264".into(),
        (_, "prores") if to != "mov" => "h264".into(),
        _ => c,
    }
}

fn audio_for_video(to: &str, vcodec: &str) -> Vec<&'static str> {
    match (to, vcodec) {
        ("webm", _) | ("mkv", "vp9" | "av1") => vec!["-c:a", "libopus", "-b:a", "128k"],
        ("avi", _) => vec!["-c:a", "libmp3lame", "-b:a", "192k"],
        ("wmv", _) => vec!["-c:a", "wmav2", "-b:a", "192k"],
        ("mpeg", _) => vec!["-c:a", "mp2", "-b:a", "192k"],
        (_, "prores") => vec!["-c:a", "pcm_s16le"],
        _ => vec!["-c:a", "aac", "-b:a", "160k"],
    }
}

fn video_args(mut cmd: Cmd, from: &Format, to: &Format, o: &Opts, length: f64) -> Result<Cmd> {
    let codec = video_codec(to.id, o);
    let remove_audio = o.str_or("audio", "keep") == "remove";

    if codec == "copy" {
        cmd = if to.id == "mkv" { cmd.args(["-map", "0", "-c", "copy"]) } else { cmd.args(["-map", "0:v", "-map", "0:a?", "-c", "copy", "-sn"]) };
        if remove_audio {
            cmd = cmd.arg("-an");
        }
        if to.id == "mp4" || to.id == "mov" {
            cmd = cmd.args(["-movflags", "+faststart"]);
        }
        return Ok(cmd);
    }

    let q = quality_index(o);
    let speed = o.str_or("speed", "medium");
    let target_kbps = o.num("targetMb").filter(|v| *v > 0.0 && length > 0.0).map(|mb| {
        let audio = if remove_audio { 0.0 } else { 160.0 };
        ((mb * 8192.0 / length) - audio).max(100.0)
    });

    let x26x_preset = match speed.as_str() {
        "fast" => "veryfast",
        "slow" => "slow",
        _ => "medium",
    };
    let mut yuv420 = true;
    let rate = |cmd: Cmd, crf: &str| -> Cmd {
        match target_kbps {
            Some(k) => cmd.arg("-b:v").arg(format!("{}k", k as u64)).arg("-maxrate").arg(format!("{}k", (k * 1.5) as u64)).arg("-bufsize").arg(format!("{}k", (k * 2.0) as u64)),
            None => cmd.arg("-crf").arg(crf),
        }
    };
    cmd = match codec.as_str() {
        "h265" => {
            let c = rate(cmd.args(["-c:v", "libx265", "-preset", x26x_preset]), ["22", "28", "32"][q]);
            if to.id == "mp4" || to.id == "mov" { c.args(["-tag:v", "hvc1"]) } else { c }
        }
        "av1" => {
            let p = match speed.as_str() {
                "fast" => "10",
                "slow" => "5",
                _ => "8",
            };
            rate(cmd.args(["-c:v", "libsvtav1", "-preset", p]), ["28", "35", "42"][q])
        }
        "vp9" => {
            let cpu = match speed.as_str() {
                "fast" => "5",
                "slow" => "1",
                _ => "3",
            };
            let c = cmd.args(["-c:v", "libvpx-vp9", "-row-mt", "1", "-deadline", "good", "-cpu-used", cpu]);
            match target_kbps {
                Some(k) => c.arg("-b:v").arg(format!("{}k", k as u64)),
                None => c.args(["-b:v", "0", "-crf", ["24", "32", "40"][q]]),
            }
        }
        "prores" => {
            yuv420 = false;
            cmd.args(["-c:v", "prores_ks", "-profile:v", ["3", "2", "0"][q], "-pix_fmt", "yuv422p10le"])
        }
        "mpeg4" => cmd.args(["-c:v", "mpeg4", "-q:v", ["2", "4", "7"][q]]),
        "wmv2" => cmd.args(["-c:v", "wmv2", "-q:v", ["2", "4", "7"][q]]),
        "mpeg2" => cmd.args(["-c:v", "mpeg2video", "-q:v", ["2", "4", "7"][q]]),
        _ => {
            let c = rate(cmd.args(["-c:v", "libx264", "-preset", x26x_preset]), ["18", "23", "28"][q]);
            if to.id == "3gp" { c.args(["-profile:v", "baseline"]) } else { c }
        }
    };
    if yuv420 {
        cmd = cmd.args(["-pix_fmt", "yuv420p"]);
    }

    // Scale down to the chosen height and keep sizes even (most encoders need that).
    let height = o.str_or("height", "keep");
    let scale = if height == "keep" {
        "scale=trunc(iw/2)*2:trunc(ih/2)*2".to_string()
    } else {
        format!("scale=-2:'trunc(min(ih,{height})/2)*2'")
    };
    cmd = cmd.arg("-vf").arg(scale);
    let fps = o.str_or("fps", "keep");
    if fps != "keep" {
        cmd = cmd.arg("-r").arg(fps);
    }

    if remove_audio || from.id == "gif" {
        cmd = cmd.arg("-an");
    } else {
        cmd = cmd.args(audio_for_video(to.id, &codec));
    }
    if to.id != "mkv" {
        cmd = cmd.arg("-sn");
    }
    if to.id == "mp4" || to.id == "mov" {
        cmd = cmd.args(["-movflags", "+faststart"]);
    }
    Ok(cmd)
}
