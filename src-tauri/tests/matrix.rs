//! Runs every conversion route through the real engines.
//!
//!   npm run test:convert
//!
//! It makes a few sample files with the engines, converts each sample to every
//! format it can become, and uses those results as samples for the next round.
//! Results go to tests/out (ignored by git). Needs the engines (npm run engines).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use convert_everything_lib::core::convert;
use convert_everything_lib::core::detect;
use convert_everything_lib::core::engines::Engines;
use convert_everything_lib::core::formats::{self, Category};
use convert_everything_lib::core::options::Opts;
use convert_everything_lib::core::routes;
use convert_everything_lib::core::run::{Cmd, Ctx};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

const TIMEOUT: Duration = Duration::from_secs(240);

/// Samples that should come from a particular route, because the first one to
/// finish may be a poor example (a PDF made from a picture has no text).
const PREFERRED: &[(&str, &str)] = &[("md", "pdf"), ("md", "docx"), ("md", "epub")];

fn ctx(engines: &Arc<Engines>, work: PathBuf) -> Ctx {
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    Ctx::new(engines.clone(), work, CancellationToken::new(), Arc::new(|_| {}))
}

async fn make_seeds(engines: &Arc<Engines>, dir: &Path) -> HashMap<&'static str, PathBuf> {
    std::fs::create_dir_all(dir).unwrap();
    let c = ctx(engines, dir.join("_work"));
    let p = |name: &str| dir.join(name);
    let mut seeds = HashMap::new();

    let text_seeds: &[(&str, &str, &str)] = &[
        ("md", "sample.md", "# Sample document\n\nSome **bold** text and a list:\n\n- one\n- two\n\n| a | b |\n|---|---|\n| 1 | 2 |\n"),
        ("html", "sample.html", "<!doctype html><html><head><title>Sample</title></head><body><h1>Sample</h1><p>Hello <b>world</b>.</p></body></html>"),
        ("csv", "sample.csv", "name,age,city\nAda,36,London\nAlan,41,Wilmslow\n"),
        ("json", "sample.json", "[{\"name\": \"Ada\", \"age\": 36, \"address\": {\"city\": \"London\"}}, {\"name\": \"Alan\", \"age\": 41}]"),
        ("yaml", "sample.yaml", "people:\n  - name: Ada\n    age: 36\n  - name: Alan\n    age: 41\n"),
        ("toml", "sample.toml", "title = \"Sample\"\n\n[owner]\nname = \"Ada\"\nborn = 1815-12-10\n"),
        ("xml", "sample.xml", "<people><person id=\"1\"><name>Ada</name></person><person id=\"2\"><name>Alan</name></person></people>"),
        ("srt", "sample.srt", "1\n00:00:00,500 --> 00:00:02,000\nHello there.\n\n2\n00:00:02,500 --> 00:00:04,000\nA second line.\n"),
        ("svg", "sample.svg", "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"80\"><rect width=\"120\" height=\"80\" fill=\"#04aa6d\"/><circle cx=\"60\" cy=\"40\" r=\"25\" fill=\"#ffffff\"/></svg>"),
    ];
    for (id, name, text) in text_seeds {
        std::fs::write(p(name), text).unwrap();
        seeds.insert(*id, p(name));
    }

    let magick = || Cmd::new(engines.magick());
    c.run(magick().arg("rose:").args(["-resize", "280x"]).arg(p("sample.png"))).await.unwrap();
    seeds.insert("png", p("sample.png"));
    for (id, name) in [("psd", "sample.psd"), ("tga", "sample.tga"), ("eps", "sample.eps")] {
        c.run(magick().arg(p("sample.png")).arg(p(name))).await.unwrap();
        seeds.insert(id, p(name));
    }

    let ff = || Cmd::new(engines.ffmpeg()).args(["-hide_banner", "-loglevel", "error", "-y"]);
    c.run(ff().args(["-f", "lavfi", "-i", "sine=frequency=440:duration=3"]).arg(p("sample.wav"))).await.unwrap();
    seeds.insert("wav", p("sample.wav"));
    c.run(ff().args(["-i"]).arg(p("sample.wav")).args(["-c:a", "wmav2"]).arg(p("sample.wma"))).await.unwrap();
    seeds.insert("wma", p("sample.wma"));
    c.run(
        ff().args(["-f", "lavfi", "-i", "testsrc=duration=3:size=320x240:rate=25", "-f", "lavfi", "-i", "sine=duration=3"])
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest"])
            .arg(p("sample.mp4")),
    )
    .await
    .unwrap();
    seeds.insert("mp4", p("sample.mp4"));

    let seven = || Cmd::new(engines.sevenzip()).args(["a", "-y", "-bso0", "-bsp0"]);
    c.run(seven().arg("-tzip").arg(p("sample.zip")).arg(p("sample.md")).arg(p("sample.png"))).await.unwrap();
    seeds.insert("zip", p("sample.zip"));
    c.run(seven().arg("-tzip").arg(p("sample.cbz")).arg(p("sample.png"))).await.unwrap();
    seeds.insert("cbz", p("sample.cbz"));
    for (id, kind) in [("gz", "gzip"), ("bz2", "bzip2"), ("xz", "xz")] {
        let out = p(&format!("sample.md.{id}"));
        c.run(seven().arg(format!("-t{kind}")).arg(&out).arg(p("sample.md"))).await.unwrap();
        seeds.insert(id, out);
    }

    // Real files that can't be made here (HEIC, camera RAW, RAR...) can be added with
    // CE_EXTRA_SAMPLES=<folder>. Each file is used as the sample for its format.
    if let Some(extra) = std::env::var_os("CE_EXTRA_SAMPLES") {
        for e in std::fs::read_dir(extra).unwrap().flatten() {
            if let Some(f) = detect::detect(&e.path()) {
                seeds.insert(f.id, e.path());
            }
        }
    }

    for font in ["C:\\Windows\\Fonts\\arial.ttf", "C:\\Windows\\Fonts\\segoeui.ttf"] {
        if Path::new(font).exists() {
            std::fs::copy(font, p("sample.ttf")).unwrap();
            seeds.insert("ttf", p("sample.ttf"));
            break;
        }
    }
    seeds
}

