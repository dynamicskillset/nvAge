import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import "./App.css";

// ── FLIP Animation for List Reordering ──
function flipAnimateList(container: HTMLElement, prevOrder: string[]) {
  const items = Array.from(container.querySelectorAll<HTMLElement>(".note-item"));
  const currentOrder = items.map((el) => el.dataset.noteId || "");

  if (JSON.stringify(prevOrder) === JSON.stringify(currentOrder)) return;

  const firstPositions = new Map<string, DOMRect>();
  items.forEach((item) => {
    const id = item.dataset.noteId;
    if (id) firstPositions.set(id, item.getBoundingClientRect());
  });

  requestAnimationFrame(() => {
    items.forEach((item) => {
      const id = item.dataset.noteId;
      if (!id) return;
      const first = firstPositions.get(id);
      if (!first) return;

      const last = item.getBoundingClientRect();
      const deltaY = first.top - last.top;

      if (Math.abs(deltaY) < 1) return;

      item.style.transform = `translateY(${deltaY}px)`;
      item.style.transition = "none";

      requestAnimationFrame(() => {
        item.style.transition = "transform 0.35s cubic-bezier(0.2, 0.8, 0.2, 1)";
        item.style.transform = "translateY(0)";
      });
    });
  });
}

// ── View Transition for Note Open/Close ──
function withViewTransition(fn: () => void) {
  if (document.startViewTransition) {
    document.startViewTransition(() => {
      fn();
    });
  } else {
    fn();
  }
}

// ── Theme Morph: Circular Reveal ──
function morphTheme(targetTheme: "dark" | "light", originX: number, originY: number) {
  const maxRadius = Math.hypot(
    Math.max(originX, window.innerWidth - originX),
    Math.max(originY, window.innerHeight - originY)
  );

  if (document.startViewTransition) {
    const transition = document.startViewTransition(() => {
      document.documentElement.setAttribute("data-theme", targetTheme);
      localStorage.setItem("nvage-theme", targetTheme);
    });

    transition.ready.then(() => {
      const clipPath = [
        `circle(0px at ${originX}px ${originY}px)`,
        `circle(${maxRadius}px at ${originX}px ${originY}px)`,
      ];
      document.documentElement.animate(
        { clipPath },
        { duration: 500, easing: "cubic-bezier(0.4, 0, 0.2, 1)", pseudoElement: "::view-transition-new(root)" }
      );
    });
  } else {
    document.documentElement.setAttribute("data-theme", targetTheme);
    localStorage.setItem("nvage-theme", targetTheme);
  }
}

// ── Search Highlight: wrap matching text in <mark> ──
function highlightText(text: string, query: string): React.ReactNode {
  if (!query.trim() || !text) return text;
  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const regex = new RegExp(`(${escaped})`, "gi");
  const parts = text.split(regex);
  return parts.map((part, i) =>
    part.toLowerCase() === query.toLowerCase() ? (
      <mark key={i} className="search-highlight">{part}</mark>
    ) : (
      part
    )
  );
}

// ── Pure function for relative time (no React dependencies) ──
function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days < 7) return `${days}d ago`;
  return date.toLocaleDateString();
}

// ── Editor Theme Builder ──
const editorThemes = {
  dark: {
    bg: "#2e3440",
    text: "#eceff4",
    cursor: "#eceff4",
    selection: "#5e81ac",
    heading: "#eceff4",
    link: "#88c0d0",
    emphasis: "#d8dee9",
    monospace: "#d8dee9",
    strikethrough: "#4c566a",
    atom: "#4c566a",
  },
  light: {
    bg: "#eceff4",
    text: "#2e3440",
    cursor: "#2e3440",
    selection: "#88c0d0",
    heading: "#2e3440",
    link: "#5e81ac",
    emphasis: "#3b4252",
    monospace: "#3b4252",
    strikethrough: "#4c566a",
    atom: "#4c566a",
  },
};

