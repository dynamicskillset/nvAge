# nvAge — Progress Report

**Last updated:** 2026-04-05

## What nvAge Is

A local-first, cross-platform desktop notes app in the tradition of Notational Velocity and nvALT. Built with Tauri v2 (Rust backend + React/TypeScript frontend). Notes stored as plain Markdown files with UUID-based identity in YAML frontmatter.

Full product spec: `nvage-prd.md`

## Current Status: v0.3.0 — Sync Persistence Fix, Flatpak CI, Release Artifacts

Milestones 1, 2, and 3 are complete. v0.3.0 is a quality pass following a technical audit: fixes sync pull counting (now uses `git diff` to detect actual changes), adds missing CSS for sync config display, replaces hard-coded hex with CSS variables, improves touch targets, and cleans up compiler warnings.

### Recent work (2026-04-05)

- **Sync error fix**: Fixed double "failed" in sync error message ("Sync failed failed" → "Sync failed"). Added `reset_sync` IPC command and "Remove sync" button in the sync setup UI so users can start fresh.
- **Flatpak CI**: Multiple attempts to get Flatpak building on GitHub Actions. The `flatpak/flatpak-github-actions/flatpak-builder@v6` action runs in an isolated Docker container that can't see pre-built binaries from the host runner. Current approach: build Tauri natively on the runner, then run `flatpak-builder` directly (not via the action) to package the pre-built binary. Still failing — the Docker container used by the action can't access the host filesystem. **This is the top priority for the next session.**
- **Slash commands**: `/handoff` and `/catchup` created and confirmed working after OpenCode restart.
- **`mistakes.md`**: New file to track things that went wrong and should be avoided.
- **Kin remote**: Configured `origin` as default remote (`github / git-export` → `https://github.com/dynamicskillset/nvAge`). Fixed `.kin/config.toml` world preset case sensitivity (`"Native"` → `"native"`).

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
| `lib.rs` | Tauri app core: `AppState` (config + search index + sync provider + key path via `Arc<Mutex<>>`), 15 IPC commands (7 notes + 6 sync + 1 config + 1 version), filesystem watcher setup, incremental index updates, sync config persistence |
| `config.rs` | App config: loads/saves JSON at `~/.config/nvage/config.json` with `notes_folder` path |
| `note.rs` | Note model: `Note` struct, YAML frontmatter parsing/serialization, slug-based filenames, CRUD file I/O, `deserialize_content` for sync |
| `index.rs` | SQLite search index: database at `.nvage/search.db`, incremental updates, LIKE-based substring search, rebuild/insert/delete/search operations |
| `watcher.rs` | Filesystem watcher via `notify` crate: watches for `.md` changes, passes changed file paths for incremental reindex |
| `crypto.rs` | `age` encryption module: key generation/import, encrypt/decrypt, file-level encryption, ASCII-armoured output |
| `sync_provider.rs` | `SyncProvider` trait: `push`, `pull`, `sync`, `status`, `is_configured` methods, `SyncStatus` and `SyncResult` types |
| `sync_git.rs` | Git sync provider: shells out to `git` CLI, clone/fetch, encrypt notes to `<uuid>.md.age`, stage/commit/push, pull/decrypt, conflict detection |
| `main.rs` | Binary entry point |

### Tauri IPC Commands

#### Note commands
| Command | Direction | Purpose |
|---|---|---|
| `search_notes(query)` | Frontend → Backend | Substring search (LIKE), returns `SearchResult[]` ordered by recency |
| `get_note(id)` | Frontend → Backend | Load a single note by UUID |
| `create_note(title, content)` | Frontend → Backend | Create new note with frontmatter |
| `update_note(id, content)` | Frontend → Backend | Update note content, rename file if title changed |
| `delete_note_cmd(id)` | Frontend → Backend | Hard-delete note (removes file and index entry) |
| `set_notes_folder(folder)` | Frontend → Backend | Change notes directory, rebuild index |
| `get_notes_folder()` | Frontend → Backend | Get current notes folder path |

#### Sync commands
| Command | Direction | Purpose |
|---|---|---|
| `generate_sync_key()` | Frontend → Backend | Generate new `age` keypair, save to `~/.config/nvage/key.txt` |
| `import_sync_key(key_str)` | Frontend → Backend | Import existing `age` key from string |
| `configure_sync(remote_url, branch)` | Frontend → Backend | Set up Git sync provider with remote repo URL |
| `sync_notes(direction)` | Frontend → Backend | Run sync: `push`, `pull`, or `full` cycle — returns "All notes are up to date" when nothing changed |
| `get_sync_status()` | Frontend → Backend | Get current sync status (idle, syncing, error, conflict, not_configured) |
| `validate_sync_setup(remote_url)` | Frontend → Backend | Validate Git installed, key exists, remote reachable |
| `get_sync_config()` | Frontend → Backend | Get persisted sync config (remote URL + branch) |

---

## Key Design Decisions Made

### Note Identity
- UUID stored in YAML frontmatter (`id`, `created`) provides stable identity across renames
- Filename derived from title slug (human-readable), UUID used internally
- Encrypted remote files use UUID as filename (`<uuid>.md.age`) — title not leaked to sync destination

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

