# Buffer Model Deep Dive: Piece Tables and Beyond

## Introduction

The buffer is the heart of any text editor. This document explores the data structures used to store and manipulate text, from simple ropes to Fresh's piece table with integrated line tracking.

---

## Part 1: Buffer Data Structures Compared

### Gap Buffer

A gap buffer maintains a contiguous gap at the cursor position:

```
Initial: "Hello| World"  (| = cursor, gap at position 5)
Memory:  ['H','e','l','l','o',_,_,_,_,_,' ','W','o','r','l','d']
                   ^--- gap (5 slots) ---^

Type "Beautiful":
"HelloBeautiful| World"
['H','e','l','l','o','B','e','a','u','t','i','f','u','l',_,' ','W','o','r','l','d']

Move cursor right 3 positions:
"HelloBeauti|ful World"
['H','e','l','l','o','B','e','a','u','t','i','f','u','l',_,_,_,'f','u','l',' ','W','o','r','l','d']
                                           ^--- move gap right ---^
```

```rust
pub struct GapBuffer {
    buffer: Vec<char>,
    gap_start: usize,
    gap_end: usize,
}

impl GapBuffer {
    pub fn new(initial: &str) -> Self {
        let chars: Vec<char> = initial.chars().collect();
        let initial_size = chars.len();
        let mut buffer = chars;
        buffer.resize(initial_size + GAP_SIZE, '_');  // Add gap at end

        GapBuffer {
            buffer,
            gap_start: initial_size,
            gap_end: initial_size + GAP_SIZE,
        }
    }

    pub fn insert(&mut self, ch: char) {
        if self.gap_start == self.gap_end {
            self.expand_gap();
        }
        self.buffer[self.gap_start] = ch;
        self.gap_start += 1;
    }

    pub fn delete(&mut self) -> Option<char> {
        if self.gap_start < self.buffer.len() - (self.gap_end - self.gap_start) {
            let deleted = self.buffer[self.gap_end];
            self.gap_end += 1;
            Some(deleted)
        } else {
            None
        }
    }

    fn expand_gap(&mut self) {
        let gap_size = self.gap_end - self.gap_start;
        let new_size = self.buffer.len() + gap_size * 2;
        self.buffer.resize(new_size, '_');

        // Move everything after gap to end
        let content_after = self.buffer[self.gap_end..].to_vec();
        let new_gap_end = self.buffer.len() - content_after.len();

        for (i, &ch) in content_after.iter().enumerate() {
            self.buffer[new_gap_end + i] = ch;
        }

        self.gap_end = new_gap_end;
    }
}
```

**Complexity**:
- Insert at cursor: O(1)
- Delete at cursor: O(1)
- Move cursor: O(distance) - must shift gap
- Insert far from cursor: O(distance)

**Used by**: Emacs, early text editors

### Rope

A rope is a balanced binary tree where leaves store string chunks:

```
         Root (len=100)
        /            \
    Node(40)        Node(60)
    /    \           /    \
"Hello" " World"  "How "  "are you?"
(10)    (6)       (4)     (8)
```

```rust
pub struct Rope {
    root: Option<Rc<RopeNode>>,
}

enum RopeNode {
    Leaf {
        text: String,
        length: usize,  // char count
    },
    Internal {
        left: Rc<RopeNode>,
        right: Rc<RopeNode>,
        total_length: usize,  // sum of left + right
    },
}

impl Rope {
    pub fn insert(&mut self, pos: usize, text: &str) {
        // Split rope at pos
        let (left, right) = self.split(pos);

        // Create new leaf for inserted text
        let middle = Rc::new(RopeNode::Leaf {
            text: text.to_string(),
            length: text.chars().count(),
        });

        // Concatenate: left + middle + right
        self.root = Some(self.concat(left, middle));
        self.root = Some(self.concat(self.root.take().unwrap(), right));
    }

    fn split(&self, pos: usize) -> (Option<Rc<RopeNode>>, Option<Rc<RopeNode>>) {
        // Recursively split the tree
        // O(log n) with balanced tree
        todo!()
    }
}
```

**Complexity**:
- Insert: O(log n)
- Delete: O(log n)
- Char access: O(log n)
- Substring: O(log n)

**Used by**: Sublime Text, Atom, many modern editors

### Piece Table (Fresh's Choice)

A piece table uses two buffers and a list of "pieces":

```
Original Buffer (immutable after load):
"Hello World\nThis is a test\n"

Added Buffer (all insertions appended here):
"Beautiful "

Pieces (describe the current document state):
[Original: 0-6] [Added: 0-10] [Original: 5-end]

Result: "Hello " + "Beautiful " + " World\n..."
```