/// Presentations can only be made from other presentations, so make one from a PDF.
async fn presentation_seed(engines: &Arc<Engines>, pdf: &Path, dir: &Path) -> Option<PathBuf> {
    let c = ctx(engines, dir.join("_work-odp"));
    let profile = dir.join("_lo-profile");
    let url = format!("file:///{}", profile.to_string_lossy().replace('\\', "/").replace(' ', "%20"));
    let cmd = Cmd::new(engines.soffice())
        .arg(format!("-env:UserInstallation={url}"))
        .args(["--headless", "--norestore", "--infilter=impress_pdf_import", "--convert-to", "odp", "--outdir"])
        .arg(dir)
        .arg(pdf);
    c.run(cmd).await.ok()?;
    let odp = dir.join(format!("{}.odp", pdf.file_stem()?.to_string_lossy()));
    odp.exists().then_some(odp)
}

#[derive(Debug)]
enum Outcome {
    Pass(Duration),
    Fail(String),
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore]
async fn every_route() {
    let engines = Arc::new(Engines::from_env_or_dev());
    assert!(engines.ffmpeg().exists(), "Engines missing. Run: npm run engines");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/out");
    let _ = std::fs::remove_dir_all(&root);
    let seeds_dir = root.join("seeds");

    let filter: Option<String> = std::env::var("CE_ONLY").ok();
    let mut samples = make_seeds(&engines, &seeds_dir).await;
    let mut results: BTreeMap<(String, String), Outcome> = BTreeMap::new();
    let mut tried: HashSet<(&str, &str)> = HashSet::new();
    let sem = Arc::new(Semaphore::new(6));
    let started = Instant::now();

    for round in 1..=6 {
        if round == 3
            && !samples.contains_key("odp")
            && let Some(pdf) = samples.get("pdf").cloned()
            && let Some(odp) = presentation_seed(&engines, &pdf, &seeds_dir).await
        {
            samples.insert("odp", odp);
        }
        let todo: Vec<_> = routes::registry()
            .routes
            .iter()
            .filter(|r| samples.contains_key(r.from) && !tried.contains(&(r.from, r.to)))
            .filter(|r| filter.as_deref().is_none_or(|f| r.from.contains(f) || r.to.contains(f)))
            .collect();
        if todo.is_empty() {
            break;
        }
        println!("round {round}: {} conversions", todo.len());
        let mut set = tokio::task::JoinSet::new();
        for r in todo {
            tried.insert((r.from, r.to));
            let input = samples[r.from].clone();
            let (engines, sem, root) = (engines.clone(), sem.clone(), root.clone());
            set.spawn(async move {
                let _permit = sem.acquire_owned().await.unwrap();
                let work = root.join("_work").join(format!("{}-{}", r.from, r.to));
                let c = ctx(&engines, work.clone());
                let from = formats::by_id(r.from).unwrap();
                let to = formats::by_id(r.to).unwrap();
                let t0 = Instant::now();
                let res = tokio::time::timeout(TIMEOUT, convert::convert(&c, &input, from, to, &Opts::default())).await;
                let outcome = match res {
                    Err(_) => {
                        c.cancel.cancel();
                        (Outcome::Fail("timed out".into()), vec![])
                    }
                    Ok(Err(e)) => (Outcome::Fail(format!("{e:#}\n{}", c.log_text())), vec![]),
                    Ok(Ok(files)) => {
                        let dest = root.join("out").join(format!("{}-to-{}", r.from, r.to));
                        std::fs::create_dir_all(&dest).unwrap();
                        let moved: Vec<PathBuf> = files
                            .iter()
                            .map(|f| {
                                let d = dest.join(f.file_name().unwrap());
                                std::fs::rename(f, &d).unwrap();
                                d
                            })
                            .collect();
                        let main = moved[0].clone();
                        let check = if main.is_file() && to.category != Category::Folder {
                            match detect::detect(&main) {
                                Some(f) if f.category == to.category || f.id == to.id => Ok(()),
                                Some(f) => Err(format!("output looks like {} not {}", f.name, to.name)),
                                None => Ok(()),
                            }
                        } else {
                            Ok(())
                        };
                        match check {
                            Ok(()) => (Outcome::Pass(t0.elapsed()), moved),
                            Err(e) => (Outcome::Fail(e), vec![]),
                        }
                    }
                };
                let _ = std::fs::remove_dir_all(&work);
                (r.from, r.to, outcome)
            });
        }
        while let Some(joined) = set.join_next().await {
            let (from, to, (outcome, files)) = joined.unwrap();
            if let (Outcome::Pass(_), Some(main)) = (&outcome, files.first())
                && main.is_file()
                && (!samples.contains_key(to) || PREFERRED.contains(&(from, to)))
            {
                samples.insert(to, main.clone());
            }
            match &outcome {
                Outcome::Pass(d) => println!("  ok    {from:>6} -> {to:<7} {:>6.1}s", d.as_secs_f64()),
                Outcome::Fail(e) => println!("  FAIL  {from:>6} -> {to:<7} {}", e.lines().next().unwrap_or("")),
            }
            results.insert((from.to_string(), to.to_string()), outcome);
        }
    }

    let total = routes::registry().routes.len();
    let failed: Vec<_> = results.iter().filter(|(_, o)| matches!(o, Outcome::Fail(_))).collect();
    let untested: BTreeMap<&str, usize> = routes::registry().routes.iter().filter(|r| !tried.contains(&(r.from, r.to))).fold(BTreeMap::new(), |mut m, r| {
        *m.entry(r.from).or_default() += 1;
        m
    });
    println!(
        "\n{} routes, {} tested, {} passed, {} failed, in {:.0}s",
        total,
        results.len(),
        results.len() - failed.len(),
        failed.len(),
        started.elapsed().as_secs_f64()
    );
    println!("Not tested (no sample of the input format): {untested:?}");
    for ((from, to), o) in &failed {
        if let Outcome::Fail(e) = o {
            println!("\n--- {from} -> {to}\n{}", e.lines().take(12).collect::<Vec<_>>().join("\n"));
        }
    }
    assert!(failed.is_empty(), "{} conversions failed", failed.len());
}
