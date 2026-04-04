# Changelog

All notable changes to nvAge are documented in this file.

Versioning follows [Pride Versioning](https://pridever.org/) (`PROUD.DEFAULT.SHAME`).

## [1.2.0] — 2026-04-04

### Proud

- RPM packages for Fedora, RHEL, openSUSE, and other RPM-based distros
- Flatpak builds for Fedora Silverblue and other Flatpak-friendly distros
- GitHub Pages site at https://dynamicskillset.github.io/nvAge/ with dynamic version display

### Default

- Duplicate GitHub link removed from site header
- Flatpak build skips run tests to avoid xvfb failures in CI

## [1.1.0] — 2026-04-04

### Proud

- AppImage builds for Linux — works on Fedora Silverblue, Ubuntu, Debian, Arch, and more
- Auto-sync every 5 minutes when sync is configured
- Update check on startup — queries GitHub releases and shows a badge when a newer version is available
- Built app binaries published as GitHub releases on tag push

### Default

- Note text now wraps within the window bounds instead of flowing past the edge
- Editor theme correctly switches between dark and light modes
- CI builds on Ubuntu, Windows, and macOS with Node.js 22

### Shame

- Cross-compilation targets removed from CI — was failing due to missing toolchains
- Redundant sync indicator removed from editor pane
- Screenshot updated in README

## [1.0.0] — 2026-04-03

First release. All three milestones complete.

### Proud

- Instant full-text search across all notes on every keystroke
- Keyboard-first workflow: arrow keys navigate, Enter opens or creates, Escape returns to search
- Encrypted sync via Git and `age` — notes encrypted before they leave your device
- Plain Markdown files with YAML frontmatter — your notes are always readable, even without the app
- Nord colour palette with dark and light modes, respecting OS preference
- Cinematic transitions: View Transitions for note open/close, FLIP list reordering, staggered entry animations
- Undo after note deletion (5-second window)
- Window size, position, and maximised state remembered between sessions

### Default

- Filesystem watching — notes edited outside the app are picked up automatically
- Create confirmation step to prevent accidental note creation
- Two-click delete confirmation with 3-second timeout
- Sync setup validation (checks Git installed, key exists, remote reachable)
- Conflict detection — conflicting versions saved as `.conflict-<date>.md` files
- Search highlighting with Nord accent colour
- Notes folder can be changed from the UI
- Error messages in plain English, not raw system errors

### Shame

- Git binary resolution fixed for Tauri apps (was failing with ENOENT)
- Create flow fixed — Enter now creates notes instead of opening the first result
- Full reindex on every autosave replaced with incremental per-file updates
- Theme toggle icons were reversed
- Missing CSS import meant Nord colours weren't loading
- Regex state bug in search highlighting