```rust
pub struct PieceTable {
    original: Vec<u8>,      // Original file content
    added: Vec<u8>,         // All additions
    pieces: Vec<Piece>,     // Current document structure
}

#[derive(Clone)]
pub struct Piece {
    buffer: BufferType,     // Original or Added
    start: usize,           // Offset in that buffer
    length: usize,          // Length in bytes
}

#[derive(Clone, Copy)]
pub enum BufferType {
    Original,
    Added,
}

impl PieceTable {
    pub fn insert(&mut self, pos: usize, text: &[u8]) {
        // 1. Find which piece contains pos
        let (piece_idx, offset_in_piece) = self.find_piece(pos);

        // 2. Add text to added buffer
        let added_start = self.added.len();
        self.added.extend_from_slice(text);

        // 3. Split the piece and insert new piece
        let new_piece = Piece {
            buffer: BufferType::Added,
            start: added_start,
            length: text.len(),
        };

        // Split existing piece if needed
        self.split_piece(piece_idx, offset_in_piece, new_piece);
    }
}
```

**Complexity**:
- Insert: O(n) for piece list, O(log n) with piece tree
- Delete: O(n) for piece list, O(log n) with piece tree
- Char access: O(n) for list, O(log n) with tree

**Used by**: Fresh, VS Code (variant), many Vim implementations

---

## Part 2: Fresh's Piece Tree Implementation

Fresh enhances the piece table with a **balanced tree** and **integrated line tracking**:

### Tree Structure

```rust
pub enum PieceTreeNode {
    Internal {
        left: Arc<PieceTreeNode>,
        right: Arc<PieceTreeNode>,
        left_bytes: usize,      // Total bytes in left subtree
        lf_left: Option<usize>, // Total line feeds in left subtree
    },
    Leaf {
        location: BufferLocation,  // Which buffer (Original/Added)
        offset: usize,             // Offset in that buffer
        bytes: usize,              // Length in bytes
        line_feed_cnt: Option<usize>, // Line feeds in this piece
    },
}
```

### Finding Byte Offset

```rust
impl PieceTreeNode {
    /// Find the piece containing byte offset `target`
    /// Returns (piece_info, bytes_before_that_piece)
    pub fn find_byte_offset(&self, target: usize) -> (PieceInfo, usize) {
        match self {
            PieceTreeNode::Leaf { location, offset, bytes, .. } => {
                (PieceInfo {
                    location: *location,
                    offset: *offset,
                    bytes: *bytes,
                    offset_in_piece: Some(target),  // target is within this piece
                }, 0)
            }
            PieceTreeNode::Internal { left, right, left_bytes, .. } => {
                if target < *left_bytes {
                    // Target is in left subtree
                    let (info, bytes_before) = left.find_byte_offset(target);
                    (info, bytes_before)
                } else {
                    // Target is in right subtree
                    let (mut info, bytes_before) = right.find_byte_offset(target - *left_bytes);
                    info.offset_in_piece = info.offset_in_piece.map(|o| o);
                    (info, bytes_before + *left_bytes)
                }
            }
        }
    }
}
```

### Finding Line Number

```rust
impl PieceTreeNode {
    /// Find the position of line `target_line` (0-indexed)
    /// Returns (byte_offset, column)
    pub fn find_line(&self, target_line: usize) -> (usize, usize) {
        match self {
            PieceTreeNode::Leaf { bytes, line_feed_cnt, .. } => {
                // Linear scan within this piece
                // (In practice, pieces are small)
                (0, 0)  // Simplified
            }
            PieceTreeNode::Internal { left, right, lf_left, left_bytes, .. } => {
                match lf_left {
                    None => {
                        // Line count unknown, must scan
                        (0, 0)  // Simplified
                    }
                    Some(lf_count) => {
                        if target_line < *lf_count {
                            // Target line is in left subtree
                            left.find_line(target_line)
                        } else {
                            // Target line is in right subtree
                            let (byte_offset, col) = right.find_line(target_line - *lf_count);
                            (byte_offset + *left_bytes, col)
                        }
                    }
                }
            }
        }
    }
}
```

---

## Part 3: Lazy Loading for Huge Files

Fresh handles multi-gigabyte files with **lazy loading**:

### Chunk-Based Loading

