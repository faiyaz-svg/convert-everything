// Calls into the Rust side. Outside Tauri (plain `npm run dev` in a browser) a mock
// stands in, so the interface can be worked on and screenshotted without the engines.

import type {
  ConvertRequest,
  DragEvent,
  EngineStatus,
  HistoryEntry,
  JobUpdate,
  PdfInfo,
  PdfRequest,
  ProbedFile,
  Registry,
  Settings,
} from "./types";

export interface Api {
  registry(): Promise<Registry>;
  probe(paths: string[]): Promise<ProbedFile[]>;
  start(requests: ConvertRequest[]): Promise<void>;
  startPdf(request: PdfRequest): Promise<void>;
  cancel(id: string): Promise<void>;
  pdfInfo(path: string, thumbnails: boolean): Promise<PdfInfo>;
  getSettings(): Promise<Settings>;
  saveSettings(s: Settings): Promise<void>;
  history(): Promise<HistoryEntry[]>;
  clearHistory(): Promise<void>;
  engines(): Promise<EngineStatus[]>;
  licensesFolder(): Promise<string>;
  takeLaunchFiles(): Promise<string[]>;
  onJobUpdate(cb: (u: JobUpdate) => void): Promise<() => void>;
  onLaunchFiles(cb: () => void): Promise<() => void>;
  onDragDrop(cb: (e: DragEvent) => void): Promise<() => void>;
  pickFiles(filter?: { name: string; extensions: string[] }): Promise<string[]>;
  pickFolder(): Promise<string | null>;
  reveal(path: string): Promise<void>;
  openPath(path: string): Promise<void>;
  openUrl(url: string): Promise<void>;
  version(): Promise<string>;
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriApi(): Promise<Api> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  const { getVersion } = await import("@tauri-apps/api/app");
  const dialog = await import("@tauri-apps/plugin-dialog");
  const opener = await import("@tauri-apps/plugin-opener");

  return {
    registry: () => invoke("get_registry"),
    probe: (paths) => invoke("probe_files", { paths }),
    start: (requests) => invoke("start_conversions", { requests }),
    startPdf: (request) => invoke("start_pdf_tool", { request }),
    cancel: (id) => invoke("cancel_job", { id }),
    pdfInfo: (path, thumbnails) => invoke("pdf_info", { path, thumbnails }),
    getSettings: () => invoke("get_settings"),
    saveSettings: (settings) => invoke("save_settings", { settings }),
    history: () => invoke("get_history"),
    clearHistory: () => invoke("clear_history"),
    engines: () => invoke("engine_status"),
    licensesFolder: () => invoke("licenses_folder"),
    takeLaunchFiles: () => invoke("take_launch_files"),
    onJobUpdate: (cb) => listen<JobUpdate>("job-update", (e) => cb(e.payload)),
    onLaunchFiles: (cb) => listen("launch-files", () => cb()),
    onDragDrop: (cb) =>
      getCurrentWebview().onDragDropEvent((e) => {
        const p = e.payload;
        cb({ type: p.type, paths: "paths" in p ? p.paths : [] });
      }),
    pickFiles: async (filter) => {
      const r = await dialog.open({ multiple: true, directory: false, filters: filter ? [filter] : undefined });
      if (!r) return [];
      return Array.isArray(r) ? r : [r];
    },
    pickFolder: async () => {
      const r = await dialog.open({ multiple: false, directory: true });
      return typeof r === "string" ? r : null;
    },
    reveal: (path) => opener.revealItemInDir(path),
    openPath: (path) => opener.openPath(path),
    openUrl: (url) => opener.openUrl(url),
    version: () => getVersion(),
  };
}

export const api: Api = await (inTauri ? tauriApi() : import("./mock").then((m) => m.mockApi()));
