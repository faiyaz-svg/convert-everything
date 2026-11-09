# App design

The app should feel like W3Schools: flat, plain, quick to scan, with the work doing the talking. It copies the look of the Math and CS Foundations textbook sites and should not look like a generic AI-generated app. All styles live in `src/styles/site.css`. Colours are CSS variables on `:root`, with a dark set under `:root[data-theme="dark"]`.

## Layout

- A white header with the app name ("Convert" in dark text, "Everything" in green), a "Find a format" search box, the light/dark switch and Settings.
- A dark bar of tabs under the header (`#282a35`, white uppercase labels): All, Images, Audio, Video, Documents, Ebooks, PDF tools, Archives, Data, Fonts. The current tab has a light gray background with dark text. A small number after a label counts the files waiting in that tab. The labels must fit on one line at the 960px minimum window width.
- A light gray left sidebar (`#ebedee`). In a file tab it lists that tab's formats; each opens a reference page, like a W3Schools reference. In PDF tools it lists the tools. Under that, "Recent" lists the last conversions. The current item is white with a green left border.
- The page title is large and light (weight 400). Section headings are plain and smaller.
- Under every page title there is an outlined button on the left (Clear list, Back) and a green button on the right (Convert, Merge, Split; `#04aa6d`). The pair repeats at the bottom when the list is long.
- A dark footer says that files never leave the computer, with the version, license files and source link.

## Boxes

| Box | Look | Used for |
|---|---|---|
| Drop zone | Gray background, 5px corners; green dashed border while files are dragged over the window | Adding files |
| Note | Pale yellow, no icon | Tips about the chosen conversions |
| Warning | Pale red with a red left border, no icon | Errors |
| Panel | White, thin border, green left border | "More options", finished PDF tool jobs |
| Table | 1px border, gray header row, thin row lines | The file list, engines, format reference |

## Colour

One dark colour (tab bar and footer), one accent (green), and gray neutrals. Links are dark green and underlined. Inline code is dark red on light gray. Dark mode keeps the same layout with dark grays and a slightly lighter green; no neon and no glow. Buttons stay `#04aa6d` in both modes so white text stays readable.

## Text

All text in the app follows `STYLE.md`: sentence case, plain words, no marketing language and no em dashes.

## Things to avoid

These are common signs of AI-generated design. Don't add any of them.

- Gradients of any kind, glow or aurora backgrounds, glass blur effects, neon colours on dark backgrounds.
- Purple, indigo or violet anywhere.
- Emoji used as icons, bullets or headings. Buttons are words.
- A centered "hero" section with a pill badge, a big tagline and three feature cards with icons.
- The Inter or Geist fonts. The app uses the system font stack (Segoe UI on Windows).
- Drop shadows, corners rounder than 5px, and cards nested inside cards.
- Marketing words: "unlock", "elevate", "master", "supercharge", "seamless", "your journey", "level up".
- Giving every element the same visual weight. The title leads, then headings, then body text.

## Checking a change

Run `npm run dev` and open `http://127.0.0.1:1420/?demo=1` in a browser. The `demo` flag fills the list with sample files and the mock back end (`src/lib/mock.ts`) pretends to convert them. Look at the window at 1000x700 and 1400x900, in light and dark mode. Check that the tabs fit, the buttons line up, the boxes match the table above and nothing from the avoid list crept in. Then check the real thing with `npm run tauri dev`.
