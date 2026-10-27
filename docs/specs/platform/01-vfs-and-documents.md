# Platform spec 01: VFS, documents, file types, language detection

Status: specification (clean-room, written from IntelliJ IDEA Community behavior). ID prefix `PLAT-VFS`.
Priorities: `MUST` (needed for the first Java/Kotlin/Spring code-intelligence release), `SHOULD` (needed
for parity, schedule after the MUSTs), `LATER` (document now, build later, but do not design it out).

Supersedes and deepens section 1.1 of `docs/analysis/code-intelligence-spec.md`. Disagreements are
listed in `README.md` (section "Disagreements with the code-intelligence spec").

## 1. Scope and vocabulary

This document describes the layer that turns "files on disk" into "text and identity the analysis can
rely on", and the layer that decides *what kind of file* something is and *which language* parses it.

| Term | Meaning in this spec |
|---|---|
| **Virtual file** | The IDE's in-memory identity of one file or directory (a handle plus cached metadata). It is not the disk; it is a cache of the disk that is reconciled by *refresh*. |
| **VFS** | The set of virtual files, the file systems behind them (local disk, archives, in-memory), and the change events they emit. |
| **File id** | A small positive integer that identifies a virtual file and is usable as an index key. |
| **Document** | The editable text of one file while the IDE holds it in memory: always `\n` line breaks, a modification stamp, an "unsaved" flag. |
| **Committed** | A document whose syntax tree (PSI) reflects its latest text. "Uncommitted" documents have text that is ahead of their tree. |
| **Physical** | A file or PSI that corresponds to a real file in the project (as opposed to a synthetic copy, a code fragment, a preview, a scratch buffer used only for analysis). |
| **File type** | A classification of a file (Java, XML, plain text, binary, ...) by name, content or special rules. It decides the icon, whether the file is text, which language parses it, and which indexes accept it. |
| **Language** | An identity for a grammar plus the set of language-support services registered for it. Several file types may share one language (many XML dialects). |

Source files studied: `ic: platform/core-api/src/com/intellij/openapi/vfs/` (`VirtualFile.java`,
`VirtualFileManager.java`, `AsyncFileListener.java`, `PersistentFSConstants.java`, `newvfs/BulkFileListener.java`,
`newvfs/events/*`, `limits/FileSizeLimit.kt`), `ic: platform/core-api/src/com/intellij/openapi/fileEditor/FileDocumentManager.java`,
`ic: platform/core-api/src/com/intellij/openapi/fileTypes/` (`FileType.java`, `LanguageFileType.java`, `FileTypeRegistry.java`),
`ic: platform/platform-impl/src/com/intellij/openapi/fileTypes/impl/` (`FileTypeManagerImpl.java`, `FileTypeDetectionService.java`),
`ic: jps/model-impl/src/com/intellij/openapi/fileTypes/impl/FileTypeAssocTable.java`,
`ic: platform/core-api/src/com/intellij/lang/LanguageUtil.java`, `ic: platform/core-api/src/com/intellij/psi/LanguageSubstitutor(s).java`,
`ic: platform/core-impl/src/com/intellij/psi/impl/PsiDocumentManagerBase.java`, `ic: platform/core-api/src/com/intellij/psi/PsiDocumentManager.java`,
`ic: platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/FileDocumentManagerImpl.java`,
`ic: platform/core-impl/src/com/intellij/openapi/fileEditor/impl/LoadTextUtil.java`,
`ic: platform/util-rt/src/com/intellij/openapi/util/io/FileUtilRt.java`, `ic: platform/util/resources/misc/registry.properties`.

## 2. Model

### 2.1 Virtual file

A virtual file has: a name; a path unique within its file system (forward slashes); a parent (none for a
root); children (for directories); flags (directory, writable, hidden, symlink, special); a length;
a disk timestamp; a **content modification stamp** (changes on every content change and is unrelated to
the disk timestamp); a charset and optional BOM; arbitrary user data; and a file id.