```rust
pub const LARGE_FILE_THRESHOLD: usize = 100 * 1024 * 1024;  // 100MB
pub const LOAD_CHUNK_SIZE: usize = 1024 * 1024;  // 1MB chunks
pub const CHUNK_ALIGNMENT: usize = 64 * 1024;  // 64KB alignment

pub enum BufferData {
    /// Fully loaded with line indexing
    Loaded {
        data: Vec<u8>,
        line_starts: Option<Vec<usize>>,
    },
    /// Not yet loaded - represents a file region
    Unloaded {
        file_path: PathBuf,
        file_offset: usize,
        bytes: usize,
    },
}

impl TextBuffer {
    pub fn new_for_file(path: &Path, fs: Arc<dyn FileSystem>) -> Result<Self> {
        let metadata = fs.metadata(path)?;
        let file_size = metadata.size();

        if file_size > LARGE_FILE_THRESHOLD {
            // Large file: don't load, create unloaded buffer
            Self::new_lazy(path, fs)
        } else {
            // Small file: load fully
            Self::new_eager(path, fs)
        }
    }

    /// Load a chunk of an unloaded buffer
    fn load_chunk(&mut self, byte_offset: usize, chunk_size: usize) {
        // Align to CHUNK_ALIGNMENT
        let aligned_offset = (byte_offset / CHUNK_ALIGNMENT) * CHUNK_ALIGNMENT;
        let load_size = chunk_size + (byte_offset - aligned_offset);

        // Load from file
        let data = self.fs.read_range(
            &self.file_path,
            aligned_offset as u64,
            load_size,
        )?;

        // Create new loaded buffer (no line indexing for chunks)
        let buffer = StringBuffer::new_loaded(
            self.next_buffer_id(),
            data,
            false,  // Don't compute line starts
        );

        // Update piece tree to reference this loaded chunk
        // ...
    }
}
```

### Why Not Line Index Huge Files?

For a 1GB file with average line length of 80 bytes:
- ~12.5 million lines
- Line index would be 12.5 million `usize` values = 100MB
- Computing line index requires scanning entire file = 5-10 seconds

Instead, Fresh:
1. Loads only visible chunks (1MB at a time)
2. Doesn't compute line starts for chunks
3. Scans linearly within chunks for line navigation (acceptable for small chunks)

---

## Part 4: Encoding Detection

Fresh detects and handles multiple text encodings:

```rust
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Latin1,
    ShiftJis,
    GB18030,
    EucKr,
    // ... more encodings
}

impl TextBuffer {
    pub fn detect_encoding(data: &[u8]) -> Encoding {
        // Check for BOM (Byte Order Mark)
        if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return Encoding::Utf8;
        }
        if data.starts_with(&[0xFF, 0xFE]) {
            return Encoding::Utf16Le;
        }
        if data.starts_with(&[0xFE, 0xFF]) {
            return Encoding::Utf16Be;
        }

        // Use heuristics for other encodings
        use charset_normalizer_rs::from_bytes;
        let result = from_bytes(data).best();
        match result.encoding() {
            "ASCII" | "UTF-8" => Encoding::Utf8,
            "ISO-8859-1" => Encoding::Latin1,
            "Shift_JIS" => Encoding::ShiftJis,
            "GB18030" => Encoding::GB18030,
            "EUC-KR" => Encoding::EucKr,
            _ => Encoding::Utf8,  // Default
        }
    }
}
```

### Non-Resynchronizable Encodings

Some encodings (Shift-JIS, GB18030) are **non-resynchronizable**:
- Character boundaries depend on previous bytes
- Can't jump to middle of file and decode

Fresh requires user confirmation before loading large files with these encodings:

```rust
pub struct LargeFileEncodingConfirmation {
    pub path: PathBuf,
    pub file_size: usize,
    pub encoding: Encoding,
}

// User prompt:
// "GB18030 (95 MB) requires full load. (l)oad, (e)ncoding, (C)ancel?"
```

---

## Part 5: Binary File Support

Fresh detects and handles binary files:

```rust
impl TextBuffer {
    pub fn is_binary(data: &[u8]) -> bool {
        // Check for null bytes (common in binary files)
        let null_count = data.iter().filter(|&&b| b == 0).count();
        let null_ratio = null_count as f64 / data.len() as f64;

        // More than 10% null bytes = binary
        null_ratio > 0.1
    }

    pub fn render_binary_byte(byte: u8) -> String {
        // Render unprintable bytes as hex codes
        format!("<{:02X}>", byte)
    }
}
```

Binary files are opened read-only and display bytes as hex codes.

---

## Part 6: Undo/Redo with Piece Tree Snapshots

Fresh implements undo/redo by saving piece tree snapshots:

