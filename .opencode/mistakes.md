# nvAge — Mistakes Log

Things tried that did not work, recorded so we do not repeat them.

---

## 2026-04-05

### Flatpak build on local laptop took hours and never finished
- **Attempted:** Running `flatpak-builder` locally on Fedora Silverblue laptop
- **Why it failed:** The build downloads the entire GNOME SDK runtime and compiles every Rust dependency from scratch inside the sandbox. On a laptop this took hours without completing.
- **What was done instead:** Set up GitHub Actions with `flatpak/flatpak-github-actions:gnome-48` container to build on GitHub's runners (~10-15 min).

### Flatpak CI failed — manifest filename case mismatch
- **Attempted:** First Flatpak workflow run on GitHub Actions
- **Why it failed:** The manifest file was named `com.doug.nvAge.yml` (capital A) but the workflow referenced `com.doug.nvage.yml` (lowercase). macOS's case-insensitive filesystem meant `git` did not detect the rename when the contents were updated.
- **What was done instead:** Used `git mv` to properly rename the file to `com.doug.nvage.yml`.

### Flatpak CI failed — `npm install` "Exit handler never called!"
- **Attempted:** Second Flatpak workflow run after filename fix
- **Why it failed:** `npm install` crashes inside the flatpak-builder sandbox with "Exit handler never called!" — a known npm issue in sandboxed/containerised environments.
- **What was done instead:** Switched to `npm ci --prefer-offline --no-audit --no-fund` and added `NPM_CONFIG_OFFLINE: 'true'` environment variable.

### Flatpak CI used EOL GNOME 47 runtime
- **Attempted:** Using GNOME 47 runtime in Flatpak manifest
- **Why it failed:** GNOME 47 runtime is end-of-life as of October 2025.
- **What was done instead:** Updated manifest to GNOME 48 and the workflow container to `ghcr.io/flathub-infra/flatpak-github-actions:gnome-48`.

### Window close handler overwrote sync config on disk
- **Attempted:** Window close handler loaded `AppConfig` fresh from disk, updated window state, and saved
- **Why it failed:** This created a separate `AppConfig` instance that did not have the latest sync data from the running session. When it saved, it overwrote the disk config with a stale version that had `sync: null`, causing sync to be lost after restart.
- **What was done instead:** Changed the close handler to use `persist_state.config.lock()` to get the in-memory config (which has the latest sync data), update window state on it, and save.