Lifecycle: `valid` until the file is deleted (or an ancestor is deleted, or an external refresh finds it
gone). Once invalid it never becomes valid again; a new file at the same path is a different virtual file.
Only identity-ish operations remain meaningful on an invalid file (name, path, URL, equality, hash, user data).

### 2.2 File systems

| File system | Path shape | Notes |
|---|---|---|
| Local | absolute path, `/` separators | Backed by the disk; watched. |
| Archive (jar/zip) | `<archive path>!/<entry path>` | Read-only. Library classes and sources live here. |
| Temporary / in-memory | any | Not on disk, never indexed as project content. Used for scratch, previews, light (synthetic) files. |
| Synthetic copies | n/a | Copy of a physical file used by analysis (completion copy, reparse copy). Never fires events, never indexed. |

### 2.3 Document

A document holds the full text with `\n` as the only line separator. On load, the original separator style
of the file is remembered and re-applied on save. A document has a modification stamp that changes on every
edit, an "unsaved" state (differs from what was last read from or written to disk), and a link to its virtual file.
Documents are created lazily on first request and may be dropped when nobody holds them (they are cached
weakly), except for in-memory files where they stay.

### 2.4 Commit model (document and syntax tree)

Edits go to the document first. The syntax tree of a physical file is brought up to date by a **commit**:

1. Every document change marks the document *uncommitted* (unless the change was produced by the tree itself,
   in which case the document and tree are already consistent, and unless the file has no tree).
2. After a change the IDE schedules an **asynchronous commit**: the new tree is computed on a background thread
   (incrementally where possible, see spec 02) and the result is applied under a short exclusive section.
   If a change arrives while a commit runs, that commit is discarded and redone.
3. Code that needs a current tree *now* (typing handlers, refactorings, actions) requests a **synchronous commit**
   of one document or of all documents. Background code either waits for "all documents committed" or reads the
   tree together with the *last committed text*, never the raw document.
4. While a document is uncommitted, tree elements still describe the last committed text. Their offsets do
   **not** match the document. The platform exposes the last committed text and stamp so readers can map.
5. Changes made through the tree (rename, quick fix, refactoring) are written back to the document at the end of
   the exclusive section that made them. A document may not be edited directly while it holds pending tree changes.
6. Bulk edits (many changes inside a bulk-update bracket) defer the commit until the bracket ends, and force a full
   reparse instead of an incremental one. A whole-text replacement of a document longer than 100,000 characters
   also forces a full reparse.

## 3. Requirements

### 3.1 Identity and lifecycle

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-001 | MUST | Every file and directory known to the IDE has exactly one identity for the whole process lifetime while it exists. Two lookups of the same path yield equal identities with the same hash and the same attached user data. | Look up one path twice, attach a value through the first handle, read it through the second. |
| PLAT-VFS-002 | MUST | Rename and move keep the identity. They change the name or parent and emit a property or move event. Only deletion ends an identity. | Rename a file in the IDE; the old handle still resolves, now with the new name; an index keyed by file id still finds the file. |
| PLAT-VFS-003 | MUST | After deletion (of the file or any ancestor) the identity reports invalid. Operations other than name, path, URL, equality, hash and user data on an invalid identity return a typed error, never panic. | Delete a directory; every handle below it is invalid; calling `children()` on one returns the error. |
| PLAT-VFS-004 | MUST | Each virtual file has a numeric file id, positive and dense enough to index arrays or bitsets. The id is stable across restarts while the persistent VFS is intact. | Restart the IDE with an unchanged tree; ids of unchanged files are identical. |
| PLAT-VFS-005 | MUST | If the persistent VFS is discarded or found corrupt, the id space restarts **and every index and cache keyed by file id is discarded with it**. A stale index must never be read against a new id table. | Corrupt the VFS store; after restart all indexes are rebuilt and no query returns a file other than the one indexed. |
| PLAT-VFS-006 | MUST | A change observed from outside the IDE as "file X disappeared, file Y appeared" is delivered as delete + create (new id), unless a pairing watcher event proves it was a rename. | `mv a b` outside the IDE: old identity invalid, new identity has a new id (or, when pairing is supported, a move event with the old id). |
| PLAT-VFS-007 | SHOULD | The IDE may pair external rename events from the OS watcher into one rename/move so ids and index entries survive. Pairing must only be used when the watcher reports both ends in one event. | Watcher reports a rename pair; listeners see one move/rename event, not delete+create. |
| PLAT-VFS-008 | MUST | Child enumeration order is deterministic (stable sort by name) for a given directory state. Name lookup honors the directory's case sensitivity. | Enumerate twice, equal order; on a case-insensitive directory `Foo.java` finds `foo.java`. |
| PLAT-VFS-009 | SHOULD | Case sensitivity is a per-directory property, queried from the OS, and a change of it emits a property-change event. | Mount a case-insensitive directory inside a case-sensitive tree; lookups differ per directory. |
| PLAT-VFS-010 | SHOULD | The VFS persists the file tree (names, flags, lengths, timestamps, ids) between runs, so opening a known project does not rescan every directory; a refresh reconciles on open. | Open a 50k-file project twice; the second open does not list directories that did not change before the first index query. |
| PLAT-VFS-011 | LATER | Cache file bytes of files up to 1 MiB inside the persistent VFS to avoid disk reads for hot files. Larger files are never cached. | Reading a 500 KB file twice performs one disk read; a 2 MB file performs two. |

