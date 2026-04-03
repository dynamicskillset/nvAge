# nvAge — Product Reference Document

## Product summary

nvAge is a local-first, cross-platform notes app in the tradition of Notational Velocity and nvALT. It stores notes as plain Markdown files in a user-chosen folder, provides instant search-and-create behaviour on every keystroke, and optionally encrypts notes with `age` before syncing them to a remote location. The app works as a standalone local notes tool from first launch; encrypted sync is an optional layer that can be enabled later.

The name signals the lineage: NV-style notes, secured with `age` encryption.

## Goals

- Recreate the fast, keyboard-first, modeless note-taking experience of Notational Velocity.
- Keep notes as portable, human-readable Markdown files that work independently of the app.
- Provide optional encrypted sync so that plaintext never exists outside the user's own devices.
- Avoid building a custom cloud backend, account system, or proprietary storage format.

## Non-goals

- Collaboration, web publishing, AI features, backlinks, rich attachments, or multi-user sync.
- End-to-end mobile sync in v1.
- Replacing the user's existing sync infrastructure (Git, Dropbox, Syncthing, etc.).
- Local disk encryption in v1 (plaintext on the user's own device is acceptable; see Threat model).

## Target users

Single-user, technical or semi-technical people who want a modern, cross-platform, Markdown-based equivalent of Notational Velocity with stronger privacy than a plain private repo or cloud folder can provide.

## Threat model

The encryption in nvAge protects notes at rest when they are not on the user's own devices. The specific threat is: a third party (cloud provider, attacker who compromises the remote storage, or anyone with access to the sync destination) should not be able to read note contents.

Plaintext on the user's local disk is acceptable for v1. The design should leave room for optional local encryption later, but it is not a v1 concern.

Key loss means data loss. There is no recovery path by design. This is a deliberate trade-off in favour of simplicity and genuine zero-knowledge architecture. The app should make this clear during key generation.

## Core user stories

- As a user, I can type into one search box and instantly filter notes by title and body on every keystroke.
- As a user, if no note matches my query, pressing Enter creates a new note with that query as the title.
- As a user, I can edit notes in Markdown with autosave and no manual save flow.
- As a user, my local notes folder is a normal folder of readable Markdown files that I can browse, grep, and edit with any tool.
- As a user, I can use the app indefinitely as a pure local notes tool without configuring any sync.
- As a user, I can enable encrypted sync and choose a sync method (Git repo in v1).
- As a user, my sync destination only ever receives encrypted files.
- As a user, I can set up and manage my encryption key locally without sending it anywhere.

## Architecture

### Single-folder design

nvAge uses one local folder of plain Markdown files. This folder is the user's notes. It is human-readable, works with any text editor, and does not depend on the app for access.

There is no persistent second folder of encrypted files on the local device. Encryption happens transiently during sync: the app encrypts each changed file into a temporary staging area, syncs the encrypted files to the remote destination, and cleans up. On pull, encrypted files are decrypted directly into the notes folder.

The notes folder contains:

- `*.md` — one Markdown file per note, with a title-based slug filename (e.g. `my-great-idea.md`).
- `.nvage/` — app metadata directory containing the SQLite search index and configuration. This directory is excluded from sync.

### Note identity

Each note has a stable UUID stored in lightweight YAML frontmatter:

```yaml
---
id: a3f7b2c1-9e4d-4b8a-b6f2-1c3d5e7f9a0b
created: 2026-03-29T10:15:00Z
---
```

The frontmatter provides the stable identity anchor. The filename is derived from the note title and is human-readable, but the `id` field is what the app uses internally to track notes across renames. If a user changes a note's title, the app renames the file to match the new title slug, and the `id` field maintains continuity for sync mapping.

The encrypted remote file uses the same UUID as its filename (e.g. `a3f7b2c1-9e4d-4b8a-b6f2-1c3d5e7f9a0b.md.age`) so that note titles are not leaked to the sync destination.

### Search index

The app uses SQLite with FTS5 for instant full-text search. The index is built by reading every Markdown file on app launch and is updated incrementally via filesystem watching. The index file lives in `.nvage/` and is treated as a disposable cache — it is never synced, and it rebuilds from scratch if deleted.

This approach supports sub-10ms search response times at thousands of notes, which is necessary to match Notational Velocity's defining characteristic: search that updates with every keystroke, with no perceptible lag.

Filesystem watching also means notes edited outside the app (in Vim, VS Code, or any other editor) are picked up and reindexed.

### Encryption

nvAge uses `age` for file-level encryption. The user encrypts to their own public key and decrypts with their private key.

- Key generation and import happen locally via an in-app setup flow.
- The private key is stored on the user's device in a location they choose (defaulting to `~/.config/nvage/key.txt` or platform equivalent).
- The private key is never transmitted, committed, or included in any sync operation.
- Each note is encrypted independently as a single `age`-encrypted file.

### Sync provider abstraction

Sync is handled through a `SyncProvider` interface with methods including `sync`, `pull`, `push`, and `status`. This abstraction means the app does not assume any particular sync mechanism.

**v1 provider: Git (shelling out to CLI)**

The v1 sync provider shells out to the `git` command-line tool. This is a runtime dependency: the user must have Git installed. This is acceptable for the v1 target audience (technical/semi-technical, macOS and Linux) and avoids the complexity of bundling libgit2 or handling Git authentication within the app.

The Git provider:

- Clones or initialises a repo in a temporary/hidden working tree.
- On push: encrypts changed notes into the Git working tree, stages, commits with a generic message (`Update notes`), and pushes.
- On pull: fetches and pulls, decrypts changed files into the notes folder, updates the search index.
- Reports status: idle, syncing, error, conflict.

The Git abstraction is designed so that the implementation can later be swapped to `git2-rs` (Rust libgit2 bindings) for a fully self-contained app, without changing the rest of the codebase.

**Future providers**

For folder-based sync services (Dropbox, Google Drive, Proton Drive, Syncthing), the provider's job is simpler: copy encrypted files to a designated folder on push, and check for changed files on pull. The external service handles the actual transport. These providers are expected to be straightforward to implement once the abstraction exists.

### Conflict handling

When a pull brings in a changed encrypted file whose corresponding local note has also been modified since the last sync:

- The app does not silently overwrite either version.
- The incoming version is decrypted and saved alongside the local version with a conflict suffix (e.g. `my-great-idea.conflict-2026-03-29.md`).
- The user is shown a clear warning in the UI.
- Resolution is manual in v1 (the user edits and deletes the conflict file).

### Stack

- **Desktop shell:** Tauri (TypeScript frontend, Rust backend).
- **Editor:** CodeMirror in the webview, adequate for v1 with potential to improve later.
- **Search index:** SQLite with FTS5, accessed from the Rust backend.
- **Encryption:** `age` crate (Rust) or shelling out to the `age` CLI.
- **Git operations:** shelling out to `git` CLI behind the `SyncProvider` abstraction.

## UX requirements

The app should feel close to Notational Velocity: minimal chrome, one prominent search field, very fast filtering, keyboard-first interaction, and near-instant note switching.

**Primary flow:**

1. App opens with cursor in the search field and the full note list visible.
2. Typing filters the list instantly on every keystroke, matching against titles and body text.
3. Arrow keys move through the filtered results.
4. Enter opens the selected note for editing, or creates a new note if no match exists.
5. Escape returns focus to the search field and restores the search terms.
6. Autosave is invisible — there is no save button, no save shortcut, no save indicator.

**Sync status** is visible but low-friction: a small indicator showing idle, syncing, error, or conflict. Sync is triggered manually in v1 (a keyboard shortcut or button), not automatic.

The interface should not look or feel like a large PKM app. Keep it sparse and focused.

## Functional requirements

- Desktop app for macOS and Linux using Tauri.
- Store plaintext notes in a user-selected folder (with a sensible default).
- Provide instant full-text search across all notes on every keystroke.
- Support autosave and filesystem watching for external edits.
- Provide a simple note list plus editor UI, optimised for keyboard use.
- Support basic Markdown editing; preview is secondary to editing.
- Allow the app to be used indefinitely without configuring sync.
- Provide an optional sync setup flow: choose provider (Git in v1), configure remote, generate or import `age` key.
- On sync push: encrypt changed notes, commit, push. On sync pull: pull, decrypt, reindex.
- Never sync plaintext notes, search indices, or private keys.
- Provide sync status indicator in the UI.
- Detect conflicts and surface them clearly rather than silently overwriting.
- Allow user configuration for remote repo URL, branch, local notes folder, and key location.

## Security requirements

- All encryption and decryption happens locally.
- Private keys remain on the device and are never committed or transmitted.
- The sync destination only ever receives encrypted note files and non-sensitive metadata.
- Encrypted filenames use the note UUID, not the note title, to avoid leaking content.
- Commit messages are generic (e.g. `Update notes`) to avoid leaking note titles.
- The `.nvage/` directory (search index, configuration) is excluded from sync via `.gitignore` or equivalent.
- The app should clearly communicate during key setup that key loss means permanent data loss for the encrypted remote copies.

## Testing requirements

The core security invariant — that the sync destination never contains plaintext — must be verified by an automated integration test. This test should:

1. Create and edit several notes.
2. Run a sync cycle.
3. Clone or inspect the resulting repo/folder.
4. Assert that every file in the sync destination is a valid `age`-encrypted blob.
5. Assert that no plaintext note content, titles, or search index data is present.

This test should run in CI for every change to the sync or encryption modules.

## Milestones

### Milestone 1: Local NV-style app

The app works as a standalone local notes tool with no sync or encryption.

- Pick or create a notes folder.
- Create, edit, search, and delete Markdown notes.
- Instant FTS5-backed search filtering on every keystroke.
- Keyboard-first navigation: search, arrow, Enter, Escape.
- Autosave on edit.
- Filesystem watching for external edits with automatic reindex.
- Title-based slug filenames with UUID in frontmatter.

**Acceptance:** A user can launch the app, create notes, and find them instantly by typing. Notes are plain Markdown files in a folder that can be opened with any editor.

### Milestone 2: Encryption and sync

Optional encrypted sync layers on top of the working local app.

- In-app setup flow for generating or importing an `age` key.
- Git sync provider (shell out to `git` CLI).
- Encrypt on push, decrypt on pull.
- Sync status indicator in the UI.
- Manual sync trigger (keyboard shortcut or button).
- Generic commit messages; UUID-based encrypted filenames.
- `.gitignore` excludes `.nvage/`, plaintext notes, and key files.

**Acceptance:** After configuring sync, a push results in only encrypted files on the remote. A pull on a fresh device with the same key decrypts all notes correctly.

### Milestone 3: Robustness

Hardening for real-world multi-device use.

- Conflict detection when both local and remote have changed.
- Conflict file creation and user-facing warning.
- Graceful handling of failed push (e.g. remote has diverged).
- Graceful handling of pull when local has unsaved changes.
- Error recovery and clear error messages for Git failures, missing keys, and encryption errors.
- Setup validation (check Git is installed, key is accessible, remote is reachable).

**Acceptance:** The app handles all common multi-device scenarios without data loss or silent overwrites.

## Nice-to-have after v1

- Folder-based sync providers (Dropbox, Syncthing, Proton Drive).
- Swap Git CLI for `git2-rs` to remove the Git runtime dependency.
- Android client or companion app.
- Selective local encryption of the notes folder.
- Better conflict merge UX (side-by-side diff).
- Command palette.
- Pinned notes, tags, and archived notes.
- Optional background sync scheduler.
- Import from nvALT, FSNotes, or plain Markdown folders.
- Improved editor experience (richer CodeMirror configuration, themes).

## Delivery instructions for Claude Code

Build nvAge as a Tauri desktop application following the three milestones in order. Each milestone should be independently functional and testable.

**Milestone 1** is the priority. Scaffold the Tauri app with a minimal NV-style interface: a single search field at the top, a filterable results list, and an editor pane. Implement the SQLite FTS5 search index in the Rust backend, filesystem watching, autosave, and the frontmatter-based note identity scheme. Get the core search-filter-create-edit loop feeling fast and right before moving on.

**Milestone 2** adds the sync provider abstraction, the `age` encryption module, and the Git provider implementation. Build the key setup flow, then the push cycle (encrypt → stage → commit → push), then the pull cycle (fetch → pull → decrypt → reindex). Test the security invariant: write an integration test that inspects the sync destination and asserts no plaintext exists.

**Milestone 3** adds conflict detection, error handling, and setup validation.

Keep the code modular and well documented. The sync provider interface should be clean enough that adding a new provider is straightforward. Optimise for simplicity, local reliability, and future extensibility rather than feature breadth.
