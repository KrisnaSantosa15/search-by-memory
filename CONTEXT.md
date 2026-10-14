# Search by Memory

A local-first desktop launcher that finds files on the user's machine from a natural-language description of what the user remembers about them, rather than from filenames or exact keywords.

## Language

### Content

**Location**:
A position inside a file expressed in terms the user can navigate to: a page number for paged documents, a line range for text and code, a time range for audio and video, or the whole file for images.
_Avoid_: Offset, position, chunk ID

**Chunk**:
A contiguous slice of a file's extracted content, with its Location, that is embedded and searched as a unit.
_Avoid_: Segment, block, passage (when meaning the stored unit)

**Snippet**:
The verbatim text of a Chunk as shown to the user; never paraphrased or generated.
_Avoid_: Summary, excerpt, preview text

### Indexing

**Indexed Folder**:
A folder or whole drive the user has chosen to make searchable; everything beneath it is indexed unless excluded.
_Avoid_: Scope, root, watched folder, library

**Exclusion**:
A folder or name pattern the user does not want indexed, even though it sits beneath an Indexed Folder.
_Avoid_: Ignore rule, filter, blacklist

**Default Exclusions**:
The Exclusions pre-filled by the app for well-known system and tooling folders, which the user can remove or extend.
_Avoid_: Built-in ignores, system filter

**Online-only File**:
A cloud-synced file (OneDrive, Google Drive, Dropbox) whose name exists on disk but whose content still lives in the cloud; it appears in the Name Index but not the Content Index until the user downloads it.
_Avoid_: Placeholder, stub, cloud file, dehydrated file

**Disconnected Drive**:
A drive holding an Indexed Folder (USB, external, or network) that is currently unavailable; its files stay in both indexes and appear in Results flagged as not connected, and are re-checked for changes when the drive returns.
_Avoid_: Offline drive ("offline" means no internet in this project), unmounted drive, missing drive

**Missing File**:
A file still in the indexes whose drive is connected but which no longer exists on disk, because it was deleted or moved without the app noticing yet.
_Avoid_: Deleted file, stale file, orphan

**Copy**:
One of several files at different paths whose content is byte-for-byte identical; Copies share one set of Chunks and are shown together as a single Result in File view.
_Avoid_: Duplicate, location (Location means a page or line range), instance

**Name Index**:
The index of every file beneath the Indexed Folders (minus Exclusions), searchable by filename and folder path, regardless of file format.
_Avoid_: File index, catalog, filename search

**Content Index**:
The index of Chunks from files in supported formats, searchable by meaning and by keyword.
_Avoid_: Semantic index, vector index, embedding index

**Initial Indexing**:
The first pass over newly added Indexed Folders: the Name Index is built in full first, then the Content Index is filled from the most recently modified files back to the oldest.
_Avoid_: Full scan, bootstrap, backfill

**Live Update**:
Indexing of a file created or changed after Initial Indexing began; it is processed ahead of any remaining Initial Indexing work.
_Avoid_: Incremental index, delta, refresh

### Searching

**Query**:
The natural-language description the user types into the launcher.
_Avoid_: Prompt, search term, keyword

**Content Match**:
A Chunk that is relevant to the Query, by meaning or by keyword.
_Avoid_: Match (unqualified), hit, candidate

**Name Match**:
A file whose filename or folder path is relevant to the Query, found through the Name Index.
_Avoid_: Filename hit, title match

**Match Reason**:
The label on a Result stating truthfully why it was returned: by name, by content, or both.
_Avoid_: Explanation, badge, justification

**Result**:
One row in the launcher's result list: one file (with its Copies), found by a Name Match, Content Matches, or both.
_Avoid_: Item, entry

**File view**:
The way the launcher lists Results: one per file, shown with its best Content Match (if any).
_Avoid_: Grouped view, document view

**Preview Pane**:
The right-hand side of the launcher, showing the selected Result: its best Content Match with the Query's words marked, where the file is, why it matched, its Copies, and the actions to open or show it.
_Avoid_: Detail Panel, sidebar, inspector

**Weaker Match**:
A Result that is related to the Query but much less so than the best ones; listed under a collapsed group so it never crowds the strong Results.
_Avoid_: Low-score hit, noise

**Passage view**:
One Result per Content Match, so a single file may appear several times. Kept in the index API and the Benchmark; the launcher does not show it.
_Avoid_: Chunk view, flat view

### Evaluation

**Benchmark**:
A fixed set of labelled Queries used to measure retrieval quality over time.
_Avoid_: Test set, eval set, golden set

**Target**:
The labelled correct answer for a Benchmark Query: a file plus the Location where the remembered content lives.
_Avoid_: Ground truth, expected result, answer

**Public Corpus**:
A shared, reproducible collection of files that Benchmark Queries are labelled against, so results can be compared and run in CI.
_Avoid_: Sample data, fixtures

**Personal Corpus**:
The developer's own files, used for private reality checks; its Benchmark results are never shared.
_Avoid_: Real data, my files

**Location Accuracy**:
How often the Snippet shown for a correctly found file overlaps the Target's Location.
_Avoid_: Snippet accuracy, page accuracy
