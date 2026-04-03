# nvAge — Progress Report

**Last updated:** 2026-04-03

## What nvAge Is

A local-first, cross-platform desktop notes app in the tradition of Notational Velocity and nvALT. Built with Tauri v2 (Rust backend + React/TypeScript frontend). Notes stored as plain Markdown files with UUID-based identity in YAML frontmatter.

Full product spec: `nvage-prd.md`

## Current Status: Milestone 1 Complete — Polished

The core local notes app is built, functional, and visually polished. Nord theme with dark/light mode toggle, responsive layout, accessibility hardening, and cinematic animations are all in place. Milestones 2 (encrypted sync) and 3 (robustness) remain.

---

## Architecture

### Frontend (`src/`)

| File | Purpose |
|---|---|
| `src/App.tsx` | Entire UI: two-pane layout (sidebar + editor), debounced search, keyboard navigation, autosave, note CRUD via Tauri IPC, create-from-search confirmation, dark/light theme toggle, View Transitions, FLIP list animations, search highlighting |
| `src/App.css` | Component styles: sidebar, search input, note list, empty states, create prompt, editor header, CodeMirror overrides, responsive breakpoints, animation keyframes, theme toggle |
| `src/index.css` | Global styles: Tailwind v4 import, Nord CSS variables (dark + light themes), resets, scrollbar styling, reduced-motion media query |
| `src/main.tsx` | React entry point, imports `index.css` |

### Backend (`src-tauri/src/`)

| File | Purpose |
|---|---|
| `lib.rs` | Tauri app core: `AppState` (config + search index via `Arc<Mutex<>>`), 7 IPC commands, filesystem watcher setup, incremental index updates |
| `config.rs` | App config: loads/saves JSON at `~/.config/nvage/config.json` with `notes_folder` path |
| `note.rs` | Note model: `Note` struct, YAML frontmatter parsing/serialization, slug-based filenames, CRUD file I/O |
| `index.rs` | SQLite search index: database at `.nvage/search.db`, incremental updates, LIKE-based substring search, rebuild/insert/delete/search operations |
| `watcher.rs` | Filesystem watcher via `notify` crate: watches for `.md` changes, passes changed file paths for incremental reindex |
| `main.rs` | Binary entry point |

### Tauri IPC Commands

| Command | Direction | Purpose |
|---|---|---|
| `search_notes(query)` | Frontend → Backend | Substring search (LIKE), returns `SearchResult[]` ordered by recency |
| `get_note(id)` | Frontend → Backend | Load a single note by UUID |
| `create_note(title, content)` | Frontend → Backend | Create new note with frontmatter |
| `update_note(id, content)` | Frontend → Backend | Update note content, rename file if title changed |
| `delete_note_cmd(id)` | Frontend → Backend | Delete note file and index entry |
| `set_notes_folder(folder)` | Frontend → Backend | Change notes directory, rebuild index |
| `get_notes_folder()` | Frontend → Backend | Get current notes folder path |

---

## Key Design Decisions Made

### Note Identity
- UUID stored in YAML frontmatter (`id`, `created`) provides stable identity across renames
- Filename derived from title slug (human-readable), UUID used internally
- Encrypted remote files will use UUID as filename (planned for Milestone 2)

### Search
- SQLite with LIKE-based substring matching (replaced FTS5 — simpler, more predictable for short queries)
- Queries under 3 characters return all notes ordered by recency
- Index stored in `.nvage/search.db` — treated as disposable cache, never synced
- Filesystem watcher triggers **incremental** updates (not full rebuild) for changed files

### Editor
- CodeMirror 6 with Nord-based theme that responds to CSS variable changes
- System font stack, Nord Polar Night background (`#2e3440` dark / `#eceff4` light)
- Subtle syntax highlighting: headings bold, links Frost blue, emphasis italic
- 300ms debounced autosave on every edit

### Theme
- Nord palette throughout — dark mode (Polar Night) and light mode (Snow Storm)
- Toggle button on the editor empty state (sun/moon SVG icons)
- Theme persisted in `localStorage`
- View Transition circular reveal on theme switch (Chrome/Edge/Safari)

