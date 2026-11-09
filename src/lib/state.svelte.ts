import { api } from "./api";
import { DEFAULT_TARGET, TABS, type TabId, tabForCategory } from "./tabs";
import type { FormatInfo, HistoryEntry, Item, JobUpdate, OptionSpec, Registry, Route, Settings } from "./types";

export type View = "list" | "format" | "settings";
export type PdfTool = "merge" | "split" | "arrange" | "images" | "toImages" | "compress";

export const app = $state({
  registry: null as Registry | null,
  settings: null as Settings | null,
  items: [] as Item[],
  history: [] as HistoryEntry[],
  /** PDF tool jobs, by id. */
  jobs: {} as Record<string, JobUpdate>,
  tab: "all" as TabId,
  view: "list" as View,
  formatPage: null as string | null,
  pdfTool: "merge" as PdfTool,
  dragging: false,
  dark: false,
  update: null as { version: string; url: string } | null,
  version: "",
});

// ---------------------------------------------------------------- registry lookups

let formats = new Map<string, FormatInfo>();
let routesFrom = new Map<string, Route[]>();

function setRegistry(r: Registry) {
  formats = new Map(r.formats.map((f) => [f.id, f]));
  routesFrom = new Map();
  for (const route of r.routes) {
    const list = routesFrom.get(route.from) ?? [];
    list.push(route);
    routesFrom.set(route.from, list);
  }
  app.registry = r;
}

export const fmt = (id: string | null | undefined): FormatInfo | undefined => (id ? formats.get(id) : undefined);
export const targets = (from: string | null): Route[] => (from ? (routesFrom.get(from) ?? []) : []);
export const findRoute = (from: string | null, to: string | null): Route | undefined => targets(from).find((r) => r.to === to);
export const optionSpecs = (r: Route | undefined): OptionSpec[] => (r ? (app.registry?.optionSets[r.optionSet] ?? []) : []);
export const sourcesOf = (to: string): Route[] => app.registry?.routes.filter((r) => r.to === to) ?? [];

export function targetLabel(r: Route): string {
  const name = fmt(r.to)?.name ?? r.to;
  if (r.from !== r.to) return name;
  switch (fmt(r.from)?.category) {
    case "pdf":
      return `${name} (compress)`;
    case "image":
      return `${name} (resize or compress)`;
    case "video":
      return `${name} (compress, resize or trim)`;
    case "audio":
      return `${name} (change quality or trim)`;
    default:
      return name;
  }
}

export function itemTab(item: Item): TabId {
  return tabForCategory(fmt(item.format)?.category);
}

export function itemsInTab(tab: TabId): Item[] {
  return tab === "all" ? app.items : app.items.filter((i) => itemTab(i) === tab);
}

function defaultTarget(format: string): string | null {
  const list = targets(format);
  const remembered = app.settings?.lastTargets?.[format];
  for (const want of [remembered, DEFAULT_TARGET[format]]) {
    if (want && list.some((r) => r.to === want)) return want;
  }
  return list.find((r) => r.to !== format)?.to ?? list[0]?.to ?? null;
}

// ---------------------------------------------------------------- start-up

export async function init() {
  const [registry, settings, history, version] = await Promise.all([api.registry(), api.getSettings(), api.history(), api.version()]);
  setRegistry(registry);
  app.settings = settings;
  app.history = history;
  app.version = version;
  applyTheme();
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyTheme);

  await api.onJobUpdate(applyUpdate);
  await api.onDragDrop((e) => {
    if (e.type === "enter" || e.type === "over") app.dragging = true;
    else if (e.type === "leave") app.dragging = false;
    else {
      app.dragging = false;
      if (e.paths.length) handleDrop(e.paths);
    }
  });
  await api.onLaunchFiles(async () => addPaths(await api.takeLaunchFiles()));
  const launch = await api.takeLaunchFiles();
  if (launch.length) await addPaths(launch);
  if (settings.checkUpdates) checkForUpdate();
}

