---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/marker/
language: Python
version: 1.10.2
explored_at: 2026-07-04
license: GPL-3.0-or-later (code), OpenRAIL-M (models)
---

# spec.md — Marker Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/marker/`
- **Language:** Python 3.10+
- **Version:** 1.10.2
- **LOC:** 13,599 Python lines (209 files)
- **License:** GPL-3.0-or-later (code), OpenRAIL-M (models)
- **Repository:** github.com/VikParuchuri/marker

## What Marker Is

Marker converts PDFs, images, PPTX, DOCX, XLSX, HTML, and EPUB files to markdown, JSON, HTML, and structured chunks with high accuracy. It uses Surya OCR models for layout detection, text recognition, and table extraction. Optional LLM integration (Gemini, Ollama) boosts accuracy for complex documents.

## Architecture Overview

Marker follows a pipeline architecture:

1. **Provider** → Input document (PDF, image, DOCX, etc.)
2. **Builders** → Construct document structure
3. **Processors** → Process specific block types
4. **Renderer** → Output format (markdown, JSON, HTML)

## Documentation Structure

```
src.PDFToMarkdown/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-providers.md
│   ├── 03-builders.md
│   ├── 04-processors.md
│   ├── 05-renderers.md
│   ├── 06-models.md
│   ├── 07-llm-integration.md
│   └── 08-cli-api.md
└── html/
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-architecture.md | TODO |
| 3 | 02-providers.md | TODO |
| 4 | 03-builders.md | TODO |
| 5 | 04-processors.md | TODO |
| 6 | 05-renderers.md | TODO |
| 7 | 06-models.md | TODO |
| 8 | 07-llm-integration.md | TODO |
| 9 | 08-cli-api.md | TODO |
| 10 | HTML generation | TODO |
| 11 | Grandfather review | TODO |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.PDFToMarkdown
python3 build.py .
```