function buildEditorTheme(theme: "dark" | "light") {
  const c = editorThemes[theme];

  const highlightStyle = HighlightStyle.define([
    { tag: t.heading, color: c.heading, fontWeight: "bold" },
    { tag: t.strong, color: c.heading, fontWeight: "bold" },
    { tag: t.emphasis, color: c.emphasis, fontStyle: "italic" },
    { tag: t.link, color: c.link },
    { tag: t.url, color: c.link },
    { tag: t.monospace, color: c.monospace, fontFamily: "monospace" },
    { tag: t.strikethrough, color: c.strikethrough, textDecoration: "line-through" },
    { tag: t.atom, color: c.atom },
  ]);

  return [
    EditorView.theme({
      "&": {
        backgroundColor: `${c.bg} !important`,
        color: c.text,
        fontFamily: 'system-ui, sans-serif',
        fontSize: "15px",
        lineHeight: "1.6",
      },
      ".cm-scroller": {
        padding: "0",
        backgroundColor: `${c.bg} !important`,
      },
      ".cm-content": {
        padding: "16px",
        caretColor: c.cursor,
        backgroundColor: `${c.bg} !important`,
        color: c.text,
      },
      ".cm-cursor": {
        borderLeftColor: c.cursor,
      },
      ".cm-gutters": {
        display: "none",
        backgroundColor: `${c.bg} !important`,
      },
      ".cm-activeLine": {
        backgroundColor: "transparent !important",
      },
      ".cm-focused .cm-activeLine": {
        backgroundColor: "transparent !important",
      },
      ".cm-selectionBackground": {
        background: `${c.selection} !important`,
      },
      ".cm-line": {
        color: c.text,
      },
    }),
    syntaxHighlighting(highlightStyle),
  ];
}

