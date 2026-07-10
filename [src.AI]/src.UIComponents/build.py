#!/usr/bin/env python3
"""Build all UIComponents subproject docs and generate central index.

Usage:
    python3 build.py              # build all subprojects + central index
    python3 build.py <subproject> # build one subproject only
"""

import os
import re
import sys
import subprocess
from pathlib import Path

# ── Subprojects ──────────────────────────────────────────────────────────────

SUBPROJECTS = [
    "kumo",
    "mapcn",
    "next-maps",
    "performativeUI",
    "remix",
    "remote-dom",
    "src.UIComponents",
    "steal-a-gif",
    "table-forge",
    "universal-timer",
    "use-stick-to-bottom",
]

# ── HTML template for central index ──────────────────────────────────────────

INDEX_TEMPLATE = """\
<!DOCTYPE html>
<html lang="en" data-theme="light">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>src.UIComponents -- Documentation</title>
  <link rel="stylesheet" href="styles.css">
  <script>
    (function() {{
      var s = localStorage.getItem('theme');
      var p = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      document.documentElement.setAttribute('data-theme', s || p);
    }})();
  </script>
</head>
<body>
  <nav class="nav">
    <span class="nav-brand">~/src.UIComponents/docs</span>
    <div class="nav-actions">
      <button class="theme-toggle" onclick="toggleTheme()" title="Toggle dark/light theme">theme</button>
    </div>
  </nav>

  <h1>src.UIComponents Documentation</h1>
  <p>11 UI component projects: component libraries, map components, table builders, remote DOM, GIF tools, timers, and more. {total_docs} documents across {total_words} words.</p>

  <h2>Projects</h2>
  <ul class="doc-index">
    {links}
  </ul>

  <hr>
  <p style="color: var(--fg-soft); font-size: 0.85rem;">
    Generated from markdown. Each project has its own build.py and html/ output.
  </p>
  <script>function toggleTheme(){{var c=document.documentElement.getAttribute('data-theme'),n=c==='dark'?'light':'dark';document.documentElement.setAttribute('data-theme',n);localStorage.setItem('theme',n);}}</script>
</body>
</html>
"""

SHARED_CSS = """\
:root { --bg: #fff; --bg-strong: #f5f5f5; --bg-muted: #eee; --fg: #1a1a1a; --fg-soft: #666; --line-strong: #ddd; --accent: #2563eb; --accent-hover: #1d4ed8; }
[data-theme="dark"] { --bg: #0d1117; --bg-strong: #161b22; --bg-muted: #21262d; --fg: #e6edf3; --fg-soft: #8b949e; --line-strong: #30363d; --accent: #58a6ff; --accent-hover: #79c0ff; }
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: var(--bg); color: var(--fg); line-height: 1.6; padding: 1rem; max-width: 900px; margin: 0 auto; }
.nav { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 1.5rem; padding-bottom: 1rem; border-bottom: 1px solid var(--line-strong); }
.nav-brand { font-weight: 600; font-size: 1.1rem; }
.nav-actions { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
.nav-btn, .nav-breadcrumb { padding: 0.3rem 0.7rem; border: 1px solid var(--line-strong); border-radius: 9999px; text-decoration: none; color: var(--fg); font-size: 0.85rem; background: var(--bg-strong); }
.nav-btn:hover { background: var(--bg-muted); }
.nav-btn-next { border-color: var(--accent) !important; color: var(--accent); }
.nav-btn-next:hover { background: var(--accent); color: #fff !important; }
.theme-toggle { padding: 0.3rem 0.7rem; border: 1px solid var(--line-strong); border-radius: 9999px; background: var(--bg-strong); color: var(--fg); cursor: pointer; font-size: 0.85rem; }
.prose { line-height: 1.7; }
.prose h1 { font-size: 2rem; margin: 1.5rem 0 1rem; }
.prose h2 { font-size: 1.5rem; margin: 1.5rem 0 0.75rem; }
.prose h3 { font-size: 1.25rem; margin: 1.25rem 0 0.5rem; }
.prose p { margin: 0.75rem 0; }
.prose pre { background: var(--bg-strong); padding: 1rem; border-radius: 0.5rem; overflow-x: auto; font-size: 0.85rem; }
.prose code { background: var(--bg-strong); padding: 0.15rem 0.35rem; border-radius: 0.25rem; font-size: 0.85rem; }
.prose pre code { background: none; padding: 0; }
.prose table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
.prose th, .prose td { border: 1px solid var(--line-strong); padding: 0.5rem; text-align: left; }
.prose th { background: var(--bg-strong); }
.prose blockquote { border-left: 3px solid var(--accent); padding-left: 1rem; color: var(--fg-soft); margin: 1rem 0; }
.prose ul, .prose ol { margin: 0.5rem 0 0.5rem 1.5rem; }
.mermaid { background: var(--bg-strong); padding: 1rem; border-radius: 0.5rem; text-align: center; cursor: pointer; }
.doc-index li { margin: 0.5rem 0; }
.doc-index a { text-decoration: none; color: var(--accent); }
.doc-index a:hover { text-decoration: underline; }
.diagram-modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.8); z-index: 1000; display: flex; align-items: center; justify-content: center; }
.diagram-modal-container { position: relative; max-width: 95vw; max-height: 90vh; overflow: auto; background: var(--bg); border-radius: 0.5rem; padding: 1rem; }
.diagram-modal-toolbar { position: sticky; top: 0; display: flex; gap: 0.5rem; justify-content: center; padding: 0.5rem; background: var(--bg); border-radius: 0.5rem; z-index: 1; }
.diagram-modal-btn { padding: 0.3rem 0.7rem; border: 1px solid var(--line-strong); border-radius: 0.25rem; background: var(--bg-strong); color: var(--fg); cursor: pointer; }
.diagram-modal-hint { position: fixed; bottom: 1rem; left: 50%; transform: translateX(-50%); background: var(--bg-strong); color: var(--fg-soft); padding: 0.3rem 0.7rem; border-radius: 0.25rem; font-size: 0.8rem; }
@media (max-width: 768px) { body { padding: 0.5rem; } .nav { flex-direction: column; align-items: flex-start; } }
"""


