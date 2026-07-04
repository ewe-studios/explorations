# fasthtml Documentation

Python web framework with HTMX for minimal, fast web apps.

## Documents

| # | Document | Description |
|---|----------|-------------|
| 0 | [Overview](00-overview.md) | Architecture, HTMX integration, module structure |

## Source

`fasthtml/` — 33 Python files

## Usage

```python
from fasthtml.common import *

app,rt = fast_app()

@rt('/')
def get(): return Div(P('Hello World!'), hx_get="/change")

serve()
```

## Docs

https://docs.fastht.ml