### 3.2 Paths, URLs and file systems

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-020 | MUST | A path is absolute, uses `/` only, has no trailing slash and (for local files) no `.` or `..` segments. The UI never shows this path; it shows a presentable (OS-style) path. | Round trip a Windows-style path; internal form uses `/`, presented form uses `\`. |
| PLAT-VFS-021 | MUST | A URL is `<protocol>://<path>` and is unique across file systems. The protocol of the local file system is `file`; archive entries use `jar` with `!/` between archive and entry. Finding a file by URL works for every registered protocol. | `jar:///repo/lib/a.jar!/org/A.class` resolves to the entry; its `file` URL parent is the archive file. |
| PLAT-VFS-022 | MUST | Relative path resolution from a file supports `.` (stay), `..` (parent), repeated slashes, and, for a symlink in the middle, `..` resolves to the parent of the link **target**, not of the link. | Resolve `../x` from a file reached through a symlink; the result is next to the target. |
| PLAT-VFS-023 | MUST | Symlinks are modeled: the virtual file has a symlink flag and a canonical target (or none if broken). Analysis never follows a symlink loop; a loop is reported as a broken link. | A directory symlink pointing to its ancestor does not make the project scan infinite. |
| PLAT-VFS-024 | MUST | Archive files expose their entries as a read-only tree, opened lazily on first access. Entries have length and timestamp. The archive handle is released when no virtual file inside is referenced for a while. | Open a 200 MB jar, list one package; only the central directory is read; memory stays bounded. |
| PLAT-VFS-025 | SHOULD | The JDK's runtime image (the `jrt` virtual file system) is exposed like an archive so JDK classes resolve like library classes. | A project with a JDK 17 SDK resolves `java.util.List` to a class inside the JDK image. |
| PLAT-VFS-026 | MUST | In-memory files (scratch, preview, copies) are virtual files of a non-physical file system. They have no id in the persistent VFS, are not indexed as project files, never produce VFS change events for disk, and are never counted as "in project". | Create a scratch file with text; it parses, highlights, resolves, and does not appear in "Go to File". |