export function applyTheme() {
  const t = app.settings?.theme ?? "system";
  app.dark = t === "dark" || (t === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = app.dark ? "dark" : "light";
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;
export function saveSettings(now = false) {
  if (!app.settings) return;
  clearTimeout(saveTimer);
  const run = async () => {
    try {
      await api.saveSettings($state.snapshot(app.settings!));
    } catch (e) {
      alert(String(e));
      app.settings = await api.getSettings();
    }
  };
  if (now) run();
  else saveTimer = setTimeout(run, 400);
}

async function checkForUpdate() {
  try {
    const r = await fetch("https://api.github.com/repos/faiyaz-svg/convert-everything/releases/latest");
    if (!r.ok) return;
    const j = await r.json();
    const latest = String(j.tag_name ?? "").replace(/^v/, "");
    if (latest && newer(latest, app.version)) app.update = { version: latest, url: j.html_url };
  } catch {
    // Offline is fine: the app works without the internet.
  }
}

function newer(a: string, b: string): boolean {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    if ((pa[i] ?? 0) !== (pb[i] ?? 0)) return (pa[i] ?? 0) > (pb[i] ?? 0);
  }
  return false;
}

// ---------------------------------------------------------------- queue

/** Set by the open PDF tool so drops go to it instead of the queue. */
export const drop = { handler: null as ((paths: string[]) => void) | null };

function handleDrop(paths: string[]) {
  if (app.view === "list" && app.tab === "pdf" && drop.handler) drop.handler(paths);
  else addPaths(paths);
}

export async function addPaths(paths: string[]) {
  if (!paths.length) return;
  const probed = await api.probe(paths);
  const known = new Set(app.items.map((i) => i.path));
  const added: Item[] = [];
  for (const p of probed) {
    if (known.has(p.path)) continue;
    known.add(p.path);
    added.push({
      id: crypto.randomUUID(),
      path: p.path,
      name: p.name,
      size: p.size,
      format: p.format,
      to: p.format ? defaultTarget(p.format) : null,
      options: {},
      state: "ready",
      progress: null,
      message: null,
      outputs: [],
      log: null,
      showOptions: false,
      showLog: false,
    });
  }
  if (!added.length) return;
  app.items.push(...added);
  // Open the tab for these files, unless files waiting in the list belong to
  // different tabs (a right-click on several files sends them one at a time).
  const waiting = app.items.filter((i) => i.state === "ready");
  const tabs = new Set(waiting.map(itemTab));
  app.tab = tabs.size === 1 ? [...tabs][0] : "all";
  app.view = "list";
}

export function setTarget(item: Item, to: string) {
  item.to = to;
  item.options = {};
  if (item.state !== "running" && item.state !== "queued") resetItem(item);
  if (app.settings && item.format) {
    app.settings.lastTargets[item.format] = to;
    saveSettings();
  }
}

/** Row edits live here, not in components, so the item list has one owner. */
export function toggleItem(item: Item, field: "showOptions" | "showLog") {
  item[field] = !item[field];
}

export function setOption(item: Item, key: string, value: unknown) {
  item.options[key] = value;
}

export function resetOptions(item: Item) {
  item.options = {};
}

export function copyOptions(from: Item, to: Item[]) {
  const snap = $state.snapshot(from.options);
  for (const o of to) o.options = { ...snap };
}

function resetItem(item: Item) {
  item.state = "ready";
  item.progress = null;
  item.message = null;
  item.outputs = [];
  item.log = null;
  item.showLog = false;
}

/** Option values with defaults filled in, empty values left out. */
export function resolvedOptions(item: Item): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const spec of optionSpecs(findRoute(item.format, item.to))) {
    const v = item.options[spec.key] ?? spec.default;
    if (v !== null && v !== undefined && v !== "") out[spec.key] = v;
  }
  return out;
}

export const canConvert = (i: Item) => !!findRoute(i.format, i.to) && (i.state === "ready" || i.state === "failed" || i.state === "canceled");

export async function convertItems(items: Item[]) {
  const ready = items.filter(canConvert);
  if (!ready.length) return;
  for (const i of ready) {
    resetItem(i);
    i.state = "queued";
    i.id = crypto.randomUUID();
  }
  await api.start(ready.map((i) => ({ id: i.id, input: i.path, from: i.format!, to: i.to!, options: resolvedOptions(i) })));
}

export async function cancelItem(item: Item) {
  await api.cancel(item.id);
}

export function removeItem(item: Item) {
  if (item.state === "running" || item.state === "queued") return;
  app.items = app.items.filter((i) => i !== item);
}

export function clearTab(tab: TabId) {
  const shown = new Set(itemsInTab(tab));
  app.items = app.items.filter((i) => !shown.has(i) || i.state === "running" || i.state === "queued");
}

function applyUpdate(u: JobUpdate) {
  const item = app.items.find((i) => i.id === u.id);
  if (!item) {
    app.jobs[u.id] = u;
    if (u.state === "done") refreshHistory();
    return;
  }
  item.state = u.state;
  item.progress = u.progress;
  if (u.message !== null) item.message = u.message;
  if (u.outputs.length) item.outputs = u.outputs;
  if (u.log) item.log = u.log;
  if (u.state === "done") refreshHistory();
}

let historyTimer: ReturnType<typeof setTimeout> | undefined;
function refreshHistory() {
  clearTimeout(historyTimer);
  historyTimer = setTimeout(async () => {
    app.history = await api.history();
  }, 300);
}

export function openTab(id: TabId) {
  app.tab = id;
  app.view = "list";
  document.getElementById("main")?.scrollTo(0, 0);
}

export function openFormat(id: string) {
  app.formatPage = id;
  app.view = "format";
  const cat = fmt(id)?.category;
  if (cat && cat !== "folder") app.tab = tabForCategory(cat);
  document.getElementById("main")?.scrollTo(0, 0);
}

export function tabCount(id: TabId): number {
  if (id === "pdf") return 0;
  return itemsInTab(id).length;
}

export function formatsInTab(id: TabId): FormatInfo[] {
  const tab = TABS.find((t) => t.id === id);
  if (!tab || !app.registry) return [];
  return app.registry.formats.filter((f) => tab.categories.includes(f.category) && f.canRead);
}

export function humanSize(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} bytes`;
}

export function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
