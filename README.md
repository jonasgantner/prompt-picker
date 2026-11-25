# Prompt Picker

A macOS menubar utility for composing LLM prompts from reusable components. Press a keyboard shortcut, search your prompt library, select and combine pieces, and get the result on your clipboard — ready to paste.

Built for people who maintain prompt collections as markdown files (works great with Obsidian vaults).

## How it works

You keep your prompts as `.md` files in one or more folders on disk. prompt-picker indexes them, lets you search and select components, resolves dependencies between them, and copies the combined result to your clipboard.

**Open the picker** — press `Cmd+Shift+P` (configurable) or click the menubar icon.

**Search** — start typing to fuzzy-search by name. Type `#coding` to filter by tag.

**Select** — press `Tab` to add the highlighted prompt to the staging area. If it depends on other prompts (via `extends`), those are pulled in automatically.

**Insert** — press `Enter`. All staged components are concatenated in order, frontmatter is stripped, and the result is pasted directly into the previously focused app. Press `Cmd+C` to copy the highlighted prompt body without pasting.

### Try a two-part prompt

Save these two files at the root of a folder configured under [Configuration](#configuration). If the new files do not appear in search, right-click the menu bar icon and choose **More → Reload**.

`review-format.md`

```markdown
---
type: prompt
name: Review format
---
List the most important finding first and include one concrete next step.
```

`pr-review.md`

```markdown
---
type: prompt
name: PR review
extends: [review-format.md]
---
Review the diff below for correctness and missing edge cases.
```

Open a text editor, then open Prompt Picker with **⌘⇧P**. Search for **PR review**, press **Tab** to stage it, then press **Enter**. Prompt Picker assembles both bodies in order and returns the result to your editor. If macOS Accessibility permission is off, paste the result from the clipboard with **⌘V**.

With the default separator, the result is:

```text
List the most important finding first and include one concrete next step.

---

Review the diff below for correctness and missing edge cases.
```

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
order: 10
use_when: "Use this when a plan needs a hard review before execution."
suggested_follow_up: "Run Executable steps after fixing the plan."
alternatives:
  - "Adversarial critique"
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
| `name`          | Optional fallback display name when the filename is still `Untitled` or otherwise not useful. Usually omit it.    |
| `type: prompt`  | Optional legacy/root-file marker. Files inside configured section folders are treated as prompts automatically.    |
| `extends`       | Ordered list of file paths (relative to repo root) that get auto-included when this prompt is selected.           |
| `tags`          | Filterable tags. Search with `#tagname` in the picker. Obsidian-compatible format.                                |
| `pinned`        | Sorts the prompt first inside its category and marks it with a compact pin badge.                                 |
| `order`         | Numeric ordering within the prompt's folder-derived section. Missing values preserve alphabetical ordering by prompt name. |
| `use_when`      | Short use case shown in the preview metadata area. It is not inserted into the final prompt body.                  |
| `suggested_follow_up` | Optional next step shown in the preview metadata area.                                                     |
| `alternatives`  | Optional list of related prompt names shown in the preview metadata area.                                         |

Files without any frontmatter are still indexed and selectable. Files inside configured section folders are treated as first-class prompts automatically.

### Section file

Sections are collection-level metadata. Put them in `prompt-picker.toml` at the root of a prompt repository:

```toml
ignore = ["README.md"]

[[sections]]
folder = "01-start"
key = "start"
name = "Start"
icon = "play-circle"
order = 10

[[sections]]
folder = "02-work"
key = "work"
name = "Work"
icon = "list-check"
order = 20

[[sections]]
folder = "03-close"
key = "close"
name = "Close"
icon = "check-circle"
order = 30
```

Prompt files join a section by living in that folder. For example, `01-start/shape.md` belongs to `start`; the prompt file does not need `section`, `section_name`, `section_icon`, or `section_order` frontmatter.

A new `.md` file created inside a configured section folder appears in the picker as soon as the file watcher sees it. Its filename is the prompt name; `name:` frontmatter is only a fallback for placeholder filenames such as `Untitled.md`.

If a folder is not listed in `prompt-picker.toml`, prompt-picker derives a section from the folder name by stripping a leading numeric prefix. For example, `02-review` becomes key `review`, label `Review`, and order `2`.

Older prompt libraries that still use `section`, `section_name`, `section_icon`, or `section_order` in prompt frontmatter continue to work as a fallback.

### Recommended lifecycle sections

The picker does not require a fixed taxonomy, but this set keeps agent prompts easy to scan:

| Section  | Folder          | Key        | Useful for                                      |
| -------- | --------------- | ---------- | ----------------------------------------------- |
| Start    | `01-start`      | `start`    | Intake, alignment, source selection             |
| Research | `02-research`   | `research` | Source sweeps, deep research, system audits     |
| Plan     | `03-plan`       | `plan`     | Decisions, gates, executable plans              |
| Work     | `04-work`       | `work`     | Confirmed agent work                            |
| Finish   | `05-finish`     | `finish`   | Verification, handoff, Linear cleanup, distill  |
| Review   | `06-review`     | `review`   | Critique, QA, assumption checks                 |

Use gaps of `10` for manifest `order` and prompt `order` so sections and prompts can be inserted later without renumbering everything.

### Creating prompts in Obsidian

Obsidian can stay the source of truth for editing. A quick workflow:

1. Right-click the Prompt Picker menu bar icon and choose **More → Copy Prompt Template**.
2. Choose **Open Prompts Folder** to jump to the configured prompt library.
3. Create the markdown file in the right lifecycle folder.
4. Paste the template, update `order`, `use_when`, `suggested_follow_up`, and `alternatives`.
5. Write the prompt body below the closing `---`.
6. Choose **More → Reload** if the file watcher has not picked it up yet.

Keep filenames clean and human-readable, because the filename is the prompt name in the picker. Put section labels/icons/order in `prompt-picker.toml`, not in each prompt file. Put picker-only guidance in frontmatter, not in the body; inserted prompts should contain only the instructions the target agent should receive.

### Category ordering

The home view is category-first. It renders lifecycle categories directly, such as **Start**, **Work**, and **Close**, rather than wrapping everything in separate Pinned/Frequent/All buckets.

Prompts are grouped by their first folder under the configured prompt repository. Categories sort by manifest `order`, then derived folder order, then label. Prompts inside a category sort by:

1. `pinned: true`
2. prompt classification, which is automatic for files inside configured section folders
3. `order`
4. usage count
5. `name`

Plain markdown files at the repository root fall into **Other** at the end unless ignored by `prompt-picker.toml`. Search results stay relevance-first: prefix match, contains match, fuzzy match, with pinned and automatic prompt classification boosts preserved. Category labels appear as compact badges, but grouping does not override search relevance.

### Preview pane

The picker shows a read-only preview pane on the right side of the window. It updates as you move through results with the arrow keys, shows compact frontmatter metadata at the top, and displays the highlighted prompt body with frontmatter stripped.

Use `Right` to focus the preview pane, `↑`/`↓` to scroll it, and `Left` to return to the result list. Editing prompts in the preview pane is intentionally deferred; edit source markdown files in your editor or Obsidian.

## Installation

### From releases

Download the `aarch64.dmg` for Apple silicon or the `x64.dmg` for Intel from [Releases](../../releases). Open the disk image and drag **Prompt Picker** to **Applications**.

The macOS download is ad hoc signed and is distributed without Apple notarization. Try opening the app once. If macOS blocks it because the developer cannot be verified, go to **System Settings → Privacy & Security → Open Anyway**, then confirm **Open**. [Apple’s first-launch guide](https://support.apple.com/en-us/102445)

### From source

Requirements:

- [Rust](https://rustup.rs/) (latest stable)
- [Xcode Command Line Tools](https://developer.apple.com/xcode/) (`xcode-select --install`)
- [bun](https://bun.sh/)

```bash
git clone https://github.com/jonasgantner/prompt-picker.git
cd prompt-picker
bun install
bun tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

The built `.app` and `.dmg` will be in `src-tauri/target/release/bundle/`.

For a local macOS build that should keep Accessibility permission stable across reinstalls, create a trusted local code-signing identity named `Prompt Picker Local Code Signing`, then run:

```bash
scripts/build-local-signed.sh
```

The default config uses ad-hoc signing so CI and release builds do not depend on a private local Keychain identity. Override the local identity name with:

```bash
PROMPT_PICKER_SIGNING_IDENTITY="Your Identity Name" scripts/build-local-signed.sh
```

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
| `Right`          | Focus the preview pane                                   |
| `Left`           | Return focus from preview to results                     |

**Results**

| Key              | Action                                                   |
| ---------------- | -------------------------------------------------------- |
| `↑` `↓`          | Navigate results                                         |
| `Option+↑` `Option+↓` | Jump between visible categories                     |
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
| `Left`           | Return to results                                        |

## Permissions

prompt-picker requires the following macOS permissions:

- **Accessibility** — needed for the auto-paste feature (`Enter` to paste). The app simulates a `Cmd+V` keystroke in the previously focused app, which requires accessibility access. Grant it in **System Settings > Privacy & Security > Accessibility**. Without this permission, `Enter` will copy to clipboard but the paste won't go through.

If **Enter** copies the prompt without inserting it, choose **More → Copy Paste Report** from the menu bar menu and check `accessibility_trusted`. After an update, you may need to remove the old Prompt Picker entry and add the current app again in **System Settings → Privacy & Security → Accessibility**.

## Launch at login

Right-click the menu bar icon and toggle **Open at Login**. On macOS this manages the LaunchAgent at:

```
~/Library/LaunchAgents/com.jonasgantner.promptpicker.plist
```

The LaunchAgent starts the app at the next login. If you move the app, toggle **Open at Login** off and on so it points to the new location.

## How prompts are organized

prompt-picker scans `.md` files recursively in your configured folders. The home view groups prompts by the first folder under each prompt repository, with optional section metadata from `prompt-picker.toml`. Pinned prompts sort first inside their category, frequently used prompts get a small ranking boost, and uncategorized root files appear in **Other**.

When you search, categories collapse into a single ranked list with compact category badges.

## Tech stack

- [Tauri v2](https://v2.tauri.app/) — Rust backend, system WebKit frontend
- [React](https://react.dev/) + [TypeScript](https://www.typescriptlang.org/)
- [TailwindCSS](https://tailwindcss.com/)
- [Vite](https://vitejs.dev/)

The app is ~5 MB, uses no Chromium, and has near-zero idle resource usage.

## Tray icon

Left-click the monochrome menu bar icon to open Prompt Picker. Right-click for **Settings**, **Open Prompts Folder**, **Open at Login**, and **Quit Prompt Picker**. The **More** submenu contains **Reload**, **Copy Prompt Template**, **Copy Paste Report**, and the version.

## Authors

[Jonas Gantner](https://github.com/jonasgantner) and [Maxim Kopecki](https://github.com/mkopecki).

## License

[MIT](LICENSE)
