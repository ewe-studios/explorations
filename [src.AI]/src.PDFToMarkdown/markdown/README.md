# Marker — PDF to Markdown Converter

Marker converts PDFs, images, PPTX, DOCX, XLSX, HTML, and EPUB files to structured markdown, JSON, HTML, and chunked output with high accuracy. Powered by 5 Surya OCR models and an optional LLM layer (Gemini, Claude, OpenAI, Ollama) for handling complex regions, equations, and handwriting.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/marker/` — 209 Python files, 17,862 LOC

## Documents

### Foundation

| # | Document | What You'll Learn |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | Project philosophy, pipeline at a glance, 5 Surya models, features |
| 01 | [Architecture](01-architecture.md) | Full dependency graph, block types, Document model, resolve_dependencies |

### Deep Dives

| # | Document | What You'll Learn |
|---|----------|-------------------|
| 02 | [Providers](02-providers.md) | 8 input types, file-type detection, PdfProvider internals, bad OCR detection |
| 03 | [Builders](03-builders.md) | DocumentBuilder pipeline, LayoutBuilder with Surya, LineBuilder text merging, OcrBuilder Surya OCR, StructureBuilder grouping |
| 04 | [Processors](04-processors.md) | All 27 processors, ordering, the processor pipeline, heuristic vs LLM processors |
| 05 | [Renderers](05-renderers.md) | Markdown via markdownify, HTML renderer, JSON output, chunk renderer, extraction format |

### Cross-Cutting

| # | Document | What You'll Learn |
|---|----------|-------------------|
| 06 | [LLM Integration](06-llm-integration.md) | 6 LLM services, BaseLLMProcessor, prompt construction, concurrent inference, response parsing |
| 07 | [CLI & API](07-cli-api.md) | CLI tools, ConfigParser, Python API, settings, output formats |

## Quick Start

```bash
# CLI — convert a single PDF
marker_single input.pdf --output_format markdown

# CLI — batch convert
marker /path/to/pdfs --output_format markdown

# Python API
from marker.converters.pdf import PdfConverter
from marker.models import create_model_dict

models = create_model_dict()
converter = PdfConverter(artifact_dict=models)
rendered = converter("input.pdf")
```

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.PDFToMarkdown
python3 build.py .
```
