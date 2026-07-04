---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/marker/
language: Python
version: 1.10.2
explored_at: 2026-07-04
license: GPL-3.0-or-later (code), OpenRAIL-M (models)
repository: github.com/VikParuchuri/marker
---

# spec.md — Marker PDF-to-Markdown Converter

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/marker/`
- **Language:** Python 3.10+
- **Version:** 1.10.2
- **LOC:** 17,862 Python lines (209 files)
- **License:** GPL-3.0-or-later (code), OpenRAIL-M (Surya models)
- **Repository:** github.com/VikParuchuri/marker

## What Marker Is

Marker is a high-accuracy document conversion pipeline that transforms PDFs, images, PPTX, DOCX, XLSX, HTML, and EPUB files into structured markdown, JSON, HTML, and chunked output. It combines Surya OCR models (layout detection, text recognition, table extraction) with an optional LLM layer (Gemini, Claude, OpenAI, Ollama) for handling complex regions, equations, handwriting, and form recognition. The pipeline processes documents through four stages: Input (8 provider types) → Build (4 builders constructing the document graph) → Process (27 processors cleaning and enhancing) → Render (6 output formats).

## Architecture Overview

Marker follows a pipeline architecture centered on a `Document` object graph:

1. **Provider** — Reads the input file and extracts raw text/spans/lines. Each file type (PDF, DOCX, image, etc.) has its own provider that normalizes to a common `ProviderOutput` format.
2. **Builders** — Construct the `Document` object: `DocumentBuilder` creates pages, `LayoutBuilder` runs Surya layout detection, `LineBuilder` assigns text lines into layout blocks, `OcrBuilder` runs Surya OCR on blocks that need it, `StructureBuilder` assembles the final page structure.
3. **Processors** — 27 post-processors transform the document: equation detection, table formatting, list grouping, footnote linking, code block detection, section header identification, and optional LLM-based enhancements.
4. **Renderer** — Converts the `Document` object graph to the output format (markdown via `markdownify`, clean HTML, structured JSON, or chunked text).

## Documentation Structure

```
src.PDFToMarkdown/
├── spec.md
├── build.py
├── markdown/
│   ├── README.md
│   ├── 00-overview.md              ← What Marker is, philosophy, features, pipeline at a glance
│   ├── 01-architecture.md          ← Full dependency graph, Document model, block types, layers
│   ├── 02-providers.md             ← All 8 input providers with file-type detection
│   ├── 03-builders.md              ← 4 builders: Document, Layout, Line, OCR, Structure
│   ├── 04-processors.md            ← 27 processors: heuristic + LLM, ordered pipeline
│   ├── 05-renderers.md             ← 6 renderers: Markdown, HTML, JSON, Chunks, OCR JSON, Extraction
│   ├── 06-llm-integration.md       ← LLM services (Gemini, Claude, OpenAI, Ollama, Vertex), BaseLLMProcessor
│   └── 07-cli-api.md               ← CLI tools, ConfigParser, Python API, settings
└── html/
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-architecture.md | DONE |
| 3 | 02-providers.md | DONE |
| 4 | 03-builders.md | DONE |
| 5 | 04-processors.md | DONE |
| 6 | 05-renderers.md | DONE |
| 7 | 06-llm-integration.md | DONE |
| 8 | 07-cli-api.md | DONE |
| 9 | HTML generation | DONE |
| 10 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.PDFToMarkdown
python3 build.py .
```

**Script:** `build.py` (shared pattern)
**Dependencies:** None (Python 3.12+ stdlib only)

## Quality Requirements

All documents must meet:
- Detailed sections with code snippets from actual source (minimum 10 per deep-dive doc)
- At least 2 mermaid diagrams per document
- At least 1 Aha moment per document
- Links to at least 2 related documents
- Source path references for every claim
- First paragraph states the core concept clearly
- Grandfather review: names, numbers, flows, coverage all verified against source

## Expected Outcome

After reading these documents, a developer should be able to:
- Understand the full conversion pipeline end to end
- Configure Marker for different input types and output formats
- Extend Marker with custom providers, processors, or renderers
- Integrate LLM services for enhanced accuracy
- Debug and optimize the pipeline for specific document types
- Use both the CLI and Python API effectively

## Resume Point

All documents are written in order (00 → 01 → 02 → ... → 07). Build HTML with `python3 build.py .`, then run grandfather review against source code at `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/marker/`.
