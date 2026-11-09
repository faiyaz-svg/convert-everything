//! Runs engine programs for one job: hidden window, progress, cancel and a short log.

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::engines::Engines;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Returned when the user cancels a job.
#[derive(Debug)]
pub struct Canceled;

impl std::fmt::Display for Canceled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Canceled")
    }
}

impl std::error::Error for Canceled {}

pub fn is_canceled(e: &anyhow::Error) -> bool {
    e.downcast_ref::<Canceled>().is_some()
}

/// How to read progress from a program's output.
#[derive(Clone, Copy, Debug)]
pub enum Progress {
    None,
    /// ffmpeg `-progress pipe:1` lines, against the expected output length in seconds.
    Ffmpeg { duration: f64 },
    /// Lines that start with a percentage, like Calibre's "45% Converting...".
    Percent,
    /// Ghostscript "Page N" lines, against the number of pages.
    Pages { total: u32 },
}

pub struct Cmd {
    program: PathBuf,
    args: Vec<OsString>,
    envs: Vec<(OsString, OsString)>,
    cwd: Option<PathBuf>,
    progress: Progress,
    capture: bool,
}

impl Cmd {
    pub fn new(program: impl AsRef<Path>) -> Self {
        Cmd { program: program.as_ref().to_path_buf(), args: vec![], envs: vec![], cwd: None, progress: Progress::None, capture: false }
    }
    pub fn arg(mut self, a: impl AsRef<OsStr>) -> Self {
        self.args.push(a.as_ref().to_os_string());
        self
    }
    pub fn args<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args.extend(items.into_iter().map(|a| a.as_ref().to_os_string()));
        self
    }
    pub fn env(mut self, k: impl AsRef<OsStr>, v: impl AsRef<OsStr>) -> Self {
        self.envs.push((k.as_ref().to_os_string(), v.as_ref().to_os_string()));
        self
    }
    pub fn cwd(mut self, dir: impl AsRef<Path>) -> Self {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }
    pub fn progress(mut self, p: Progress) -> Self {
        self.progress = p;
        self
    }
    /// Keep standard output and return it.
    pub fn capture(mut self) -> Self {
        self.capture = true;
        self
    }
}

pub type ProgressFn = Arc<dyn Fn(Option<f64>) + Send + Sync>;

pub struct Ctx {
    pub engines: Arc<Engines>,
    /// Private folder for this job. Results go in `work/out`.
    pub work: PathBuf,
    pub cancel: CancellationToken,
    progress: ProgressFn,
    log: Mutex<VecDeque<String>>,
}

const LOG_LINES: usize = 80;

impl Ctx {
    pub fn new(engines: Arc<Engines>, work: PathBuf, cancel: CancellationToken, progress: ProgressFn) -> Self {
        Ctx { engines, work, cancel, progress, log: Mutex::new(VecDeque::new()) }
    }

    /// A path inside the job's scratch folder (`work/tmp`), created on demand.
    pub fn tmp(&self, name: &str) -> PathBuf {
        let dir = self.work.join("tmp");
        let _ = std::fs::create_dir_all(&dir);
        dir.join(name)
    }

    pub fn report(&self, fraction: Option<f64>) {
        (self.progress)(fraction.map(|f| f.clamp(0.0, 1.0)));
    }

    pub fn log(&self, line: impl Into<String>) {
        let mut log = self.log.lock().unwrap();
        if log.len() >= LOG_LINES {
            log.pop_front();
        }
        log.push_back(line.into());
    }

    pub fn log_text(&self) -> String {
        self.log.lock().unwrap().iter().cloned().collect::<Vec<_>>().join("\n")
    }

    pub fn check_canceled(&self) -> Result<()> {
        if self.cancel.is_cancelled() { Err(Canceled.into()) } else { Ok(()) }
    }

