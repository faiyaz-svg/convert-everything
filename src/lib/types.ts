export type Category =
  | "image"
  | "audio"
  | "video"
  | "subtitle"
  | "document"
  | "spreadsheet"
  | "presentation"
  | "pdf"
  | "ebook"
  | "archive"
  | "data"
  | "font"
  | "folder";

export interface FormatInfo {
  id: string;
  name: string;
  category: Category;
  exts: string[];
  description: string;
  canRead: boolean;
  canWrite: boolean;
}

export interface Route {
  from: string;
  to: string;
  handler: string;
  note?: string;
  optionSet: string;
}

export interface Choice {
  value: string;
  label: string;
}

export interface OptionSpec {
  key: string;
  label: string;
  kind: "select" | "number" | "text" | "toggle";
  choices?: Choice[];
  default: unknown;
  min?: number;
  max?: number;
  unit?: string;
  help?: string;
  placeholder?: string;
}

export interface Registry {
  formats: FormatInfo[];
  routes: Route[];
  optionSets: Record<string, OptionSpec[]>;
}

export interface ProbedFile {
  path: string;
  name: string;
  size: number;
  format: string | null;
}

export type JobState = "ready" | "queued" | "running" | "done" | "failed" | "canceled";

export interface JobUpdate {
  id: string;
  state: Exclude<JobState, "ready">;
  progress: number | null;
  message: string | null;
  outputs: string[];
  log: string | null;
}

export interface Item {
  id: string;
  path: string;
  name: string;
  size: number;
  format: string | null;
  to: string | null;
  options: Record<string, unknown>;
  state: JobState;
  progress: number | null;
  message: string | null;
  outputs: string[];
  log: string | null;
  showOptions: boolean;
  showLog: boolean;
}

export interface Settings {
  theme: "system" | "light" | "dark";
  outputMode: "source" | "folder";
  outputFolder: string;
  parallel: number;
  checkUpdates: boolean;
  explorerMenu: boolean;
  lastTargets: Record<string, string>;
}

export interface HistoryEntry {
  time: number;
  input: string;
  output: string;
  from: string;
  to: string;
}

export interface EngineStatus {
  name: string;
  title: string;
  version: string;
  license: string;
  source: string;
  found: boolean;
}

export interface PdfInfo {
  pages: number;
  thumbnails: string[];
}

export interface ConvertRequest {
  id: string;
  input: string;
  from: string;
  to: string;
  options: Record<string, unknown>;
}

export type PdfRequest =
  | { tool: "merge"; id: string; inputs: string[]; name: string }
  | { tool: "imagesToPdf"; id: string; inputs: string[]; name: string }
  | { tool: "split"; id: string; input: string; ranges: string }
  | { tool: "arrange"; id: string; input: string; pages: { page: number; rotate: number }[] };

export interface DragEvent {
  type: "enter" | "over" | "drop" | "leave";
  paths: string[];
}
