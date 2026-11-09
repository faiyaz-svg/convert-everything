// A stand-in for the Rust side when the UI runs in a normal browser (npm run dev).
// It uses the real format table (src/lib/mock/registry.json) and pretends to convert.

import type { Api } from "./api";
import type { EngineStatus, HistoryEntry, JobUpdate, ProbedFile, Registry, Settings } from "./types";

const listeners: ((u: JobUpdate) => void)[] = [];

function emit(u: Partial<JobUpdate> & { id: string; state: JobUpdate["state"] }) {
  const full: JobUpdate = { progress: null, message: null, outputs: [], log: null, ...u };
  listeners.forEach((l) => l(full));
}

let settings: Settings = {
  theme: "system",
  outputMode: "source",
  outputFolder: "",
  parallel: 4,
  checkUpdates: false,
  explorerMenu: true,
  lastTargets: {},
};

const history: HistoryEntry[] = [
  { time: 1762700000, input: "C:\\Users\\you\\Pictures\\IMG_2041.heic", output: "C:\\Users\\you\\Pictures\\IMG_2041.jpg", from: "heic", to: "jpg" },
  { time: 1762690000, input: "C:\\Users\\you\\Documents\\Budget 2026.xlsx", output: "C:\\Users\\you\\Documents\\Budget 2026.pdf", from: "xlsx", to: "pdf" },
  { time: 1762680000, input: "C:\\Users\\you\\Music\\Interview.wav", output: "C:\\Users\\you\\Music\\Interview.mp3", from: "wav", to: "mp3" },
];

let registry: Registry | null = null;

const fakeSize = (path: string) => 200_000 + ((path.length * 7919) % 9_000_000);
const kb = (bytes: number) => (bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.round(bytes / 1024)} KB`);

function formatFor(name: string): string | null {
  const lower = name.toLowerCase();
  let best: { id: string; len: number } | null = null;
  for (const f of registry?.formats ?? []) {
    if (f.id === "alac") continue;
    for (const e of f.exts) {
      if (lower.endsWith("." + e) && (!best || e.length > best.len)) best = { id: f.id, len: e.length };
    }
  }
  return best?.id ?? null;
}

export const DEMO_FILES = [
  "C:\\Users\\you\\Pictures\\IMG_2041.heic",
  "C:\\Users\\you\\Pictures\\IMG_2042.heic",
  "C:\\Users\\you\\Videos\\Screen recording.mov",
  "C:\\Users\\you\\Documents\\Cover letter.docx",
  "C:\\Users\\you\\Downloads\\The Time Machine.mobi",
  "C:\\Users\\you\\Downloads\\photos-backup.rar",
  "C:\\Users\\you\\Documents\\contacts.csv",
  "C:\\Users\\you\\Downloads\\Inter-Regular.ttf",
];

export async function mockApi(): Promise<Api> {
  registry = (await import("./mock/registry.json")).default as unknown as Registry;
  return {
    registry: async () => registry!,
    probe: async (paths) =>
      paths.map<ProbedFile>((p) => ({
        path: p,
        name: p.split(/[\\/]/).pop() ?? p,
        size: fakeSize(p),
        format: formatFor(p),
      })),
    start: async (requests) => {
      for (const r of requests) {
        emit({ id: r.id, state: "queued" });
        let p = 0;
        const timer = setInterval(() => {
          p += 0.08 + Math.random() * 0.1;
          if (p >= 1) {
            clearInterval(timer);
            const out = r.input.replace(/\.[^.\\]+$/, "") + "." + (registry!.formats.find((f) => f.id === r.to)?.exts[0] ?? r.to);
            const size = fakeSize(r.input);
            emit({ id: r.id, state: "done", progress: 1, message: `${kb(size)} to ${kb(size * 0.3)}`, outputs: [out] });
          } else {
            emit({ id: r.id, state: "running", progress: r.to === "folder" ? null : p });
          }
        }, 250);
      }
    },
    startPdf: async (req) => {
      emit({ id: req.id, state: "running", progress: null });
      setTimeout(() => emit({ id: req.id, state: "done", progress: 1, outputs: ["C:\\Users\\you\\Documents\\Merged.pdf"] }), 900);
    },
    cancel: async (id) => emit({ id, state: "canceled" }),
    pdfInfo: async () => ({ pages: 6, thumbnails: [] }),
    getSettings: async () => ({ ...settings }),
    saveSettings: async (s) => {
      settings = { ...s };
    },
    history: async () => [...history],
    clearHistory: async () => {
      history.length = 0;
    },
    engines: async () =>
      (
        [
          ["ffmpeg", "FFmpeg", "9.0.2", "GPL-3.0-or-later"],
          ["imagemagick", "ImageMagick", "7.1.2-35", "ImageMagick License"],
          ["ghostscript", "Ghostscript", "10.08.0", "AGPL-3.0-or-later"],
          ["libreoffice", "LibreOffice", "26.8.2.2", "MPL-2.0"],
          ["pandoc", "Pandoc", "3.13", "GPL-2.0-or-later"],
          ["calibre", "Calibre", "9.18.0", "GPL-3.0"],
        ] as const
      ).map<EngineStatus>(([name, title, version, license]) => ({ name, title, version, license, source: "", found: true })),
    licensesFolder: async () => "C:\\Program Files\\Convert Everything\\engines\\licenses",
    takeLaunchFiles: async () => (new URLSearchParams(location.search).has("demo") ? [...DEMO_FILES] : []),
    onJobUpdate: async (cb) => {
      listeners.push(cb);
      return () => listeners.splice(listeners.indexOf(cb), 1);
    },
    onLaunchFiles: async () => () => {},
    onDragDrop: async () => () => {},
    pickFiles: async () => DEMO_FILES.slice(0, 3),
    pickFolder: async () => "C:\\Users\\you\\Converted",
    reveal: async () => {},
    openPath: async () => {},
    openUrl: async (url) => {
      window.open(url, "_blank");
    },
    version: async () => "0.1.1",
  };
}
