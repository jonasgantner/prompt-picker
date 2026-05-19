# Prompt Picker

A macOS menubar utility for composing LLM prompts from reusable components. Press a keyboard shortcut, search your prompt library, select and combine pieces, and get the result on your clipboard — ready to paste.

Built for people who maintain prompt collections as markdown files (works great with Obsidian vaults).

## How it works

You keep your prompts as `.md` files in one or more folders on disk. prompt-picker indexes them, lets you search and select components, resolves dependencies between them, and copies the combined result to your clipboard.

**Open the picker** — press `Cmd+Shift+P` (configurable) or click the menubar icon.

**Search** — start typing to fuzzy-search by name. Type `#coding` to filter by tag.

**Select** — press `Tab` to add the highlighted prompt to the staging area. If it depends on other prompts (via `extends`), those are pulled in automatically.

**Insert** — press `Enter`. All staged components are concatenated in order, frontmatter is stripped, and the result is pasted directly into the previously focused app. Press `Cmd+C` to copy the highlighted prompt body without pasting.

### The extends system

Prompts can declare dependencies on other prompts. A file like `code-review.md` can extend `base-system-prompt.md` and `mentor-persona.md`. When you select it, the full chain is assembled automatically:

```
1. base-system-prompt.md    (auto)
2. mentor-persona.md        (auto)
3. code-review.md           (selected)
```

Dependencies resolve transitively and are deduplicated. You always see the full chain before copying.

### Prompt frontmatter

prompt-picker uses standard YAML frontmatter that's fully compatible with Obsidian:

```yaml
---
type: prompt
name: "Code review expert"
section: review
section_name: Review
section_icon: shield
section_order: 60
order: 10
extends:
  - base/system-prompt.md
  - personas/mentor.md
tags:
  - coding
  - review
pinned: true
---
You are an expert code reviewer. Focus on correctness,
performance, readability, and security...
```

| Field           | Description                                                                                                       |
| --------------- | ----------------------------------------------------------------------------------------------------------------- |
| `type: prompt`  | Marks the file as a first-class prompt (prioritized in search). Files without this still work, just ranked lower. |
| `name`          | Display name in the picker. Falls back to the filename if not set.                                                |
| `extends`       | Ordered list of file paths (relative to repo root) that get auto-included when this prompt is selected.           |
| `tags`          | Filterable tags. Search with `#tagname` in the picker. Obsidian-compatible format.                                |
| `pinned`        | Shows the prompt in the Pinned section at the top of the picker.                                                  |
| `section`       | Stable machine key for lifecycle grouping, such as `start`, `plan`, or `review`.                                  |
| `section_name`  | Human label shown for grouped section headers. Falls back to a title-cased `section`.                             |
| `section_icon`  | Optional icon key for the section header, such as `play-circle`, `search`, `list-check`, or `shield`.             |
| `section_order` | Numeric ordering between sections. Missing values sort after numbered sections, before `Other`.                   |
| `order`         | Numeric ordering within a section. Missing values preserve alphabetical ordering by prompt name.                   |

Files without any frontmatter are still indexed and selectable — they just appear with lower priority in search results.

### Recommended lifecycle sections

The picker does not require a fixed taxonomy, but this set keeps agent prompts easy to scan:

| Section       | Key           | Order | Useful for                                      |
| ------------- | ------------- | ----- | ----------------------------------------------- |
| Start         | `start`       | `10`  | Intake, alignment, source selection             |
| Investigate   | `investigate` | `20`  | Audits, source sweeps, deep research handoffs   |
| Plan          | `plan`        | `30`  | Decisions, gates, executable plans              |
| Execute       | `execute`     | `40`  | Confirmed agent work                            |
| Finish        | `finish`      | `50`  | Verification, handoff, Linear cleanup           |
| Review        | `review`      | `60`  | Critique, QA, assumption checks                 |
| Meta          | `meta`        | `70`  | Session distillation, prompt/SOP improvement    |

Use gaps of `10` for `section_order` and `order` so prompts can be inserted later without renumbering everything.

### Creating prompts in Obsidian

Obsidian can stay the source of truth for editing. A quick workflow:

1. Right-click the Prompt Picker menubar icon and choose **Copy Prompt Frontmatter Template**.
2. Choose **Open Prompt Folder** to jump to the configured prompt library.
3. Create the markdown file in the right lifecycle folder.
4. Paste the template, update `name`, `section`, `section_name`, `section_icon`, `section_order`, and `order`.
5. Write the prompt body below the closing `---`.
6. Choose **Reload** from the tray menu if the file watcher has not picked it up yet.

Keep `name` clean and human-readable. Put ordering in `section_order` and `order`, not in visible prompt names.

### Section ordering

The home view keeps the existing top-level buckets:

1. **Pinned**
2. **Frequent**
3. **All prompts**

Inside each bucket, prompt-type items are grouped by `section`. Sections sort by `section_order`, then `section_name`. Prompts inside a section sort by `order`, then `name`. Plain markdown files and prompts without a section fall into **Other** at the end.

Search results stay relevance-first: prefix match, contains match, fuzzy match, with pinned and `type: prompt` boosts preserved. Section labels may appear as compact badges, but grouping does not override search relevance.

### Preview pane

The picker shows a read-only preview pane on the right side of the window. It updates as you move through results with the arrow keys and displays the highlighted prompt body with frontmatter stripped.

