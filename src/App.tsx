import { useState, useEffect, useRef, useCallback } from "react";
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

// ── Editor Theme Builder ──
function buildEditorTheme() {
  const style = getComputedStyle(document.documentElement);
  const v = (name: string) => style.getPropertyValue(name).trim();

  const highlightStyle = HighlightStyle.define([
    { tag: t.heading, color: v("--editor-heading"), fontWeight: "bold" },
    { tag: t.strong, color: v("--editor-heading"), fontWeight: "bold" },
    { tag: t.emphasis, color: v("--editor-emphasis"), fontStyle: "italic" },
    { tag: t.link, color: v("--editor-link") },
    { tag: t.url, color: v("--editor-link") },
    { tag: t.monospace, color: v("--editor-monospace"), fontFamily: "monospace" },
    { tag: t.strikethrough, color: v("--editor-strikethrough"), textDecoration: "line-through" },
    { tag: t.atom, color: v("--editor-atom") },
  ]);

  return [
    EditorView.theme({
      "&": {
        backgroundColor: `${v("--editor-bg")} !important`,
        color: v("--editor-text"),
        fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
        fontSize: "15px",
        lineHeight: "1.6",
      },
      ".cm-scroller": {
        padding: "0",
        backgroundColor: `${v("--editor-bg")} !important`,
      },
      ".cm-content": {
        padding: "16px",
        caretColor: v("--editor-cursor"),
        backgroundColor: `${v("--editor-bg")} !important`,
        color: v("--editor-text"),
      },
      ".cm-cursor": {
        borderLeftColor: v("--editor-cursor"),
      },
      ".cm-gutters": {
        display: "none",
        backgroundColor: `${v("--editor-bg")} !important`,
      },
      ".cm-activeLine": {
        backgroundColor: "transparent !important",
      },
      ".cm-focused .cm-activeLine": {
        backgroundColor: "transparent !important",
      },
      ".cm-selectionBackground": {
        background: `${v("--editor-selection")} !important`,
      },
      ".cm-line": {
        color: v("--editor-text"),
      },
    }),
    syntaxHighlighting(highlightStyle),
  ];
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
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    const saved = localStorage.getItem("nvage-theme");
    return (saved === "light" || saved === "dark") ? saved : "dark";
  });
  const [prevResultIds, setPrevResultIds] = useState<string[]>([]);
  const noteListRef = useRef<HTMLDivElement>(null);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const deleteTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Apply theme to document
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("nvage-theme", theme);
  }, [theme]);

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

  // Build editor theme from current CSS variables (rebuilds on theme change)
  const editorTheme = buildEditorTheme();

  const search = useCallback(async (q: string) => {
    try {
      const res = await invoke<SearchResult[]>("search_notes", { query: q });
      setResults(res);
      setError(null);
    } catch (e) {
      setError(`Search failed: ${e}`);
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
      setError(`Failed to open note: ${e}`);
    }
  }, []);

  // On narrow screens, hide sidebar when a note is opened
  useEffect(() => {
    if (isNarrow && isEditing) {
      // sidebar auto-hides via showSidebarView
    }
  }, [isNarrow, isEditing]);

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
      setError(`Failed to create note: ${e}`);
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
      await search(query);
      setError(null);
    } catch (e) {
      setError(`Failed to save note: ${e}`);
    }
  }, [activeNote, query, search]);

  const deleteNote = useCallback(async (id: string) => {
    try {
      await invoke("delete_note_cmd", { id });
      setActiveNote(null);
      setIsEditing(false);
      setEditorContent("");
      await search(query);
      setError(null);
    } catch (e) {
      setError(`Failed to delete note: ${e}`);
    }
  }, [query, search]);

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

  // Focus search on mount
  useEffect(() => {
    search("");
    searchInputRef.current?.focus();
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
        setDeleteConfirm(false);
        if (deleteTimeoutRef.current) clearTimeout(deleteTimeoutRef.current);
        if (isEditing) {
          setIsEditing(false);
        }
        setQuery("");
        setSelectedIdx(-1);
        searchInputRef.current?.focus();
      } else if (e.key === "?" && !isEditing) {
        e.preventDefault();
        setShowShortcuts((prev) => !prev);
      }
    },
    [selectedIdx, results, query, selectNote, createNote, isEditing, showShortcuts, showCreateConfirm]
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

  const formatRelativeTime = useCallback((dateStr: string) => {
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
  }, []);

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

          <div className="note-list" role="listbox" aria-label="Search results">
          {results.map((result, idx) => (
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
                {highlightText(result.title || "Untitled", query)}
              </div>
              <div className="note-item-preview">
                {result.preview ? highlightText(result.preview, query) : "Empty note"}
              </div>
              <div className="note-item-time">
                {formatRelativeTime(result.modified)}
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
              <div className="create-confirm-hint">Press Enter to confirm, Escape to cancel</div>
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
                aria-label={`Editing: ${activeNote?.title || "New note"}`}
              >
                {activeNote?.title || "New note"}
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
                <kbd>?</kbd>
                <span>This help</span>
              </div>
            </div>
          </div>
        )}
      </div>
      )}
    </div>
  );
}

export default App;