### UX
- Keyboard-first: ↑↓ navigate, Enter open/create, Escape back to search, `?` for shortcuts
- Create confirmation: two-step (Enter shows confirmation, Enter again creates)
- Relative time display on notes
- Empty state with keyboard shortcut hints
- Delete requires two clicks within 3 seconds
- Error banner with dismiss button for IPC failures

### Animations (Overdrive)
- **View Transitions** — note open/close morphs between sidebar and editor views
- **Shared element transitions** — note title physically moves from list item to editor header
- **FLIP list reordering** — search results slide to new positions instead of jumping
- **Staggered entry** — list items cascade in with 20ms delay per item
- **Search highlighting** — matching text highlighted with Nord accent color
- **Theme morph** — circular reveal expands from toggle button position
- All animations respect `prefers-reduced-motion`

---

## Bugs Fixed During Development

1. **Stale closure in keyboard handler** — `showCreatePrompt` was missing from `handleKeyDown` dependency array, preventing creation of notes after the first one
2. **Inaccessible editor colors** — CodeMirror theme selectors needed `!important` to override default light theme on `.cm-scroller`, `.cm-content`, `.cm-line`
3. **Tauri feature mismatch** — `shell-open` feature doesn't exist in Tauri v2, replaced by `tauri-plugin-opener`
4. **Arc state management** — `AppState` needed to be `Arc<AppState>` for sharing between Tauri state management and filesystem watcher closure
5. **Missing CSS import** — `index.css` (Nord variables) was never imported in `main.tsx`
6. **Enter key opened first result instead of creating** — fixed keyboard handler logic
7. **Full reindex on every autosave** — replaced with incremental per-file updates
8. **FTS5 dead code** — removed unused virtual table and sync triggers
9. **Regex state bug in search highlighting** — `regex.test()` is stateful, replaced with string comparison
10. **Theme toggle icons reversed** — sun/moon now correctly reflect current state

---

## Dependencies

### npm
`@codemirror/lang-markdown`, `@codemirror/theme-one-dark`, `@uiw/react-codemirror`, `@tauri-apps/api`, `@tauri-apps/plugin-opener`, `react`, `react-dom`, `@lezer/highlight`

### npm (dev)
`tailwindcss`, `@tailwindcss/vite`, `@tauri-apps/cli`, `typescript`, `vite`, `@vitejs/plugin-react`, `@types/react`, `@types/react-dom`

### Rust
`tauri`, `tauri-plugin-opener`, `serde`, `serde_json`, `serde_yaml`, `uuid`, `chrono`, `rusqlite` (bundled), `notify`, `slug`, `once_cell`, `log`, `anyhow`, `dirs`

---

## What's Not Done (Milestones 2 & 3)

### Milestone 2: Encrypted Sync
- [ ] `SyncProvider` trait abstraction
- [ ] `age` encryption module (Rust `age` crate)
- [ ] Git sync provider (shell out to `git` CLI)
- [ ] Key generation/import UI flow
- [ ] Push cycle: encrypt → stage → commit → push
- [ ] Pull cycle: fetch → pull → decrypt → reindex
- [ ] Sync status indicator in UI
- [ ] Security integration test (assert no plaintext in sync destination)

### Milestone 3: Robustness
- [ ] Conflict detection and conflict file creation
- [ ] Conflict warning in UI
- [ ] Graceful handling of failed push/pull
- [ ] Setup validation (Git installed, key accessible, remote reachable)
- [ ] Error recovery and clear error messages

---

## Version Control

- **Kin** — semantic VCS initialized at `.kin/`
- Latest commit: `991a0399` — "fix search highlighting (regex state bug, CSS specificity), remove autosave pulse, fix theme toggle icons"
- Total entities tracked: 95

---

## Build & Run

```bash
# Dev mode (hot reload)
npm run tauri dev

# Build debug binary
npm run tauri build -- --debug

# Run debug binary directly
./src-tauri/target/debug/nvage

# Build release
npm run tauri build
```

## Config Location
- Config: `~/.config/nvage/config.json`
- Search index: `<notes_folder>/.nvage/search.db`
- Private key (future): `~/.config/nvage/key.txt`