    fn on_line(&self, progress: Progress, is_stdout: bool, line: &str) {
        let line = line.trim_end();
        match progress {
            Progress::Ffmpeg { duration } if is_stdout => {
                if let Some(v) = line.strip_prefix("out_time_us=").or_else(|| line.strip_prefix("out_time_ms="))
                    && let Ok(us) = v.trim().parse::<f64>()
                    && duration > 0.0
                {
                    self.report(Some(us / 1_000_000.0 / duration));
                }
                return; // progress lines are not worth logging
            }
            Progress::Percent => {
                let t = line.trim_start();
                if let Some(pos) = t.find('%')
                    && let Ok(n) = t[..pos].trim().parse::<f64>()
                {
                    self.report(Some(n / 100.0));
                }
            }
            Progress::Pages { total } if total > 0 => {
                if let Some(rest) = line.strip_prefix("Page ")
                    && let Ok(n) = rest.trim().parse::<f64>()
                {
                    self.report(Some(n / total as f64));
                }
            }
            _ => {}
        }
        if !line.trim().is_empty() {
            self.log(line.to_string());
        }
    }

    /// Runs a program to the end. Fails if it exits with an error or the job is canceled.
    pub async fn run(&self, cmd: Cmd) -> Result<String> {
        self.check_canceled()?;
        let name = cmd.program.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let shown: Vec<String> = cmd.args.iter().map(|a| a.to_string_lossy().to_string()).collect();
        self.log(format!("> {name} {}", shown.join(" ")));

        let mut c = tokio::process::Command::new(&cmd.program);
        c.args(&cmd.args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        #[cfg(windows)]
        c.creation_flags(CREATE_NO_WINDOW);
        if let Some(d) = &cmd.cwd {
            c.current_dir(d);
        }
        for (k, v) in &cmd.envs {
            c.env(k, v);
        }
        let mut child = c.spawn().with_context(|| format!("Could not start {name}. The app may be missing some of its files."))?;
        let pid = child.id();

        let (tx, mut rx) = mpsc::unbounded_channel::<(bool, String)>();
        spawn_reader(child.stdout.take().unwrap(), true, tx.clone());
        spawn_reader(child.stderr.take().unwrap(), false, tx);

        let mut captured = String::new();
        let mut last_err = String::new();
        loop {
            tokio::select! {
                msg = rx.recv() => match msg {
                    Some((is_out, line)) => {
                        if cmd.capture && is_out {
                            captured.push_str(&line);
                            captured.push('\n');
                        }
                        if !is_out && !line.trim().is_empty() {
                            last_err = line.trim().to_string();
                        }
                        self.on_line(cmd.progress, is_out, &line);
                    }
                    None => break,
                },
                _ = self.cancel.cancelled() => {
                    kill_tree(pid);
                    let _ = child.wait().await;
                    return Err(Canceled.into());
                }
            }
        }
        let status = tokio::select! {
            s = child.wait() => s?,
            _ = self.cancel.cancelled() => {
                kill_tree(pid);
                let _ = child.wait().await;
                return Err(Canceled.into());
            }
        };
        if !status.success() {
            let detail = if last_err.is_empty() { format!("exit code {}", status.code().unwrap_or(-1)) } else { last_err };
            return Err(anyhow!("{} stopped with an error: {}", engine_title(&name), detail));
        }
        Ok(captured)
    }
}

fn engine_title(exe: &str) -> &'static str {
    match exe.to_lowercase().trim_end_matches(".exe") {
        "ffmpeg" | "ffprobe" => "FFmpeg",
        "magick" => "ImageMagick",
        "gswin64c" => "Ghostscript",
        "soffice" => "LibreOffice",
        "pandoc" => "Pandoc",
        "ebook-convert" => "Calibre",
        "7z" => "7-Zip",
        "qpdf" => "qpdf",
        "resvg" => "resvg",
        "vtracer" => "VTracer",
        "python" => "fontTools",
        _ => "The converter",
    }
}

fn spawn_reader<R: AsyncRead + Unpin + Send + 'static>(r: R, is_out: bool, tx: mpsc::UnboundedSender<(bool, String)>) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(r);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    // Some programs end progress lines with \r only; split those too.
                    for part in String::from_utf8_lossy(&buf).split(['\r', '\n']) {
                        if !part.is_empty() && tx.send((is_out, part.to_string())).is_err() {
                            return;
                        }
                    }
                }
            }
        }
    });
}

/// Stops a program and everything it started (LibreOffice starts a second process).
pub fn kill_tree(pid: Option<u32>) {
    let Some(pid) = pid else { return };
    let mut c = std::process::Command::new("taskkill");
    c.args(["/PID", &pid.to_string(), "/T", "/F"]).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = c.status();
}