### 3.3 Content, charset, line separators, size limits

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-030 | MUST | Content is available as bytes and as text. Text is decoded with the file's charset and has all line separators normalized to `\n`. The original separator style is remembered per file and restored on save. | Load a CRLF file, edit, save: file is still CRLF; document text contains only `\n`. |
| PLAT-VFS-031 | MUST | Charset decision, first match wins: (1) a charset declared by the file type or by the content itself (for example an XML prolog or a Python coding comment); (2) a BOM; (3) valid multi-byte UTF-8 content that cannot be pure ASCII is UTF-8; (4) an explicit per-file, per-directory or per-project setting; (5) the default charset (UTF-8). | A file with non-ASCII bytes forming valid UTF-8 opens as UTF-8 even if the project default is another charset; a Latin-1 file with an explicit setting opens as Latin-1. |
| PLAT-VFS-032 | MUST | A detected BOM is remembered and rewritten on save; it is not part of the text. | Save a BOM file unchanged: bytes are identical. |
| PLAT-VFS-033 | MUST | Charset detection on a partial read (a prefix of the file) never stores a charset on the file. Only a full read may. | Read the first 1 KB for file type detection; the file's charset is still unset afterwards. |
| PLAT-VFS-034 | MUST | A file larger than the **intelligence limit** is treated as plain text for parsing and is not indexed for content-based indexes (it stays in the file-name index). Default limit: about 2.5 MB (2,500 KiB). The limit can be raised per file extension, never lowered below the default. | A 3 MB `.java` file opens with no syntax tree, no semantic highlighting, and no entries in word/stub indexes; a 2 MB file does. |
| PLAT-VFS-035 | MUST | A file larger than the **content loading limit** (default 20 MB, never below it) is not loaded into a document at all; it opens in a read-only preview of the first part, with no editing and no parsing. | Open a 30 MB log file; the editor shows a bounded preview and memory stays flat. |
| PLAT-VFS-036 | SHOULD | Both limits can be overridden per extension through a contribution point; the larger of the contribution and the default wins. | Contribute 8 MB for `.sql`; a 5 MB `.sql` is parsed; a 5 MB `.java` is not. |
| PLAT-VFS-037 | MUST | Binary files (by file type) have no document. They can still have a tree if a decompiler or reader is registered for their type (class files). | `getDocument` for a `.png` is absent; for a `.class` with a registered decompiler a document with decompiled text exists. |
| PLAT-VFS-038 | MUST | Content modification stamp: every content change (by the IDE or detected on refresh) changes it. Equal stamp means equal content. Disk timestamp is independent. | `touch` without content change does not change the stamp after refresh; a one-byte change does. |

### 3.4 Refresh and external change detection

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-040 | MUST | The VFS is a cache. Disk changes become visible only through a *refresh* (explicit, triggered by the OS watcher, on window activation, or periodic). Until then the old state is served. | Edit a file externally; `contents` returns old bytes until refresh runs, then new bytes. |
| PLAT-VFS-041 | MUST | Refresh may be asynchronous or synchronous, of one file, a subtree (recursive) or whole VFS. A synchronous refresh must not be started while holding a read lock (deadlock). An optional post-action runs after the resulting events were delivered. | Synchronous refresh from inside a read action is rejected with an error; asynchronous refresh with a post-action calls it after events. |
| PLAT-VFS-042 | MUST | Refresh of a directory compares its children with the disk and emits create/delete events; refresh of a file compares timestamp (and length) and emits a content-change event if different. Directories ignored or excluded are not descended. | Create 100 files externally; a recursive refresh emits creates for the topmost new directory only. |
| PLAT-VFS-043 | MUST | The OS watcher reports dirty paths; the IDE refreshes exactly those. If no watcher is available, the IDE falls back to periodic and focus-based refresh. | Kill the watcher process; edits are still noticed within the periodic interval. |
| PLAT-VFS-044 | MUST | Defaults: refresh on window activation enabled; background periodic refresh enabled with a 15 second pause between scans while idle/inactive; refresh worker pool of 6 threads. All configurable. | Switch windows after an external edit: file reloads without user action. |
| PLAT-VFS-045 | SHOULD | Files created inside excluded/ignored directories are not preloaded into the VFS (avoid caching build output and dependency folders). | `target/` with 100k class files does not create 100k virtual files. |
| PLAT-VFS-046 | MUST | The IDE's own writes (save, rename, create, delete through the IDE) update the VFS immediately and emit events synchronously; they do not wait for refresh and are tagged with a requestor so listeners can tell self-changes from external ones. | Save a file; listeners receive a content-change event whose "from save" flag is true; no second event arrives from the watcher. |