Use `Cmd+Right` to focus the preview pane, `↑`/`↓` to scroll it, and `Cmd+Left` to return to the result list. Editing prompts in the preview pane is intentionally deferred; edit source markdown files in your editor or Obsidian.

## Installation

### From releases

Download the latest `.dmg` from [Releases](../../releases), open it, and drag prompt-picker to your Applications folder.

### From source

Requirements:

- [Rust](https://rustup.rs/) (latest stable)
- [Xcode Command Line Tools](https://developer.apple.com/xcode/) (`xcode-select --install`)
- [bun](https://bun.sh/)

```bash
git clone https://github.com/jonasgantner/prompt-picker.git
cd prompt-picker
bun install
bun tauri build
```

The built `.app` and `.dmg` will be in `src-tauri/target/release/bundle/`.

For development with hot-reload:

```bash
bun tauri dev
```

## Configuration

On first launch, prompt-picker creates a config file at:

```
~/.config/prompt-picker/config.toml
```

Edit it to point to your prompt folders:

```toml
# Keyboard shortcut to open the picker
shortcut = "Cmd+Shift+P"

# Separator between prompt components when copying
separator = "\n\n---\n\n"

# Add your prompt folders here
[[repos]]
name = "My Prompts"
path = "~/Documents/Obsidian/prompts"

[[repos]]
name = "Work"
path = "~/work/prompt-library"
```

The app watches this file and reloads automatically when you save changes.

## Keyboard shortcuts

The picker is designed to be used entirely from the keyboard. Press `?` with an empty search bar to see these inside the app.

**General**

| Key              | Action                                                   |
| ---------------- | -------------------------------------------------------- |
| `Cmd+Shift+P`    | Open / close picker (configurable)                       |
| `?`              | Toggle keyboard shortcuts reference                      |
| `Esc`            | Clear search (first press) / close window (second press) |
| `Enter`          | Insert into previous app and close                       |
| `Cmd+C`          | Copy highlighted prompt body and close                   |
| `Cmd+Right`      | Focus the preview pane                                   |
| `Cmd+Left`       | Return focus from preview to results                     |

**Results**

| Key              | Action                                                   |
| ---------------- | -------------------------------------------------------- |
| `↑` `↓`          | Navigate results                                         |
| `Tab`            | Add highlighted item to staging                          |
| `#keyword`       | Filter by tag (type in search bar)                       |
| `Cmd+↓`          | Jump to staging area                                     |

**Staging**

| Key                  | Action                                               |
| -------------------- | ---------------------------------------------------- |
| `↑` `↓`              | Navigate staged items                                |
| `Shift+↑` `Shift+↓`  | Reorder item                                         |
| `Shift+Tab`           | Remove item                                          |
| `Cmd+↑`              | Jump to results                                      |

**Preview**

| Key              | Action                                                   |
| ---------------- | -------------------------------------------------------- |
| `↑` `↓`          | Scroll preview while preview has focus                   |
| `PageUp` `PageDown` | Scroll preview by a larger step                       |
| `Cmd+Left`       | Return to results                                        |

## Permissions

prompt-picker requires the following macOS permissions:

- **Accessibility** — needed for the auto-paste feature (`Enter` to paste). The app simulates a `Cmd+V` keystroke in the previously focused app, which requires accessibility access. Grant it in **System Settings > Privacy & Security > Accessibility**. Without this permission, `Enter` will copy to clipboard but the paste won't go through.

If `Enter` copies the prompt but does not insert it, remove **Prompt Picker** from the Accessibility list, add `/Applications/Prompt Picker.app` again, then quit and relaunch the app. Local builds are ad-hoc signed with the stable `com.prompt-picker.app` identifier so this permission survives normal rebuilds after the first grant.

## Launch at login

Right-click the menubar icon and toggle **Launch at Login**. On macOS this writes or removes:

```
~/Library/LaunchAgents/com.prompt-picker.app.plist
```

The LaunchAgent starts the currently installed app binary at the next login. If you move or reinstall the app, toggle **Launch at Login** off and on once so the plist points at the current executable.

## How prompts are organized

prompt-picker scans all `.md` files recursively in your configured folders. It groups them into three tiers:

1. **Pinned** — prompts with `pinned: true` in frontmatter. Always at the top.
2. **Frequent** — automatically tracked by usage count. The more you use a prompt, the higher it ranks.
3. **All prompts** — everything else, with typed prompts (`type: prompt`) shown before plain markdown files.

When you search, all tiers collapse into a single ranked list.

## Tech stack

- [Tauri v2](https://v2.tauri.app/) — Rust backend, system WebKit frontend
- [React](https://react.dev/) + [TypeScript](https://www.typescriptlang.org/)
- [TailwindCSS](https://tailwindcss.com/)
- [Vite](https://vitejs.dev/)

The app is ~5 MB, uses no Chromium, and has near-zero idle resource usage.

## Tray icon

prompt-picker lives in your macOS menubar with no dock icon. Right-click the tray icon for:

- **Open config** — opens `config.toml` in your default editor
- **Open Prompt Folder** — opens the first configured prompt folder
- **Reload** — rescans all prompt folders
- **Launch at Login** — toggles the macOS LaunchAgent for automatic startup
- **Copy Prompt Frontmatter Template** — copies a current `type: prompt` YAML template with lifecycle section metadata
- **Quit**

## License

MIT
