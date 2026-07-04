---
title: Marker CLI & API — Command-Line Tools and Python Interface
---

# Marker CLI & API — Command-Line Tools and Python Interface

**Marker provides six CLI entry points and a Python API centered on the `PdfConverter` class.** The CLI tools range from single-file conversion to batch processing, streaming extraction, and a Streamlit web app. The Python API gives full programmatic control over every pipeline stage.

Source: `marker/scripts/` — 8 files, root `convert.py`, `convert_single.py`, `chunk_convert.py`, `marker_app.py`, `marker_server.py`, `extraction_app.py`

## CLI Tools

| Command | Script | Purpose |
|---------|--------|---------|
| `marker_single` | `scripts/convert_single.py` | Convert one file to markdown/JSON/HTML |
| `marker` | `scripts/convert.py` | Batch convert multiple files |
| `marker_chunk_convert` | `scripts/chunk_convert.py` | Convert to chunked RAG format |
| `marker_server` | `scripts/server.py` | HTTP server for remote conversion |
| `marker_app` | `scripts/streamlit_app.py` | Streamlit web UI |
| `marker_extraction` | `scripts/extraction_app.py` | Structured extraction mode |

### marker_single — Single File Conversion

```bash
# Basic usage
marker_single input.pdf --output_format markdown

# Full options
marker_single input.pdf \
    --output_dir ./output \
    --output_format json \
    --page_range 0,5-10,20 \
    --use_llm \
    --llm_service marker.services.gemini.GoogleGeminiService \
    --config_json config.json \
    --debug
```

Source: `marker/scripts/convert_single.py:22-43`

```python
@click.command(cls=CustomClickPrinter)
@click.argument("fpath", type=str)
@ConfigParser.common_options
def convert_single_cli(fpath, **kwargs):
    models = create_model_dict()
    config_parser = ConfigParser(kwargs)

    converter_cls = config_parser.get_converter_cls()
    converter = converter_cls(
        config=config_parser.generate_config_dict(),
        artifact_dict=models,
        processor_list=config_parser.get_processors(),
        renderer=config_parser.get_renderer(),
        llm_service=config_parser.get_llm_service(),
    )
    rendered = converter(fpath)
    out_folder = config_parser.get_output_folder(fpath)
    save_output(rendered, out_folder, config_parser.get_base_filename(fpath))
```

### ConfigParser — CLI → Config Bridge

`ConfigParser` (`marker/config/parser.py`, 177 lines) translates CLI options to the config dict consumed by builders, processors, and renderers:

```python
# marker/config/parser.py:86-115
def generate_config_dict(self) -> Dict[str, any]:
    config = {}
    for k, v in self.cli_options.items():
        match k:
            case "debug":
                config["debug_pdf_images"] = True
                config["debug_layout_images"] = True
                config["debug_json"] = True
                config["debug_data_folder"] = output_dir
            case "page_range":
                config["page_range"] = parse_range_str(v)
            case "config_json":
                with open(v, "r", encoding="utf-8") as f:
                    config.update(json.load(f))
            case "disable_multiprocessing":
                config["pdftext_workers"] = 1
            case "disable_image_extraction":
                config["extract_images"] = False
            case _:
                config[k] = v
    return config
```

**Aha:** The `config_json` option allows overriding ANY builder/processor/renderer field via a JSON file. Since `assign_config()` maps dict keys to Pydantic model fields, you can tune any internal parameter without touching code:

```json
{
    "layout_coverage_threshold": 0.3,
    "block_mode_max_lines": 20,
    "max_concurrency": 5,
    "gemini_model_name": "gemini-2.5-pro"
}
```

### Common CLI Options

```python
# marker/config/parser.py:24-84
@staticmethod
def common_options(fn):
    fn = click.option("--output_dir", type=click.Path(), default=settings.OUTPUT_DIR)(fn)
    fn = click.option("--debug", "-d", is_flag=True)(fn)
    fn = click.option("--output_format", type=click.Choice(["markdown", "json", "html", "chunks"]),
                      default="markdown")(fn)
    fn = click.option("--processors", type=str, default=None,
                      help="Comma separated list of processors.")(fn)
    fn = click.option("--config_json", type=str, default=None)(fn)
    fn = click.option("--disable_multiprocessing", is_flag=True)(fn)
    fn = click.option("--disable_image_extraction", is_flag=True)(fn)
    fn = click.option("--page_range", type=str, default=None)(fn)
    fn = click.option("--converter_cls", type=str, default=None)(fn)
    fn = click.option("--llm_service", type=str, default=None)(fn)
    return fn
```

### Renderer Selection

```python
# marker/config/parser.py:127-139
def get_renderer(self):
    match self.cli_options["output_format"]:
        case "json":
            r = JSONRenderer
        case "markdown":
            r = MarkdownRenderer
        case "html":
            r = HTMLRenderer
        case "chunks":
            r = ChunkRenderer
    return classes_to_strings([r])[0]
```

### Output Directory Convention

```python
# marker/config/parser.py:167-176
def get_output_folder(self, filepath):
    output_dir = self.cli_options.get("output_dir", settings.OUTPUT_DIR)
    fname_base = os.path.splitext(os.path.basename(filepath))[0]
    output_dir = os.path.join(output_dir, fname_base)
    os.makedirs(output_dir, exist_ok=True)
    return output_dir
```