### 3.5 Change events

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-050 | MUST | Event kinds: create, delete, content change, move, copy, property change (name, encoding, writable, hidden, symlink target, children case sensitivity). Every event carries a path and (when still valid) the file, the requestor, and flags "from refresh" and "from save". | Unit-check one event of each kind has the fields above. |
| PLAT-VFS-051 | MUST | Events are delivered in **batches**. Listeners get a *before* call (state not yet changed) and an *after* call (state changed) for the same batch, both inside an exclusive (write) section. Listeners are project-agnostic and must filter by project themselves. | A listener registered in two open projects receives the batch once per project listener; the state seen in *before* is the old one. |
| PLAT-VFS-052 | MUST | A batch can contain several events about the same file (create then delete, delete then create, parent delete after child create). Listeners must process them in order and tolerate that a file named by an event is already invalid or not yet findable in the *after* phase. | Create then delete a file inside one batch; a listener that dereferences the file in *after* handles "invalid" without error. |
| PLAT-VFS-053 | MUST | When a directory tree is created, only the topmost directory gets a create event. Listeners that need the files iterate the new directory through the project file index (which skips excluded files). | Copy a 10k-file tree in; one create event; a subsequent index scan finds all files outside excluded folders. |
| PLAT-VFS-054 | MUST | Content-change events carry old and new modification stamp and old and new length. | A listener can tell a change from the IDE (stamp equals the document's) from an external change. |
| PLAT-VFS-055 | SHOULD | Offer a second listener flavor that is split into a side-effect-free *prepare* phase (runs on a worker thread in a read action, must be cancellable) and an *apply* phase (exclusive section, fast, before and after variants). Long work moves to *prepare*. | A listener that decides "does this batch touch build files" does so in *prepare*; the UI thread stays unblocked during a 100k-event batch. |
| PLAT-VFS-056 | MUST | A listener must never throw or panic into the VFS. Failure is isolated per listener; remaining listeners still run. | A panicking listener does not prevent the PSI/index listeners from receiving the batch. |
| PLAT-VFS-057 | SHOULD | The VFS exposes two modification counters: one for content changes of a single file (the stamp, see PLAT-VFS-038) and one for structural changes of the whole VFS (create, delete, move, rename), usable as a cache dependency. | A cache keyed on the structure counter is invalid after a file is created anywhere. |

### 3.5.1 Reaction of other subsystems to events (platform side)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-060 | MUST | On delete: the tree (PSI) of the file and all its smart pointers become invalid; its document is dropped (after saving or discarding per user choice); index entries are removed. | Delete a file with a tree and one open editor: editor closes or shows "file deleted"; usages search no longer returns hits in it. |
| PLAT-VFS-061 | MUST | On rename or move: the tree is *not* rebuilt unless file type or language changes; if it changes, the tree is rebuilt (old elements invalid). Package-dependent state (the Java package of a file derived from its directory) is recomputed. | Rename `A.java` to `A.txt`: tree is rebuilt as plain text. Rename `A.java` to `B.java`: elements stay valid. |
| PLAT-VFS-062 | MUST | On external content change: if the file has no document, the tree is dropped and rebuilt lazily; if it has a document, see section 3.8. | Edit externally a file that is closed; next tree access parses the new content. |
| PLAT-VFS-063 | MUST | On any VFS change that cannot be attributed to a loaded tree, the platform emits one generic "unloaded PSI changed" notification so caches that cannot subscribe to specific files drop themselves. | Add a file in a directory with no loaded trees; a listener to generic PSI change sees one event. |
| PLAT-VFS-064 | MUST | File type changes (user mapping, plugin load) invalidate every tree whose type may change, and trigger reparse of affected open and indexed files. | Change the mapping of `*.conf` from plain text to a registered language; open files reparse; indexes for those files rebuild. |

### 3.6 File types and language detection

Detection order for "what is this file" (first non-empty result wins):

1. A temporary override active during a bulk operation (a cache so many checks give one answer).
2. Override contributions (a list of providers; first non-null answer).
3. A type pinned to the virtual file (in-memory files may carry one).
4. "Special" file types that decide from the virtual file itself rather than the name (for example types identified by a content-independent file property).
5. **Name matching**, in this order: exact file name (case-sensitive), exact file name (case-insensitive), wildcard patterns (more specific pattern first: longer pattern text first), then the extension (the part after the last dot).
6. **Content detection**, only when the name gave no type or gave the *detected by content* marker: run content detectors on the first bytes; if none decides, look for a hashbang line; if the file is text, plain text; if binary, unknown; an empty file stays *detected by content* so it can be re-detected after it gains content.
7. Unknown.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-070 | MUST | A file type has a unique name, display name, description, default extension, icon (optional), a binary/text flag, a read-only flag, optional hard-coded or content-declared charset. A *language file type* additionally names a language. | Registering two types with the same name is rejected. |
| PLAT-VFS-071 | MUST | File type is determined by the detection order above. A matching result is cached per file and recomputed on change of mapping, on content change of an empty or content-detected file, and on rename. | Rename `x` (unknown, empty) to `x.json`: type becomes JSON immediately. |
| PLAT-VFS-072 | MUST | Name patterns: exact names, extensions (without dot, case-insensitive on the extension), and wildcard patterns with `*` and `?`. User mappings override built-in ones; removal of a built-in mapping is remembered. | Map `Jenkinsfile` exactly to Groovy; `Jenkinsfile.bak` is not matched. |
| PLAT-VFS-073 | MUST | Content detectors are cheap: they read a prefix (the longest prefix any detector asks for), never touch trees, indexes or documents, never throw into the caller, and are consulted in registration order; the first non-null answer wins. | A detector throwing is logged and skipped; detection still completes. |
| PLAT-VFS-074 | MUST | Auto-detection result "text" vs "binary" is cached persistently on the file (two flag bits plus an optional detected type), so restart does not re-read content. Binary files are not re-detected on every change; text files re-detect on content change when their type came from detection. | A 10 MB file is read for detection once across restarts. |
| PLAT-VFS-075 | SHOULD | Hashbang detection: a table maps interpreter lines (for example `#!/usr/bin/env python`) to file types for extensionless files. | Extensionless script with a python hashbang opens as Python. |
| PLAT-VFS-076 | MUST | Ignored files and folders (by name patterns) are excluded from the project file index and from indexing; the list is user-editable. Default list: `*.pyc`, `*.pyo`, `*.rbc`, `*.yarb`, `*~`, `.DS_Store`, `.git`, `.hg`, `.svn`, `CVS`, `__pycache__`, `_svn`, `vssver.scc`, `vssver2.scc`. Build output and dependency folders are handled by *exclusion* in the project model (spec 07), not by this list. | `.git` and `*~` files never appear in Go to File or in any index; `node_modules` is not ignored by this list. |
| PLAT-VFS-077 | MUST | Language of a file = (1) an explicit language of an in-memory file, else the language of its file type; then (2) generic substitutors (for example scratches) are asked, first non-null wins; then (3) substitutors registered for that language are asked, first non-null wins. Binary files have no language. A file above the intelligence limit gets plain text. | A `.xml` under a configured folder substituted to a dialect uses the dialect; a 3 MB `.java` is plain text. |
| PLAT-VFS-078 | MUST | A language substitutor is a pure function of (file, project). If its answer for a file changes later, the file's tree is rebuilt with the new language. | Change the setting that drives a substitutor; open files reparse with the new language. |
| PLAT-VFS-079 | MUST | A language has exactly one primary file type. A file type that reuses another language must be marked secondary and must not declare that language as its own, so "file type of language L" is unambiguous. | Registering two non-secondary file types for one language is rejected at startup. |
| PLAT-VFS-080 | SHOULD | "Is this file of type T" has a cheap form that avoids full detection when T is not name-matched (name check, then content check only if needed). | A check for "is Java class file" on 100k files does not read content of non-class files. |

### 3.7 Documents

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-090 | MUST | For a virtual file the platform returns its document: loaded on demand from the file's bytes, absent for directories, binary files without decompiler, and files above the content loading limit. A cached-only variant returns the document only if already loaded. | `get_document(dir)` is absent; `get_cached_document(f)` is absent until `get_document(f)` was called. |
| PLAT-VFS-091 | MUST | The inverse mapping (document to file) exists for documents created from files; it is absent for pure in-memory documents. | Round-trip file->document->file. |
| PLAT-VFS-092 | MUST | "Unsaved" documents are those whose text differs from disk. The platform lists them and can save all, a filtered set, or one. Saving applies line separators, charset and BOM, optionally strips trailing spaces on modified lines and ensures a final newline according to settings, and may therefore change the document while saving. | Save with "strip trailing spaces" on: document and file lose trailing spaces on edited lines only. |
| PLAT-VFS-093 | SHOULD | Defaults: save on window deactivation enabled; save after idle disabled by default with a 15 second timeout when enabled (range 1 to 300 seconds); writes use a safe-write strategy (write temp, then replace) enabled by default. | Deactivate the window: unsaved documents are written; a crash mid-write never leaves a half-written file. |
| PLAT-VFS-094 | MUST | A save emits a content-change event flagged "from save"; consumers (the document manager itself, file type redetection) skip their reload logic for it. | Save: the document is not reloaded; the stamp relation (document stamp equals file stamp) holds afterwards. |
| PLAT-VFS-095 | SHOULD | Editing a read-only file is refused with a message; "make writable" is offered where the VFS can do it. Version control and other policies can veto write access through a contribution point. | Edit a read-only file: the edit is rejected and the user is told why, with an offer to make it writable. |

### 3.8 External change versus unsaved edits

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-100 | MUST | When a content-change event arrives (not from save) for a file with a loaded document, the document is **reloaded silently** if any of: the document's stamp equals the event's *old* stamp (nobody edited since last sync); the document is not unsaved; or a three-way merge applies cleanly. | Change a file externally while the editor has no edits: text updates, caret kept. |
| PLAT-VFS-101 | MUST | Otherwise (the document is unsaved **and** the disk differs **and** merge failed) the IDE shows a conflict dialog titled "File Cache Conflict" with the text "Changes have been made to '{file}' in memory and on disk." and the buttons "Load File System Changes", "Keep Memory Changes", "Show Difference". The diff panes are labeled "File system content" and "Memory content". The save of that document is vetoed until resolved. | Edit in IDE and externally; dialog appears; "Keep Memory Changes" leaves text and resolves; "Load..." replaces text. |
| PLAT-VFS-102 | SHOULD | Automatic merge: only for events from refresh with prefetched disk content; needs the last known common text as the base; merges the disk text and the memory text against that base; fails (shows the dialog) if the merge reports conflicts. The action to run it by hand is "Merge External Changes"; the manual reload action is "Reload from Disk". | Disjoint edits merge into one text with no dialog. |
| PLAT-VFS-103 | MUST | Reloading a document from disk replaces the text as a whole (one document change), resets the unsaved flag, keeps the undo history unreachable across the reload, and triggers a full reparse. | After reload, `isCommitted` becomes true after the next commit and the old tree elements are invalid. |
| PLAT-VFS-104 | MUST | A file whose type is binary-with-decompiler gets its document replaced when the bytes change. | Replace a jar's class file; the decompiled text updates. |

### 3.9 Document and tree commit rules

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-110 | MUST | Every physical document has a committed state. A document is *committed* iff its tree reflects its text. The platform answers `is_committed(doc)`, `uncommitted_documents()`, and `has_uncommitted()`. | Type a character: `is_committed` is false until the commit finishes, then true. |
| PLAT-VFS-111 | MUST | An **asynchronous commit** is scheduled on every change of a physical document outside bulk updates and outside tree-originated changes. It parses off the UI thread and applies the result atomically; a new change cancels and reschedules it. | Type 20 characters quickly: at most the last one's parse is applied; no intermediate tree is applied after a newer edit. |
| PLAT-VFS-112 | MUST | A **synchronous commit** of one document and of all documents is available to code running on the UI/write thread. It must not be callable from inside a tree-change listener (asserts or errors). | Calling it from a tree-change listener returns the error `reentrant commit`. |
| PLAT-VFS-113 | MUST | Provide "run when this document is committed" and "run when all documents are committed": run immediately if already true, otherwise after the commit, on the UI thread, outside the exclusive section, and never inside the commit. The "later" variant never runs immediately and is dropped when the project closes. | Register an action while uncommitted: it runs once after the commit, not before. |
| PLAT-VFS-114 | MUST | `last_committed_text(doc)` and `last_committed_stamp(doc)` give the text and stamp the tree is based on (immutable text). For a committed document they equal the document's. A "last committed document" view lets background readers use document APIs (line/offset conversion) against the tree's text. | While uncommitted, offsets from the tree map correctly with the last committed text, not with the live document. |
| PLAT-VFS-115 | MUST | A document with pending tree changes is blocked for direct edits; the owner of the tree changes can flush them into the document mid-action. Direct edits on a blocked document are errors. | Rename via tree then edit the document directly in the same action without flushing: error. |
| PLAT-VFS-116 | MUST | Non-physical files (copies, fragments, previews) have no commit machinery: their tree is built eagerly from text, they send no change events, and they do not bump global modification counters. | Edit a completion copy: no `modification_count` change, no events. |
| PLAT-VFS-117 | MUST | Commit may reparse incrementally or fully; the result must be indistinguishable from a full reparse of the last committed text (same shape, same text for every element). | Property test: random edit sequences, incremental tree equals full-parse tree (see spec 02). |
| PLAT-VFS-118 | SHOULD | When more than 100 documents or more than the content-loading-limit worth of text are uncommitted at once (batch operations), the platform commits eagerly to bound memory. | Programmatically edit 500 documents in a loop; memory stays bounded. |
| PLAT-VFS-119 | MUST | Reparse of a set of files can be requested after external configuration changes (language level, file type mapping); "include open files" is a flag. | Change language level; open files reparse when requested, closed files reparse lazily. |

### 3.10 Threading rules of this layer

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-VFS-120 | MUST | Reading virtual file structure or document text requires read access; changing the VFS or a document requires write access; refresh events are applied inside write access. | A background thread without a read section calling `children()` is rejected in debug builds. |
| PLAT-VFS-121 | MUST | Heavy VFS work (directory listing, detection, charset) is never done on the UI thread inside event listeners; listeners use cheap name-based type checks only. | A listener that calls full file type detection on 10k events is a test failure (lint). |
| PLAT-VFS-122 | SHOULD | The VFS offers a "no caching" lookup for rarely accessed trees (a lookup that reuses cached entries but does not populate the cache for new ones) to scan external trees without growing the persistent VFS. | Scanning `~/.m2` for a single jar does not add 100k entries to the VFS. |

## 4. Notes for implementers (non-binding)

- Archipel today identifies files by path and delivers watcher batches; the minimum viable step is a file table that gives
  PLAT-VFS-001..006 and an event batch type with before/after phases (PLAT-VFS-051) even if both phases are delivered on one thread.
- The commit model can be *much* simpler in Archipel than in IntelliJ if incremental parsing is synchronous and cheap
  (tree-sitter): treat "commit" as "parse finished for revision N" and expose `last_committed_text` as the text of revision N.
  The observable rules PLAT-VFS-110..117 still hold and must be testable.
- Do not skip PLAT-VFS-052 and PLAT-VFS-100..101; the user-visible dialogs exist because silent data loss is unacceptable.

## 5. Open questions

1. Default of "save on frame deactivation" is on in the product settings file for this tag; "save when idle" default is off. Confirm
   with product whether Archipel keeps the same defaults.
2. Whether the in-memory VFS (scratch files) should be included in "Everything" scope. In IntelliJ it is (see spec 07).
3. Charset rule PLAT-VFS-031(3) (valid non-ASCII UTF-8 content wins over a configured non-UTF-8 default) was derived from reading
   the detection routine; verify with a corpus of Latin-1 files before shipping.
