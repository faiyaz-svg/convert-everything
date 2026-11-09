import type { Category } from "./types";

export type TabId = "all" | "images" | "audio" | "video" | "documents" | "ebooks" | "pdf" | "archives" | "data" | "fonts";

export interface Tab {
  id: TabId;
  label: string;
  title: string;
  intro: string;
  categories: Category[];
}

export const TABS: Tab[] = [
  {
    id: "all",
    label: "All",
    title: "All files",
    intro: "Drop any files here. Each one is checked and sent to the right tab, with the formats it can become.",
    categories: [],
  },
  {
    id: "images",
    label: "Images",
    title: "Images",
    intro: "Photos, screenshots and drawings. iPhone HEIC photos become JPG files that open anywhere. Pick the same format to shrink or resize.",
    categories: ["image"],
  },
  {
    id: "audio",
    label: "Audio",
    title: "Audio",
    intro: "Music and recordings. Change the format, lower the bitrate for a smaller file, or cut out a part.",
    categories: ["audio"],
  },
  {
    id: "video",
    label: "Video",
    title: "Video",
    intro: "Change the format, make a file smaller, save the sound only, or turn a short clip into a GIF. Subtitle files are here too.",
    categories: ["video", "subtitle"],
  },
  {
    id: "documents",
    label: "Documents",
    title: "Documents",
    intro: "Word, Excel and PowerPoint files, PDFs, Markdown and other text documents.",
    categories: ["document", "spreadsheet", "presentation", "pdf"],
  },
  {
    id: "ebooks",
    label: "Ebooks",
    title: "Ebooks",
    intro: "EPUB, Kindle and comic book files. Make a Kindle book readable on other devices, or the other way around.",
    categories: ["ebook"],
  },
  {
    id: "pdf",
    label: "PDF tools",
    title: "PDF tools",
    intro: "",
    categories: [],
  },
  {
    id: "archives",
    label: "Archives",
    title: "Archives",
    intro: "ZIP, 7Z, RAR and other archives. Unpack them into a folder or repack them as another type.",
    categories: ["archive"],
  },
  {
    id: "data",
    label: "Data",
    title: "Data",
    intro: "Tables and structured data: CSV, JSON, YAML, TOML, XML and Excel sheets.",
    categories: ["data"],
  },
  {
    id: "fonts",
    label: "Fonts",
    title: "Fonts",
    intro: "Desktop fonts (TTF, OTF) and web fonts (WOFF, WOFF2).",
    categories: ["font"],
  },
];

export function tabById(id: TabId): Tab {
  return TABS.find((t) => t.id === id) ?? TABS[0];
}

export function tabForCategory(c: Category | undefined): TabId {
  if (!c) return "all";
  return TABS.find((t) => t.categories.includes(c))?.id ?? "all";
}

export const CATEGORY_NAMES: Record<Category, string> = {
  image: "Image",
  audio: "Audio",
  video: "Video",
  subtitle: "Subtitles",
  document: "Document",
  spreadsheet: "Spreadsheet",
  presentation: "Presentation",
  pdf: "PDF",
  ebook: "Ebook",
  archive: "Archive",
  data: "Data",
  font: "Font",
  folder: "Folder",
};

/** What a format usually becomes when the user has not picked anything yet. */
export const DEFAULT_TARGET: Record<string, string> = {
  heic: "jpg", png: "jpg", jpg: "png", webp: "png", avif: "jpg", gif: "mp4", bmp: "png", tiff: "jpg",
  ico: "png", svg: "png", psd: "png", jxl: "jpg", tga: "png", raw: "jpg", eps: "png",
  mp3: "wav", wav: "mp3", flac: "mp3", m4a: "mp3", ogg: "mp3", opus: "mp3", aiff: "mp3", wma: "mp3",
  mp4: "mp3", mkv: "mp4", mov: "mp4", webm: "mp4", avi: "mp4", wmv: "mp4", flv: "mp4", mpeg: "mp4", ts: "mp4", "3gp": "mp4",
  srt: "vtt", vtt: "srt", ass: "srt",
  docx: "pdf", doc: "docx", odt: "docx", rtf: "docx", txt: "pdf", md: "docx", html: "pdf", tex: "pdf", rst: "html", org: "md",
  pdf: "docx", xlsx: "csv", xls: "xlsx", ods: "xlsx", pptx: "pdf", ppt: "pptx", odp: "pptx",
  epub: "pdf", mobi: "epub", azw3: "epub", fb2: "epub", pdb: "epub", lit: "epub", cbz: "pdf", cbr: "pdf",
  zip: "folder", "7z": "zip", tar: "zip", tgz: "zip", txz: "zip", tbz2: "zip", rar: "zip", iso: "folder", cab: "folder",
  gz: "folder", bz2: "folder", xz: "folder",
  csv: "xlsx", tsv: "csv", json: "csv", yaml: "json", toml: "json", xml: "json",
  ttf: "woff2", otf: "ttf", woff: "ttf", woff2: "ttf",
};