// ── User-friendly error messages ──
function friendlyError(raw: unknown, action: string): string {
  const msg = String(raw);

  // Strip Tauri IPC wrapper noise
  const clean = msg.replace(/^Error: failed to call `[^`]+` handler: /, "");

  // Map common patterns
  if (clean.includes("No such file or directory") || clean.includes("os error 2")) {
    if (action === "sync") return "Git is not installed or cannot be found. Install Git to use sync.";
    return "A required file is missing. Check your notes folder exists.";
  }
  if (clean.includes("No sync key configured")) return "Set up your encryption key first.";
  if (clean.includes("Sync not configured")) return "Configure sync before running it.";
  if (clean.includes("Note not found")) return "This note no longer exists.";
  if (clean.includes("Failed to load config")) return "Could not read the config file.";
  if (clean.includes("Failed to create search index")) return "Could not create the search index.";
  if (clean.includes("Failed to create filesystem watcher")) return "Could not watch your notes folder for changes.";

  // Catch-all: show generic message, hide raw error
  return `${action} failed. Please try again.`;
}

interface SearchResult {
  id: string;
  title: string;
  preview: string;
  modified: string;
}

interface Note {
  id: string;
  title: string;
  content: string;
  created: string;
  modified: string;
}

function App() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIdx, setSelectedIdx] = useState(-1);
  const [activeNote, setActiveNote] = useState<Note | null>(null);
  const [editorContent, setEditorContent] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [showCreateConfirm, setShowCreateConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isNarrow, setIsNarrow] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [notesFolder, setNotesFolder] = useState("");
  const [showFolderPicker, setShowFolderPicker] = useState(false);
  const [folderInput, setFolderInput] = useState("");
  const [deletedNote, setDeletedNote] = useState<Note | null>(null);
  const [appVersion, setAppVersion] = useState("");
  const [undoTimeout, setUndoTimeout] = useState<ReturnType<typeof setTimeout> | null>(null);
  const [lastSaved, setLastSaved] = useState<number>(0);
  const [lastSynced, setLastSynced] = useState<number>(0);
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    const saved = localStorage.getItem("nvage-theme");
    if (saved === "light" || saved === "dark") return saved;
    // Default to OS preference
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  });
  const [prevResultIds, setPrevResultIds] = useState<string[]>([]);
  const noteListRef = useRef<HTMLDivElement>(null);

  // Sync state
  const [showSyncSetup, setShowSyncSetup] = useState(false);
  const [syncStep, setSyncStep] = useState<"welcome" | "key" | "remote" | "done">("welcome");
  const [syncKey, setSyncKey] = useState("");
  const [generatedKey, setGeneratedKey] = useState("");
  const [keyCopied, setKeyCopied] = useState(false);
  const [syncRemoteUrl, setSyncRemoteUrl] = useState("");
  const [syncBranch, setSyncBranch] = useState("main");
  const [syncStatus, setSyncStatus] = useState<string>("not_configured");
  const [syncMessage, setSyncMessage] = useState("");
  const [syncLoading, setSyncLoading] = useState(false);
  const [syncError, setSyncError] = useState<string | null>(null);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const deleteTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Apply theme to document
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("nvage-theme", theme);
  }, [theme]);

  // Listen for OS theme changes (only if user hasn't manually overridden)
  useEffect(() => {
    const saved = localStorage.getItem("nvage-theme");
    if (saved) return; // user has manually set a theme, don't auto-switch

    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const handler = (e: MediaQueryListEvent) => {
      setTheme(e.matches ? "light" : "dark");
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  const toggleTheme = useCallback((e?: React.MouseEvent) => {
    const x = e?.clientX ?? window.innerWidth / 2;
    const y = e?.clientY ?? window.innerHeight / 2;
    const target = theme === "dark" ? "light" : "dark";
    morphTheme(target, x, y);
    setTheme(target);
  }, [theme]);

  // Detect narrow viewport for responsive layout
  useEffect(() => {
    const check = () => setIsNarrow(window.innerWidth < 700);
    check();
    window.addEventListener("resize", check);
    return () => window.removeEventListener("resize", check);
  }, []);

  // On narrow screens, show sidebar when not editing, editor when editing
  const showSidebarView = !isNarrow || !isEditing;
  const showEditorView = !isNarrow || isEditing;

  // Build editor theme from current CSS variables — memoized on theme
  const editorTheme = useMemo(() => buildEditorTheme(theme), [theme]);

  // Memoized display title for new notes
  const displayTitle = useMemo(
    () => activeNote?.title || (editorContent ? editorContent.split("\n")[0].replace(/^#\s*/, "").trim() : "New note"),
    [activeNote?.title, editorContent]
  );

  // Memoized relative time formatter for all results
  const formattedResults = useMemo(() => {
    return results.map((r) => ({
      ...r,
      formattedTime: formatRelativeTime(r.modified),
    }));
  }, [results]);

  // Memoized highlighted results — only recompute when results or query change
  const highlightedResults = useMemo(() => {
    return formattedResults.map((r) => ({
      ...r,
      highlightedTitle: highlightText(r.title || "Untitled", query),
      highlightedPreview: r.preview ? highlightText(r.preview, query) : "Empty note",
    }));
  }, [formattedResults, query]);

  const search = useCallback(async (q: string) => {
    try {
      const res = await invoke<SearchResult[]>("search_notes", { query: q });
      setResults(res);
      setIsLoading(false);
      setError(null);
    } catch (e) {
      setError(friendlyError(e, "Search failed"));
      setIsLoading(false);
    }
  }, []);

  // FLIP animate list reordering when results change
  useEffect(() => {
    const currentIds = results.map((r) => r.id);
    if (noteListRef.current && prevResultIds.length > 0) {
      flipAnimateList(noteListRef.current, prevResultIds);
    }
    setPrevResultIds(currentIds);
  }, [results, prevResultIds]);

  const selectNote = useCallback(async (id: string) => {
    try {
      const note = await invoke<Note | null>("get_note", { id });
      if (note) {
        withViewTransition(() => {
          setActiveNote(note);
          setEditorContent(note.content);
          setIsEditing(true);
          setSelectedIdx(-1);
        });
        setError(null);
      }
    } catch (e) {
      setError(friendlyError(e, "Failed to open note"));
    }
  }, []);

  const createNote = useCallback(async (title: string, content: string) => {
    try {
      const note = await invoke<Note>("create_note", { title, content });
      withViewTransition(() => {
        setActiveNote(note);
        setEditorContent(content);
        setIsEditing(true);
        setSelectedIdx(-1);
      });
      setQuery("");
      await search("");
      setError(null);
    } catch (e) {
      setError(friendlyError(e, "Failed to create note"));
    }
  }, [search]);

  const saveNote = useCallback(async (content: string) => {
    if (!activeNote) return;
    try {
      const updated = await invoke<Note>("update_note", {
        id: activeNote.id,
        content,
      });
      setActiveNote(updated);
      setLastSaved(Date.now());
      await search(query);
      setError(null);
    } catch (e) {
      setError(friendlyError(e, "Failed to save note"));
    }
  }, [activeNote, query, search]);

  const deleteNote = useCallback(async (id: string) => {
    try {
      const noteToUndo = activeNote ? { ...activeNote } : null;
      await invoke("delete_note_cmd", { id });
      setActiveNote(null);
      setIsEditing(false);
      setEditorContent("");
      await search(query);
      setError(null);

      // Set up undo
      if (noteToUndo) {
        setDeletedNote(noteToUndo);
        const timeout = setTimeout(() => {
          setDeletedNote(null);
        }, 5000);
        setUndoTimeout(timeout);
      }
    } catch (e) {
      setError(friendlyError(e, "Failed to delete note"));
    }
  }, [activeNote, query, search]);

  const undoDelete = useCallback(async () => {
    if (!deletedNote) return;
    if (undoTimeout) clearTimeout(undoTimeout);
    try {
      await invoke<Note>("create_note", { title: deletedNote.title, content: deletedNote.content });
      setDeletedNote(null);
      await search(query);
    } catch (e) {
      setError(friendlyError(e, "Failed to restore note"));
    }
  }, [deletedNote, undoTimeout, query, search]);

  // Reset delete confirmation when switching notes
  useEffect(() => {
    setDeleteConfirm(false);
    if (deleteTimeoutRef.current) clearTimeout(deleteTimeoutRef.current);
  }, [activeNote?.id]);

  // Debounced search
  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      search(query);
    }, 50);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [query, search]);

  // Autosave
  useEffect(() => {
    if (!isEditing || !activeNote) return;
    if (saveTimeoutRef.current) clearTimeout(saveTimeoutRef.current);
    saveTimeoutRef.current = setTimeout(() => {
      saveNote(editorContent);
    }, 300);
    return () => {
      if (saveTimeoutRef.current) clearTimeout(saveTimeoutRef.current);
    };
  }, [editorContent, isEditing, activeNote, saveNote]);

  // Focus search on mount, load notes folder
  useEffect(() => {
    search("");
    searchInputRef.current?.focus();
    invoke<string>("get_notes_folder").then(setNotesFolder).catch(() => {});
    invoke<string>("get_app_version").then(setAppVersion).catch(() => {});
  }, [search]);

  // Keyboard navigation — Enter opens selected note, or shows create confirmation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIdx((prev) => Math.min(prev + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIdx((prev) => Math.max(prev - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (showCreateConfirm) {
          createNote(query.trim(), query.trim());
          setShowCreateConfirm(false);
        } else if (selectedIdx >= 0 && selectedIdx < results.length) {
          selectNote(results[selectedIdx].id);
        } else if (query.trim()) {
          setShowCreateConfirm(true);
        }
      } else if (e.key === "Escape") {
        if (showShortcuts) {
          setShowShortcuts(false);
          return;
        }
        if (showCreateConfirm) {
          setShowCreateConfirm(false);
          return;
        }
        if (showFolderPicker) {
          setShowFolderPicker(false);
          return;
        }
        setDeleteConfirm(false);
        if (deleteTimeoutRef.current) clearTimeout(deleteTimeoutRef.current);
        if (isEditing) {
          setIsEditing(false);
        }
        setQuery("");
        setSelectedIdx(-1);
        searchInputRef.current?.focus();
      } else if (e.key === "?" && !isEditing && !query.trim()) {
        e.preventDefault();
        setShowShortcuts((prev) => !prev);
      }
    },
    [selectedIdx, results, query, selectNote, createNote, isEditing, showShortcuts, showCreateConfirm, showFolderPicker]
  );

  const handleEditorChange = useCallback((value: string) => {
    setEditorContent(value);
  }, []);

  const handleNewNote = useCallback(() => {
    setQuery("");
    setSelectedIdx(-1);
    setActiveNote(null);
    setIsEditing(true);
    setEditorContent("");
    searchInputRef.current?.focus();
  }, []);

  const handleNoteClick = useCallback(
    (id: string) => {
      selectNote(id);
    },
    [selectNote]
  );

  const handleNoteKeyDown = useCallback(
    (e: React.KeyboardEvent, id: string) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        selectNote(id);
      }
    },
    [selectNote]
  );

  const handleDeleteClick = useCallback(() => {
    if (!activeNote) return;
    if (deleteConfirm) {
      deleteNote(activeNote.id);
      setDeleteConfirm(false);
      if (deleteTimeoutRef.current) clearTimeout(deleteTimeoutRef.current);
    } else {
      setDeleteConfirm(true);
      deleteTimeoutRef.current = setTimeout(() => {
        setDeleteConfirm(false);
      }, 3000);
    }
  }, [activeNote, deleteConfirm, deleteNote]);

  // ── Sync handlers ──

  const fetchSyncStatus = useCallback(async () => {
    try {
      const res = await invoke<{ status: string; message: string }>("get_sync_status");
      setSyncStatus(res.status);
      setSyncMessage(res.message);
    } catch {
      setSyncStatus("not_configured");
    }
  }, []);

  useEffect(() => {
    fetchSyncStatus();
  }, [fetchSyncStatus]);

  const handleGenerateKey = useCallback(async () => {
    setSyncLoading(true);
    setSyncError(null);
    setKeyCopied(false);
    try {
      const res = await invoke<{ public_key: string; secret_key: string }>("generate_sync_key");
      setGeneratedKey(res.secret_key);
    } catch (e) {
      setSyncError(friendlyError(e, "Failed to generate key"));
    } finally {
      setSyncLoading(false);
    }
  }, []);

  const handleCopyKey = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(generatedKey);
      setKeyCopied(true);
    } catch {
      // Fallback: select the text so user can copy manually
    }
  }, [generatedKey]);

  const handleImportKey = useCallback(async () => {
    if (!syncKey.trim()) return;
    setSyncLoading(true);
    setSyncError(null);
    try {
      await invoke("import_sync_key", { keyStr: syncKey.trim() });
      setSyncStep("remote");
    } catch (e) {
      setSyncError(friendlyError(e, "Invalid key"));
    } finally {
      setSyncLoading(false);
    }
  }, [syncKey]);

  const handleConfigureRemote = useCallback(async () => {
    if (!syncRemoteUrl.trim()) return;
    setSyncLoading(true);
    setSyncError(null);
    try {
      const validation = await invoke<{ git_installed: boolean; key_exists: boolean; remote_reachable: boolean; errors: string[] }>(
        "validate_sync_setup",
        { remoteUrl: syncRemoteUrl.trim() }
      );
      if (validation.errors.length > 0) {
        setSyncError(validation.errors.join("\n"));
        setSyncLoading(false);
        return;
      }
      await invoke("configure_sync", { remoteUrl: syncRemoteUrl.trim(), branch: syncBranch });
      setSyncStep("done");
      fetchSyncStatus();
    } catch (e) {
      setSyncError(friendlyError(e, "Failed to configure remote"));
    } finally {
      setSyncLoading(false);
    }
  }, [syncRemoteUrl, syncBranch, fetchSyncStatus]);

  const handleSync = useCallback(async (direction: string) => {
    setSyncLoading(true);
    setSyncError(null);
    try {
      const res = await invoke<{ status: string; message: string }>("sync_notes", { direction });
      setSyncStatus(res.status);
      setSyncMessage(res.message);
      if (direction === "pull" || direction === "full") {
        setLastSynced(Date.now());
        await search(query);
      }
    } catch (e) {
      setSyncError(friendlyError(e, "Sync failed"));
    } finally {
      setSyncLoading(false);
    }
  }, [query, search]);

  const openSyncSetup = useCallback(() => {
    fetchSyncStatus();
    setShowSyncSetup(true);
    if (syncStatus === "not_configured") {
      setSyncStep("welcome");
    } else {
      setSyncStep("done");
    }
  }, [fetchSyncStatus, syncStatus]);

  // ── Auto sync every 5 minutes when configured ──
  useEffect(() => {
    if (syncStatus === "not_configured") return;

    const interval = setInterval(async () => {
      if (syncLoading) return;
      try {
        const res = await invoke<{ status: string; message: string }>("sync_notes", { direction: "full" });
        setSyncStatus(res.status);
        setSyncMessage(res.message);
        if (res.status === "idle" || res.status === "conflict") {
          setLastSynced(Date.now());
          await search(query);
        }
      } catch {
        // Silently fail — user can manually sync if needed
      }
    }, 5 * 60 * 1000);

    return () => clearInterval(interval);
  }, [syncStatus, syncLoading, query, search]);

  // ── Notes folder ──

  const handleChangeFolder = useCallback(async () => {
    if (!folderInput.trim()) return;
    try {
      await invoke("set_notes_folder", { folder: folderInput.trim() });
      setNotesFolder(folderInput.trim());
      setShowFolderPicker(false);
      await search("");
    } catch (e) {
      setError(friendlyError(e, "Failed to change notes folder"));
    }
  }, [folderInput, search]);

  return (
    <div className="app">
      {showSidebarView && (
        <div className="sidebar" role="navigation" aria-label="Notes list">
          <div className="sidebar-header">
            <label htmlFor="search-input" className="sr-only">Search notes</label>
            <input
              id="search-input"
              ref={searchInputRef}
              type="text"
              className="search-input"
              placeholder="Search notes..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              aria-label="Search notes"
            />
            <button
              className="new-note-btn"
              onClick={handleNewNote}
              aria-label="Create new note"
              title="New note"
            >
              +
            </button>
          </div>

          <div className="note-list" role="listbox" aria-label="Search results" ref={noteListRef}>
          {isLoading ? (
            <div className="empty-state">
              <div className="empty-state-hint">Loading...</div>
            </div>
          ) : (
            <>
          {highlightedResults.map((result, idx) => (
            <div
              key={result.id}
              data-note-id={result.id}
              className={`note-item ${idx === selectedIdx ? "selected" : ""} ${activeNote?.id === result.id ? "active" : ""}`}
              onClick={() => handleNoteClick(result.id)}
              onMouseEnter={() => setSelectedIdx(idx)}
              onKeyDown={(e) => handleNoteKeyDown(e, result.id)}
              role="option"
              aria-selected={idx === selectedIdx}
              tabIndex={0}
            >
              <div
                className="note-item-title"
                style={activeNote?.id === result.id ? { viewTransitionName: `note-title-${result.id}` } : undefined}
              >
                {result.highlightedTitle}
              </div>
              <div className="note-item-preview">
                {result.highlightedPreview}
              </div>
              <div className="note-item-time">
                {result.formattedTime}
              </div>
            </div>
          ))}

          {results.length === 0 && query.trim() && !showCreateConfirm && (
            <div
              className="empty-state"
              onClick={() => setShowCreateConfirm(true)}
              role="button"
              tabIndex={0}
              aria-label={`Create note titled "${query}"`}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  setShowCreateConfirm(true);
                }
              }}
            >
              <div className="empty-state-icon">+</div>
              <div className="empty-state-text">
                Create &quot;{query}&quot;
              </div>
              <div className="empty-state-hint">Press Enter to create</div>
            </div>
          )}

          {results.length === 0 && query.trim() && showCreateConfirm && (
            <div className="create-confirm" role="status" aria-live="polite">
              <div className="create-confirm-text">Create note?</div>
              <div className="create-confirm-title">&quot;{query}&quot;</div>
              <div className="create-confirm-hint">
                Press Enter to confirm, Escape or click outside to cancel
              </div>
              <button className="create-confirm-cancel" onClick={() => setShowCreateConfirm(false)}>
                Cancel
              </button>
            </div>
          )}

          {results.length === 0 && !query.trim() && (
            <div className="empty-state">
              <div className="empty-state-text">No notes yet</div>
              <div className="empty-state-hint">
                Type to search, press Enter to create
              </div>
            </div>
          )}
            </>
          )}
          </div>

        <div className="sidebar-footer">
          <button
            className="sync-status-btn"
            onClick={openSyncSetup}
            aria-label={`Sync status: ${syncStatus}. Click to configure.`}
            title={syncMessage || "Configure sync"}
          >
            <span className={`sync-dot sync-dot-${syncStatus}`} />
            <span className="sync-label">
              {syncStatus === "not_configured" && "Set up sync"}
              {syncStatus === "idle" && "Sync"}
              {syncStatus === "syncing" && "Syncing..."}
              {syncStatus === "error" && "Sync error"}
              {syncStatus === "conflict" && "Conflicts"}
            </span>
          </button>
          <button
            className="folder-btn"
            onClick={() => { setFolderInput(notesFolder); setShowFolderPicker(true); }}
            aria-label="Change notes folder"
            title="Change notes folder"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            </svg>
          </button>
          <button
            className="help-btn"
            onClick={() => setShowShortcuts(true)}
            aria-label="Show keyboard shortcuts"
            title="Keyboard shortcuts"
          >
            ?
          </button>
          <span className="version-label">{appVersion ? `v${appVersion}` : ""}</span>
        </div>
      </div>
      )}

      {showEditorView && (
      <div className="editor-pane">
        {error && (
          <div className="error-banner" role="alert" aria-live="assertive">
            <span>{error}</span>
            <button onClick={() => setError(null)} aria-label="Dismiss error">
              Dismiss
            </button>
          </div>
        )}

        {deletedNote && (
          <div className="undo-banner" role="status" aria-live="polite">
            <span>Note deleted</span>
            <button onClick={undoDelete} aria-label="Undo delete">
              Undo
            </button>
          </div>
        )}

        {syncStatus === "conflict" && (
          <div className="conflict-banner" role="alert" aria-live="assertive">
            <span>{syncMessage}</span>
            <button onClick={() => { setSyncStatus("idle"); setSyncMessage(""); }} aria-label="Dismiss conflict warning">
              Dismiss
            </button>
          </div>
        )}

        {isEditing ? (
          <div className="editor-container">
            <div className="editor-header">
              {isNarrow && (
                <button
                  className="back-btn"
                  onClick={() => {
                    setIsEditing(false);
                    setActiveNote(null);
                    searchInputRef.current?.focus();
                  }}
                  aria-label="Back to notes list"
                >
                  ←
                </button>
              )}
              <span
                className="editor-title"
                style={activeNote ? { viewTransitionName: `note-title-${activeNote.id}` } : undefined}
                aria-label={`Editing: ${displayTitle}`}
              >
                {displayTitle}
              </span>
              {activeNote && (
                <button
                  className={`delete-btn ${deleteConfirm ? "delete-btn-confirm" : ""}`}
                  onClick={handleDeleteClick}
                  aria-label={deleteConfirm ? "Click again to confirm deletion" : "Delete this note"}
                  title={deleteConfirm ? "Click again to confirm" : "Delete note"}
                >
                  {deleteConfirm ? "Click again to delete" : "Delete"}
                </button>
              )}
            </div>
            <CodeMirror
              value={editorContent}
              height="100%"
              extensions={[markdown(), ...editorTheme]}
              onChange={handleEditorChange}
              basicSetup={{
                lineNumbers: false,
                foldGutter: false,
                dropCursor: false,
                allowMultipleSelections: true,
                indentOnInput: true,
              }}
            />
          </div>
        ) : (
          <div className="editor-empty">
            <div className="editor-empty-text">
              <svg className="app-logo" width="48" height="48" viewBox="0 0 32 32" fill="none">
                <rect x="7" y="4" width="18" height="24" rx="2" stroke="currentColor" strokeWidth="1.5"/>
                <g stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                  <line x1="11" y1="10" x2="21" y2="10"/>
                  <line x1="11" y1="14" x2="19" y2="14"/>
                  <line x1="11" y1="18" x2="17" y2="18"/>
                  <line x1="11" y1="22" x2="15" y2="22"/>
                </g>
              </svg>
              <div className="logo">nvAge</div>
              <div className="hint">Type to search, Enter to create</div>
              <div className="shortcuts">
                <div>↑/↓ Navigate</div>
                <div>Enter Open / Create</div>
                <div>Escape Back to search</div>
              </div>
              <button
                className="theme-toggle-btn-empty"
                onClick={toggleTheme}
                aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
                title={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
              >
                {theme === "dark" ? (
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
                  </svg>
                ) : (
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <circle cx="12" cy="12" r="5"/>
                    <line x1="12" y1="1" x2="12" y2="3"/>
                    <line x1="12" y1="21" x2="12" y2="23"/>
                    <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
                    <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
                    <line x1="1" y1="12" x2="3" y2="12"/>
                    <line x1="21" y1="12" x2="23" y2="12"/>
                    <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
                    <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
                  </svg>
                )}
              </button>
            </div>
          </div>
        )}

        {showShortcuts && (
          <div
            className="shortcuts-overlay"
            onClick={() => setShowShortcuts(false)}
            role="dialog"
            aria-modal="true"
            aria-label="Keyboard shortcuts"
          >
            <div className="shortcuts-card" onClick={(e) => e.stopPropagation()}>
              <div className="shortcuts-title">Keyboard Shortcuts</div>
              <div className="shortcut-row">
                <kbd>Enter</kbd>
                <span>Create / Open note</span>
              </div>
              <div className="shortcut-row">
                <kbd>↑</kbd> <kbd>↓</kbd>
                <span>Navigate results</span>
              </div>
              <div className="shortcut-row">
                <kbd>Esc</kbd>
                <span>Back to search</span>
              </div>
              <div className="shortcut-row">
                <kbd>+</kbd>
                <span>New note</span>
              </div>
              <div className="shortcut-row">
                <kbd>?</kbd>
                <span>This help (when search is empty)</span>
              </div>
              <div className="shortcut-row">
                <kbd>Del</kbd>
                <span>Delete note (click twice to confirm)</span>
              </div>
            </div>
          </div>
        )}

        {showFolderPicker && (
          <div className="folder-overlay" onClick={() => setShowFolderPicker(false)}>
            <div className="folder-card" onClick={(e) => e.stopPropagation()}>
              <div className="folder-card-title">Notes folder</div>
              <div className="folder-card-desc">
                Your notes are stored as plain Markdown files in this folder.
              </div>
              <input
                className="folder-input"
                type="text"
                value={folderInput}
                onChange={(e) => setFolderInput(e.target.value)}
                placeholder="/path/to/notes"
                aria-label="Notes folder path"
              />
              <div className="folder-actions">
                <button className="sync-primary-btn" onClick={handleChangeFolder}>
                  Change
                </button>
                <button className="sync-secondary-btn" onClick={() => setShowFolderPicker(false)}>
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}

        {/* ── Sync status indicator (bottom corner) ── */}
        {syncStatus !== "not_configured" && (
          <div className="sync-indicator" title={
            lastSaved <= lastSynced
              ? "All notes synced"
              : "Unsynced changes pending"
          }>
            <span className={`sync-indicator-dot ${lastSaved <= lastSynced ? "synced" : "unsynced"}`} />
          </div>
        )}
      </div>
      )}

      {/* ── Sync Setup Overlay ── */}
      {showSyncSetup && (
        <div className="sync-overlay" onClick={() => setShowSyncSetup(false)}>
          <div className="sync-card" onClick={(e) => e.stopPropagation()}>
            <button className="sync-close-btn" onClick={() => setShowSyncSetup(false)} aria-label="Close">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>

            {/* Step 1: Welcome */}
            {syncStep === "welcome" && (
              <div className="sync-step">
                <div className="sync-step-title">Sync your notes</div>
                <div className="sync-step-desc">
                  Keep your notes safe across devices. Your notes are encrypted before they leave this computer — only you can read them.
                </div>
                <button className="sync-primary-btn" onClick={() => setSyncStep("key")}>
                  Get started
                </button>
                <button className="sync-secondary-btn" onClick={() => setShowSyncSetup(false)}>
                  Not now
                </button>
              </div>
            )}

            {/* Step 2: Key */}
            {syncStep === "key" && (
              <div className="sync-step">
                <div className="sync-step-title">Your encryption key</div>
                <div className="sync-step-desc">
                  This key is used to encrypt your notes before they are sent to your remote storage. Your notes on this device are always safe — they stay as plain Markdown files on your computer.
                </div>

                <div className="sync-warning">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
                    <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
                  </svg>
                  <span>If you lose this key, your synced copies on other devices cannot be read. Copy it somewhere safe before continuing.</span>
                </div>

                {!generatedKey && (
                  <button className="sync-primary-btn" onClick={handleGenerateKey} disabled={syncLoading}>
                    {syncLoading ? "Generating..." : "Generate a key for me"}
                  </button>
                )}

                {generatedKey && (
                  <div className="sync-key-display">
                    <span className="sync-key-label">Your secret key</span>
                    <code className="sync-key-code">{generatedKey}</code>
                    <button
                      className={`sync-primary-btn ${keyCopied ? "key-copied" : ""}`}
                      onClick={handleCopyKey}
                      style={{ marginTop: "12px" }}
                    >
                      {keyCopied ? "Copied" : "Copy key"}
                    </button>
                    {keyCopied && (
                      <button
                        className="sync-secondary-btn"
                        onClick={() => setSyncStep("remote")}
                        style={{ marginTop: "8px" }}
                      >
                        Continue
                      </button>
                    )}
                  </div>
                )}

                {!generatedKey && (
                  <>
                    <div className="sync-divider">
                      <span>or</span>
                    </div>

                    <div className="sync-import">
                      <label htmlFor="sync-key-input" className="sync-label-text">Already have a key from another device?</label>
                      <textarea
                        id="sync-key-input"
                        className="sync-textarea"
                        value={syncKey}
                        onChange={(e) => setSyncKey(e.target.value)}
                        placeholder="AGE-SECRET-KEY-1..."
                        rows={3}
                      />
                      <button className="sync-secondary-btn" onClick={handleImportKey} disabled={syncLoading || !syncKey.trim()}>
                        {syncLoading ? "Importing..." : "Use this key"}
                      </button>
                    </div>
                  </>
                )}

                {syncError && <div className="sync-error-text">{syncError}</div>}
              </div>
            )}

            {/* Step 3: Remote */}
            {syncStep === "remote" && (
              <div className="sync-step">
                <div className="sync-step-title">Where to store your notes</div>
                <div className="sync-step-desc">
                  Your notes will be encrypted and stored in a Git repository. If you use GitHub, create a <strong>private</strong> repo first.
                </div>

                <div className="sync-tip">
                  <strong>New to GitHub?</strong>{" "}
                  <a href="https://github.com/new" target="_blank" rel="noopener noreferrer">
                    Create a private repo here
                  </a>, then copy the URL below the repo name.
                </div>

                <div className="sync-import">
                  <label htmlFor="sync-remote-input" className="sync-label-text">Repository URL</label>
                  <input
                    id="sync-remote-input"
                    className="sync-input"
                    type="text"
                    value={syncRemoteUrl}
                    onChange={(e) => setSyncRemoteUrl(e.target.value)}
                    placeholder="https://github.com/yourname/your-repo.git"
                  />

                  <label htmlFor="sync-branch-input" className="sync-label-text">Branch</label>
                  <input
                    id="sync-branch-input"
                    className="sync-input"
                    type="text"
                    value={syncBranch}
                    onChange={(e) => setSyncBranch(e.target.value)}
                    placeholder="main"
                  />
                </div>

                <button className="sync-primary-btn" onClick={handleConfigureRemote} disabled={syncLoading || !syncRemoteUrl.trim()}>
                  {syncLoading ? "Connecting..." : "Connect"}
                </button>

                {syncError && <div className="sync-error-text">{syncError}</div>}
              </div>
            )}

            {/* Step 4: Done */}
            {syncStep === "done" && (
              <div className="sync-step">
                <div className="sync-step-title">Sync</div>
                <div className="sync-step-desc">
                  {syncMessage || "Your notes are ready to sync."}
                </div>

                <div className="sync-actions">
                  <button className="sync-primary-btn" onClick={() => handleSync("full")} disabled={syncLoading}>
                    {syncLoading ? "Syncing..." : "Sync now"}
                  </button>
                  <button className="sync-secondary-btn" onClick={() => handleSync("push")} disabled={syncLoading}>
                    Push only
                  </button>
                  <button className="sync-secondary-btn" onClick={() => handleSync("pull")} disabled={syncLoading}>
                    Pull only
                  </button>
                </div>

                {syncError && <div className="sync-error-text">{syncError}</div>}

                <div className="sync-divider"><span>or</span></div>

                <button className="sync-secondary-btn" onClick={() => setShowSyncSetup(false)}>
                  Close
                </button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
