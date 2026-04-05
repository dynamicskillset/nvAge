# nvAge — Mistakes Log

Record things that went wrong so they are not repeated.

## 2026-04-05

- **Flatpak build in sandbox**: Running `npm ci --prefer-offline` inside flatpak-builder fails because the sandbox has no network access. The `flatpak/flatpak-github-actions/flatpak-builder@v6` action overrides build commands and forces this. Fix: build the Tauri app natively on the runner first, then use flatpak-builder to package the pre-built binary.
- **`allow-network` is not a valid flatpak-builder property**: It is silently ignored with a warning. Do not use it.
- **`NPM_CONFIG_OFFLINE: 'true'` in Flatpak manifest**: Causes npm to fail when the cache is empty. Do not set this unless you have a pre-populated cache.
- **`friendlyError` action string double-up**: Passing "Sync failed" as the action to `friendlyError` produced "Sync failed failed. Please try again." because the catch-all appends "failed. Please try again." Always pass the bare action noun (e.g. "Sync", "Search", "Delete").
- **Stash + rebase conflicts**: When pulling with rebase while having unstaged changes, stashing then popping creates merge conflicts. Commit or discard changes before pulling.