```rust
pub struct BufferSnapshot {
    pub piece_tree: PieceTree,
    pub buffers: Vec<StringBuffer>,
    pub next_buffer_id: usize,
}

pub struct UndoHistory {
    undo_stack: Vec<BufferSnapshot>,
    redo_stack: Vec<BufferSnapshot>,
    max_history: usize,
}

impl TextBuffer {
    pub fn save_snapshot(&self) -> BufferSnapshot {
        BufferSnapshot {
            piece_tree: self.piece_tree.clone(),
            buffers: self.buffers.clone(),
            next_buffer_id: self.next_buffer_id,
        }
    }

    pub fn undo(&mut self) -> bool {
        if self.undo_stack.is_empty() {
            return false;
        }

        // Save current state for redo
        self.redo_stack.push(self.save_snapshot());

        // Restore previous state
        let snapshot = self.undo_stack.pop().unwrap();
        self.restore_snapshot(snapshot);

        true
    }

    pub fn redo(&mut self) -> bool {
        if self.redo_stack.is_empty() {
            return false;
        }

        // Save current state for undo
        self.undo_stack.push(self.save_snapshot());

        // Restore redo state
        let snapshot = self.redo_stack.pop().unwrap();
        self.restore_snapshot(snapshot);

        true
    }
}
```

### BulkEdit for Compound Operations

For operations like "replace all", Fresh uses `BulkEdit`:

```rust
pub struct BulkEdit {
    edits: Vec<Edit>,
    original_snapshot: Option<BufferSnapshot>,
}

impl BulkEdit {
    pub fn start(buffer: &TextBuffer) -> Self {
        Self {
            edits: Vec::new(),
            original_snapshot: Some(buffer.save_snapshot()),
        }
    }

    pub fn add(&mut self, edit: Edit) {
        self.edits.push(edit);
    }

    pub fn finish(self, buffer: &mut TextBuffer) {
        // Apply all edits
        for edit in self.edits {
            buffer.apply(edit);
        }

        // Single undo step for entire bulk edit
        if let Some(snapshot) = self.original_snapshot {
            buffer.undo_stack.push(snapshot);
        }
    }
}
```

---

## Part 7: Performance Comparison

| Operation | Gap Buffer | Rope | Piece Table (list) | Piece Tree (Fresh) |
|-----------|-----------|------|-------------------|-------------------|
| Insert at cursor | O(1) | O(log n) | O(n) | O(log n) |
| Delete at cursor | O(1) | O(log n) | O(n) | O(log n) |
| Insert at start | O(n) | O(log n) | O(n) | O(log n) |
| Random access | O(1) | O(log n) | O(n) | O(log n) |
| Line N access | O(n) | O(log n) | O(n) | O(log n) |
| Undo/Redo | O(n) | O(log n) | O(1) | O(1) |
| Memory overhead | Low | Medium | Low | Medium |

**Fresh's choice**: Piece Tree with integrated line tracking provides:
- O(log n) for all operations
- Efficient undo/redo (just snapshots)
- Natural multi-cursor support
- Excellent huge file handling

---

## Part 8: Implementation Tips

### 1. Always Validate Piece Boundaries

```rust
fn get_text_range(&self, start: usize, end: usize) -> Vec<u8> {
    assert!(start <= end, "Invalid range: {} > {}", start, end);
    assert!(end <= self.byte_length(), "Range {} exceeds buffer length {}", end, self.byte_length());
    // ...
}
```

### 2. Use Arc for Tree Nodes

```rust
pub enum PieceTreeNode {
    Internal {
        left: Arc<PieceTreeNode>,  // Shared ownership
        right: Arc<PieceTreeNode>,
        // ...
    },
    // ...
}
```

This allows cheap cloning of tree snapshots for undo.

### 3. Chunk Large Operations

```rust
fn replace_all(&mut self, pattern: &Regex, replacement: &str) {
    let mut bulk_edit = BulkEdit::start(self);
    let mut count = 0;

    for m in pattern.find_iter(self.get_all_text()) {
        bulk_edit.add(Edit::Delete(m.start(), m.end()));
        bulk_edit.add(Edit::Insert(m.start(), replacement.to_string()));
        count += 1;

        // Yield every 1000 edits to keep UI responsive
        if count % 1000 == 0 {
            self.process_events();
        }
    }

    bulk_edit.finish(self);
}
```

### 4. Align Chunk Loads

```rust
// Align to 64KB boundaries for better I/O performance
let aligned_offset = (byte_offset / CHUNK_ALIGNMENT) * CHUNK_ALIGNMENT;
```

---

## Resources

- [Piece Table Implementation](https://www.joebergeron.io/posts/post_four.html) - Excellent detailed guide
- [Rope Science](https://www.cs.tufts.edu/courses/CS164/2002/papers/ropes.pdf) - Original paper
- [Fresh Source: model/buffer.rs](/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/fresh/crates/fresh-editor/src/model/buffer.rs) - Production implementation
- [Fresh Source: model/piece_tree.rs](/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/fresh/crates/fresh-editor/src/model/piece_tree.rs) - Tree implementation