Each input file gets its own subdirectory: `output_dir/filename/` containing `filename.md`, `filename_meta.json`, and extracted images.

### Batch Conversion

```bash
marker /path/to/pdfs/ --output_format markdown --workers 4
```

The batch converter uses multiprocessing to parallelize across input files.

### marker_server — HTTP API

```bash
marker_server --port 8000
```

Provides a REST API:
- `POST /convert` — Upload file, get markdown/JSON back
- Supports streaming responses for large files

### marker_app — Streamlit UI

```bash
marker_app
```

Launches a Streamlit web app with file upload, format selection, and live preview.

## Python API

### Basic Usage

```python
from marker.converters.pdf import PdfConverter
from marker.models import create_model_dict

# Create models (downloads on first run)
models = create_model_dict()

# Create converter
converter = PdfConverter(artifact_dict=models)

# Convert a file
rendered = converter("input.pdf")

# Access output
print(rendered.markdown)     # Markdown text
print(rendered.images)       # Dict of extracted images
print(rendered.metadata)     # Document metadata
```

### Advanced: Custom Processors

```python
from marker.converters.pdf import PdfConverter
from marker.models import create_model_dict

models = create_model_dict()

converter = PdfConverter(
    artifact_dict=models,
    processor_list=[
        "marker.processors.order.OrderProcessor",
        "marker.processors.table.TableProcessor",
        "marker.processors.text.TextProcessor",
        # ... only the processors you need
    ],
    renderer="marker.renderers.json.JSONRenderer",
)

rendered = converter("input.pdf")
```

### Advanced: LLM-Enhanced Conversion

```python
converter = PdfConverter(
    artifact_dict=models,
    config={"use_llm": True},
    llm_service="marker.services.gemini.GoogleGeminiService",
)

# OR: inject config via dict
converter = PdfConverter(
    artifact_dict=models,
    config={
        "use_llm": True,
        "gemini_api_key": "...",
        "gemini_model_name": "gemini-2.5-pro",
    },
)
```

### Advanced: Configuration Injection

```python
converter = PdfConverter(
    artifact_dict=models,
    config={
        # Provider options
        "page_range": [0, 1, 2],
        "force_ocr": True,
        "strip_existing_ocr": True,
        "keep_chars": True,

        # Builder options
        "disable_ocr": False,
        "lowres_image_dpi": 96,
        "highres_image_dpi": 192,

        # Processor options
        "disable_tqdm": True,
        "table_rec_batch_size": 8,

        # Renderer options
        "extract_images": True,
        "paginate_output": False,
        "inline_math_delimiters": ["\\(", "\\)"],
        "block_math_delimiters": ["\\[", "\\]"],
    },
)
```

### Advanced: Custom Block Overrides

```python
from marker.schema import BlockTypes

class CustomTextBlock(Text):
    # Override how text blocks render
    def assemble_html(self, document, child_blocks, ...):
        return f'<div class="custom-text">{super().assemble_html(...)}</div>'

converter = PdfConverter(
    artifact_dict=models,
    override_map={BlockTypes.Text: CustomTextBlock},
)
```

The `override_map` lets you swap any block type's rendering class without modifying Marker's source.

## Settings System

Global settings via `marker/settings.py`:

```python
class Settings(BaseSettings):
    # Paths
    OUTPUT_DIR: str = "./conversion_results"
    FONT_DIR: str = "./static/fonts"
    FONT_NAME: str = "GoNotoCurrent-Regular.ttf"

    # Output
    OUTPUT_ENCODING: str = "utf-8"
    OUTPUT_IMAGE_FORMAT: str = "JPEG"

    # LLM
    GOOGLE_API_KEY: Optional[str] = ""

    # Device
    TORCH_DEVICE: Optional[str] = None  # "cuda", "mps", or None (auto)

    @property
    def TORCH_DEVICE_MODEL(self) -> str:
        # Auto-detect: CUDA > MPS > CPU

    @property
    def MODEL_DTYPE(self) -> torch.dtype:
        # bfloat16 on CUDA, float32 otherwise

    class Config:
        env_file = find_dotenv("local.env")
        extra = "ignore"
```

Environment variables override defaults — set `TORCH_DEVICE=cuda` or `GOOGLE_API_KEY=...` in a `local.env` file.

## Output Format Comparison

| Feature | Markdown | JSON | HTML | Chunks |
|---------|----------|------|------|--------|
| Human-readable | ✓ | Partial | ✓ | Partial |
| Structure preserved | Partial | ✓ | ✓ | ✓ |
| LaTeX math | ✓ ($/$$) | ✓ (raw) | ✓ (MathJax) | — |
| Extracted images | ✓ (files) | — | ✓ (base64) | — |
| Section hierarchy | ✓ (ATX) | ✓ (nested) | ✓ (h1-h6) | ✓ (per chunk) |
| Polygon positions | — | ✓ | — | — |
| Suitable for RAG | ✓ | ✓ | — | ✓ (designed for) |

## Cross-References

- [Overview](00-overview.md) — Quick-start usage
- [Architecture](01-architecture.md) — The resolve_dependencies system
- [Renderers](05-renderers.md) — Output format details
- [Builders](03-builders.md) — Builder configuration options

## Next

→ Back to [Overview](00-overview.md) for the big picture, or explore individual deep dives for specific subsystems.