### Encryption
- `age` crate for file-level encryption (X25519 public-key)
- ASCII-armoured output for portability
- Secret key stored at `~/.config/nvage/key.txt` with `0600` permissions (Unix)
- Private key never transmitted, committed, or included in sync

### Sync
- `SyncProvider` trait abstraction allows swapping providers (Git v1, folder-based future)
- Git provider shells out to `git` CLI — user must have Git installed
- Sync repo cloned to `~/.cache/nvage/sync-repo` (hidden working tree)
- Generic commit messages (`Update notes`) to avoid leaking note titles
- Conflict files saved as `<title>.conflict-<date>.md`

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
11. **Soft-deleted notes reappearing after sync** — soft-delete approach was fundamentally broken; replaced with hard deletes. `git add -A` stages deletions so removed `.md.age` files are committed to the remote, and pull sees they're gone.
12. **File watcher re-indexing deleted notes** — `update_files` already handles missing files by removing them from the index.
13. **Duplicate CSS rules** — bin component styles were duplicated (~150 lines); all bin code removed.
14. **Sync pull always counted all notes** — `pull` iterated every `.md.age` file regardless of whether it changed; fixed by comparing HEAD before/after with `git diff --name-only`.
15. **Missing CSS for sync-config classes** — `sync-config-info`, `sync-config-row`, `sync-config-label`, `sync-config-value` used in JSX with no styles defined.
16. **Hard-coded hex in sync-dot indicators** — 4 Aurora hex values bypassed the CSS variable token system.
17. **Undersized touch targets on desktop** — new-note-btn (32px), delete-btn, sync-close-btn all below comfortable click targets.
18. **Sync config lost on restart** — window close handler was loading a fresh `AppConfig` from disk (which could be stale) and saving only window state, overwriting the sync config. Fixed by using the in-memory `AppState.config` instead.
19. **Flatpak manifest filename case mismatch** — file was `com.doug.nvAge.yml` but workflow referenced `com.doug.nvage.yml`. macOS case-insensitive filesystem hid the discrepancy.
20. **Flatpak `npm install` crash in sandbox** — npm's "Exit handler never called!" error inside flatpak-builder. Fixed by switching to `npm ci --prefer-offline`.
21. **GNOME 47 runtime EOL** — Flatpak manifest updated to GNOME 48.
22. **Missing icon in Flatpak builds** — Flatpak bypasses Tauri's bundler, so no icon or `.desktop` file was installed. Added explicit install commands for icons and desktop entry.
23. **Missing 512px icon in Tauri bundle config** — `icon-512.png` was not listed in `tauri.conf.json`, causing AppImage and other Linux bundles to fall back to the default Tauri icon.

---

## Dependencies

### npm
`@codemirror/lang-markdown`, `@codemirror/theme-one-dark`, `@uiw/react-codemirror`, `@tauri-apps/api`, `@tauri-apps/plugin-opener`, `react`, `react-dom`, `@lezer/highlight`

### npm (dev)
`tailwindcss`, `@tailwindcss/vite`, `@tauri-apps/cli`, `typescript`, `vite`, `@vitejs/plugin-react`, `@types/react`, `@types/react-dom`

### Rust
`tauri`, `tauri-plugin-opener`, `serde`, `serde_json`, `serde_yaml`, `uuid`, `chrono`, `rusqlite` (bundled), `notify`, `slug`, `once_cell`, `log`, `anyhow`, `dirs`, `age` (with `armor` feature), `rand`

---

## What's Not Done

### Milestone 2: Encrypted Sync — Complete
- [x] `SyncProvider` trait abstraction
- [x] `age` encryption module (Rust `age` crate)
- [x] Git sync provider (shell out to `git` CLI)
- [x] Key generation/import UI flow
- [x] Push cycle: encrypt → stage → commit → push
- [x] Pull cycle: fetch → pull → decrypt → reindex
- [x] Sync status indicator in UI
- [x] Security integration test (assert no plaintext in sync destination)

### Milestone 3: Robustness — Complete
- [x] Conflict detection and conflict file creation (saves `<title>.conflict-<date>.md` files)
- [x] Conflict warning in UI (amber banner with dismiss button)
- [x] Graceful handling of failed push/pull (clear error messages in sync card)
- [x] Setup validation (Git installed, key accessible, remote reachable)
- [x] Error recovery and clear error messages (error banner with dismiss, retry via sync card)

---

## Version Control

- **Git** — `https://github.com/dynamicskillset/nvAge`
- **Kin** — semantic VCS at `.kin/`
- Latest git commit: `3a682de` — "🔧 update Flatpak manifest to GNOME 48 and use npm ci for sandboxed builds"
- Latest kin commit: pending (head: 66bc199e216a43c99b9543dcfc9d30d89a47a7ef05c13ad276fbfbacbe26df95)
- Total entities tracked: 168

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
- Private key: `~/.config/nvage/key.txt`
- Sync repo cache: `~/.cache/nvage/sync-repo`
