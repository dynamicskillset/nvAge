# nvAge Project Context

## Design Context
See `.impeccable.md` for full design context, Nord palette, and conventions.

## Project Summary
nvAge is a local-first, cross-platform notes app in the tradition of Notational Velocity and nvALT. Tauri desktop app (Rust backend, React/TypeScript frontend). Notes stored as plain Markdown files in a user-chosen folder with optional `age`-encrypted Git sync.

## Stack
- **Shell:** Tauri v2
- **Frontend:** React + TypeScript + Vite
- **Editor:** CodeMirror 6
- **Search:** SQLite with FTS5
- **File watching:** notify crate (Rust)
- **Encryption:** age crate (Rust)
- **Styling:** Nord colour palette, CSS custom properties
- **Tailwind CSS v4**

## Development
```bash
npm install
npm run tauri dev
```

## Conventions
- British English always
- No em dashes — use commas, colons, semicolons, or full stops
- Prohibited words: landscape, navigate/navigating, ensure, crucial/crucially, essential, vital, robust, foster, delve, dynamic, enhance, realm, streamline, leverage, comprehensive, facilitate, innovative, cutting-edge, transform/transformative, empower, optimise/optimize, seamless, harness, paradigm, holistic, synergy, ecosystem, stakeholder, pivotal, nuanced, intersection, at the heart of, in today's [x], it's important to note, it's worth noting, in an era, game-changer, take it to the next level, dive in/into, buckle up, let's unpack

## Versioning
- **PriDever** (`PROUD.DEFAULT.SHAME`) — https://pridever.org
  - `PROUD`: releases you're genuinely excited about (resets others to 0)
  - `DEFAULT`: ordinary, acceptable releases
  - `SHAME`: embarrassing bug fixes
- Pre-release labels and build metadata follow SemVer conventions

## Commits
- **Gitmoji** prefixes — https://gitmoji.dev/
  - ✨ `:sparkles:` new feature | 🐛 `:bug:` bug fix | ♻️ `:recycle:` refactor
  - 📝 `:memo:` docs | ✅ `:white_check_mark:` tests | 💥 `:boom:` breaking change
  - 🎨 `:art:` code structure | ⚡️ `:zap:` performance | 🔧 `:wrench:` config
  - 🚀 `:rocket:` deploy | 🗑️ `:wastebasket:` remove code | 🔒 `:lock:` security

## Git / .gitignore
- Ignore all `.md` files **except** `README.md`, `CHANGELOG.md`, and `LICENSE.md`
- Standard `.gitignore` pattern:
  ```
  *.md
  !README.md
  !CHANGELOG.md
  !LICENSE.md
  ```

## Semantic VCS
- Use **Kin** alongside Git — https://github.com/firelock-ai/kin
  - Binary at `~/.local/bin/kin`; `.kin/` is gitignored so GitHub pushes are unaffected
  - Run `kin commit -m "..."` after code changes to update the semantic graph
  - Use `kin status`, `kin trace`, `kin impact`, `kin diff` for exploration
  - Git/GitHub workflow is unchanged — Kin is additive only
