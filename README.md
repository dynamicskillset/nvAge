# nvAge 🏳️‍🌈

A local-first, keyboard-first notes app in the tradition of [Notational Velocity](https://notational.net/) and [nvALT](https://brettterpstra.com/projects/nvalt/). Built with [Tauri v2](https://v2.tauri.app/) (Rust backend + React/TypeScript frontend). Notes stored as plain Markdown files with UUID-based identity in YAML frontmatter.

**The pitch:** launch the app, start typing, find or create a note instantly. No accounts, no cloud, no fuss. Your notes live in a normal folder on your disk — readable by any text editor, grep-able, portable.

## Features

- **Instant search** — full-text search across all notes on every keystroke, backed by SQLite
- **Keyboard-first** — arrow keys navigate, Enter opens or creates, Escape returns to search
- **Plain Markdown** — each note is a `.md` file with YAML frontmatter for stable UUID identity
- **Autosave** — 300ms debounced save on every edit, no manual save flow
- **Filesystem watching** — notes edited outside the app (in Vim, VS Code, etc.) are picked up and reindexed automatically
- **Nord theme** — dark and light modes using the [Nord colour palette](https://nordtheme.com)
- **Cinematic transitions** — View Transitions for note open/close, FLIP animations for list reordering, staggered entry effects

## What's Not Here (Yet)

- **Encrypted sync** — planned using [`age`](https://age-encryption.org/) encryption + Git as the sync provider
- **Conflict resolution** — detection and conflict file creation for multi-device use
- **Mobile support** — desktop only for now (macOS + Linux)

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/) (via [rustup](https://rustup.rs/))
- [Tauri dependencies](https://v2.tauri.app/start/prerequisites/) for your platform

### Development

```bash
npm install
npm run tauri dev
```

### Build

```bash
# Debug build
npm run tauri build -- --debug

# Release build
npm run tauri build
```

### Run Directly

```bash
./src-tauri/target/debug/nvage
```

## Configuration

Config lives at `~/.config/nvage/config.json`:

```json
{
  "notes_folder": "/home/doug/Documents/nvage-notes"
}
```

The search index is stored at `<notes_folder>/.nvage/search.db` — treated as a disposable cache, never synced.

## Stack

| Layer | Technology |
|-------|-----------|
| Shell | [Tauri v2](https://v2.tauri.app/) |
| Frontend | React + TypeScript + Vite |
| Editor | [CodeMirror 6](https://codemirror.net/) |
| Search | SQLite (no FTS5 — simple `LIKE` substring matching) |
| File watching | [notify](https://crates.io/crates/notify) crate |
| Styling | Nord colour palette, CSS custom properties |

## Versioning

This project uses [Pride Versioning](https://pridever.org/) (`PROUD.DEFAULT.SHAME`) and [Gitmoji](https://gitmoji.dev/) for commit messages.

## Licence

MIT
