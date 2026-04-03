# nvAge

A fast, private notes app for your desktop. Type to search, press Enter to create. Your notes live as plain text files on your own machine — no accounts, no cloud, no one else reading them.

## What It Does

Open the app and start typing. Your notes appear instantly as you type. Press Enter to open one, or to create a new note if nothing matches. That's the whole thing.

Your notes are just Markdown files in a folder you choose. You can open them in any text editor, search them with your terminal, back them up however you like. The app doesn't own your notes — you do.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| ↑ / ↓ | Move through search results |
| Enter | Open selected note, or create a new one |
| Escape | Go back to search |
| ? | Show keyboard shortcuts |

## What's Coming

- **Mobile support** — not yet, but it's on the list

## For Developers

<details>
<summary>Technical details, build instructions, and configuration</summary>

### Stack

Built with [Tauri v2](https://v2.tauri.app/) — Rust backend, React/TypeScript frontend. Notes stored as plain Markdown with YAML frontmatter for UUID identity.

| Layer | Technology |
|-------|-----------|
| Shell | [Tauri v2](https://v2.tauri.app/) |
| Frontend | React + TypeScript + Vite |
| Editor | [CodeMirror 6](https://codemirror.net/) |
| Search | SQLite (`LIKE` substring matching) |
| File watching | [notify](https://crates.io/crates/notify) crate |
| Styling | Nord colour palette, CSS custom properties |

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

### Configuration

Config lives at `~/.config/nvage/config.json`:

```json
{
  "notes_folder": "/home/doug/Documents/nvage-notes"
}
```

The search index is stored at `<notes_folder>/.nvage/search.db` — treated as a disposable cache, never synced.

### Versioning

This project uses [Pride Versioning](https://pridever.org/) (`PROUD.DEFAULT.SHAME`) and [Gitmoji](https://gitmoji.dev/) for commit messages.

</details>

## Licence

AGPL-3.0 — see [LICENSE](LICENSE) for details.