def get_project_info(sub: str, base: Path) -> dict:
    """Extract title, doc count, and word count from a subproject."""
    md_dir = base / sub / "markdown"
    docs = 0
    words = 0
    title = sub
    if md_dir.exists():
        for md_file in sorted(md_dir.glob("*.md")):
            docs += 1
            try:
                content = md_file.read_text()
                words += len(content.split())
                if md_file.name == "README.md":
                    # Try frontmatter title
                    fm = re.match(r'^---\s*\n(.*?)\n---\s*\n', content, re.DOTALL)
                    if fm:
                        title_m = re.search(r'^title:\s*"([^"]+)"', fm.group(1), re.MULTILINE)
                        if title_m:
                            title = title_m.group(1)
                            continue
                    # Try first heading
                    h = re.search(r'^#\s+(.+)', content, re.MULTILINE)
                    if h:
                        title = h.group(1).strip()
            except Exception:
                pass
    return {"sub": sub, "title": title, "docs": docs, "words": words}


def build_subproject(sub: str, base: Path):
    """Build a single subproject using its own build.py."""
    sub_dir = base / sub
    build_py = sub_dir / "build.py"
    if build_py.exists():
        print(f"  Building {sub}...")
        result = subprocess.run(
            [sys.executable, str(build_py), str(sub_dir)],
            capture_output=True, text=True
        )
        if result.returncode == 0:
            print(f"    ✓ {sub} built")
        else:
            print(f"    ✗ {sub} failed: {result.stderr[:200]}")
    else:
        print(f"  ⚠ {sub}: no build.py found")


def build_central_index(base: Path):
    """Generate the central html/index.html linking to all subprojects."""
    html_dir = base / "html"
    html_dir.mkdir(parents=True, exist_ok=True)

    # Gather info for all subprojects
    infos = [get_project_info(sub, base) for sub in SUBPROJECTS]
    total_docs = sum(i["docs"] for i in infos)
    total_words = sum(i["words"] for i in infos)

    # Build links
    links_html = ""
    for info in infos:
        sub = info["sub"]
        title = info["title"]
        docs = info["docs"]
        words = info["words"]
        sub_html = f"{sub}/html/index.html"
        links_html += (
            f'<li><a href="{sub_html}">{title}</a>'
            f' <span style="color:var(--fg-soft);font-size:0.8rem;">({docs} docs, {words:,} words)</span>'
            f'</li>\n'
        )

    index_html = INDEX_TEMPLATE.format(
        total_docs=total_docs,
        total_words=f"{total_words:,}",
        links=links_html,
    )

    with open(html_dir / "index.html", "w") as f:
        f.write(index_html)
    print(f"  ✓ html/index.html ({total_docs} projects, {total_words:,} words)")

    # Write shared CSS
    css_file = html_dir / "styles.css"
    if not css_file.exists():
        css_file.write_text(SHARED_CSS)
        print(f"  ✓ html/styles.css")


def build_one(sub: str, base: Path):
    """Build a single subproject."""
    if sub not in SUBPROJECTS:
        print(f"Unknown subproject: {sub}")
        print(f"Available: {', '.join(SUBPROJECTS)}")
        sys.exit(1)
    build_subproject(sub, base)
    build_central_index(base)


def build_all(base: Path):
    """Build all subprojects + central index."""
    print("Building all UIComponents subproject docs...\n")
    for sub in SUBPROJECTS:
        build_subproject(sub, base)
    print()
    print("Generating central index...")
    build_central_index(base)
    print("\nDone. Open html/index.html in a browser.")


if __name__ == "__main__":
    base = Path(__file__).resolve().parent
    if len(sys.argv) > 1:
        build_one(sys.argv[1], base)
    else:
        build_all(base)
