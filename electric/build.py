#!/usr/bin/env python3
"""Convert the nested Electric exploration Markdown tree to HTML.

Usage:
    python3 -I build.py [PROJECT_DIR]

The script reads every ``markdown/**/*.md`` file and writes a matching
``html/**/*.html`` file. Each directory gets its own ``index.html``.
Navigation (index, previous, next) stays inside the current directory.
Only the Python standard library is used.
"""

import os
import re
import sys
from html import escape
from pathlib import Path

HTML_TEMPLATE = """\
<!DOCTYPE html>
<html lang="en" data-theme="light">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <link rel="stylesheet" href="{prefix}styles.css">
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css">
  <script>
    (function() {{
      var s = localStorage.getItem('theme');
      var p = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      document.documentElement.setAttribute('data-theme', s || p);
    }})();
  </script>
  <script type="module">
    (async function() {{
      var blocks = document.querySelectorAll('.mermaid');
      if (blocks.length === 0) return;
      var {{ default: m }} = await import('https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs');
      function cfg() {{
        var d = document.documentElement;
        var dark = d.getAttribute('data-theme') === 'dark';
        var s = getComputedStyle(d);
        var bg = s.getPropertyValue('--bg-strong').trim();
        var fg = s.getPropertyValue('--fg').trim();
        var line = s.getPropertyValue('--line-strong').trim();
        var muted = s.getPropertyValue('--bg-muted').trim();
        return {{ startOnLoad: false, securityLevel: 'strict', htmlLabels: false, theme: 'base', flowchart: {{ curve: 'linear' }}, themeVariables: {{
          primaryColor: bg, primaryTextColor: fg, primaryBorderColor: line,
          lineColor: line, secondaryColor: muted, tertiaryColor: s.getPropertyValue('--bg').trim(),
          fontFamily: "'JetBrains Mono', monospace" }} }};
      }}
      blocks.forEach(function(b) {{ b.setAttribute('data-src', b.textContent); }});
      async function render() {{
        m.initialize(cfg());
        blocks.forEach(function(b) {{ b.removeAttribute('data-processed'); b.innerHTML = b.getAttribute('data-src'); }});
        await m.run({{ nodes: blocks }});
      }}
      await render();
      new MutationObserver(render).observe(document.documentElement, {{ attributes: true, attributeFilter: ['data-theme'] }});
      var overlay = null, container = null, svg = null;
      var scale = 1, panX = 0, panY = 0, dragging = false, startX, startY;
      function openModal(src) {{
        if (overlay) return;
        overlay = document.createElement('div');
        overlay.className = 'diagram-modal-overlay';
        overlay.setAttribute('role', 'dialog');
        container = document.createElement('div');
        container.className = 'diagram-modal-container';
        svg = src.cloneNode(true);
        container.appendChild(svg);
        var toolbar = document.createElement('div');
        toolbar.className = 'diagram-modal-toolbar';
        toolbar.appendChild(mkBtn('+', function(e) {{ e.stopPropagation(); zoom(1.3); }}));
        toolbar.appendChild(mkBtn('−', function(e) {{ e.stopPropagation(); zoom(0.7); }}));
        toolbar.appendChild(mkBtn('fit', function(e) {{ e.stopPropagation(); resetZoom(); }}));
        toolbar.appendChild(mkBtn('✕', function(e) {{ e.stopPropagation(); closeModal(); }}));
        container.appendChild(toolbar);
        var hint = document.createElement('div');
        hint.className = 'diagram-modal-hint';
        hint.textContent = 'Scroll to zoom · Drag to pan · Esc to close';
        document.body.appendChild(overlay);
        overlay.appendChild(container);
        document.body.appendChild(hint);
        scale = 1; panX = 0; panY = 0;
        container.addEventListener('mousedown', onDragStart);
        container.addEventListener('wheel', onWheel, {{ passive: false }});
        overlay.addEventListener('mousedown', onOverlayClick);
        document.addEventListener('keydown', onKey);
        setTimeout(function() {{
          var fit = Math.min(window.innerWidth * 0.9 / svg.clientWidth, window.innerHeight * 0.85 / svg.clientHeight, 1);
          scale = fit; applyTransform();
        }}, 50);
      }}
      function mkBtn(label, handler) {{
        var b = document.createElement('button');
        b.className = 'diagram-modal-btn'; b.textContent = label;
        b.addEventListener('click', handler); return b;
      }}
      function zoom(f) {{ scale *= f; applyTransform(); }}
      function resetZoom() {{
        var fit = Math.min(window.innerWidth * 0.9 / svg.clientWidth, window.innerHeight * 0.85 / svg.clientHeight, 1);
        scale = fit; panX = 0; panY = 0; applyTransform();
      }}
      function applyTransform() {{ svg.style.transform = 'translate(' + panX + 'px,' + panY + 'px) scale(' + scale + ')'; }}
      function onWheel(e) {{ e.preventDefault(); zoom(e.deltaY < 0 ? 1.15 : 0.87); }}
      function onDragStart(e) {{
        if (e.target.tagName === 'BUTTON') return;
        dragging = true; startX = e.clientX - panX; startY = e.clientY - panY;
        container.classList.add('dragging');
        document.addEventListener('mousemove', onDragMove);
        document.addEventListener('mouseup', onDragEnd);
      }}
      function onDragMove(e) {{ panX = e.clientX - startX; panY = e.clientY - startY; applyTransform(); }}
      function onDragEnd() {{ dragging = false; container.classList.remove('dragging');
        document.removeEventListener('mousemove', onDragMove);
        document.removeEventListener('mouseup', onDragEnd);
      }}
      function onOverlayClick(e) {{ if (e.target === overlay) closeModal(); }}
      function onKey(e) {{ if (e.key === 'Escape') closeModal(); }}
      function closeModal() {{
        if (!overlay) return;
        overlay.remove(); overlay = null; container = null; svg = null;
        document.removeEventListener('keydown', onKey);
        document.removeEventListener('mousemove', onDragMove);
        document.removeEventListener('mouseup', onDragEnd);
      }}
      setTimeout(function() {{
        document.querySelectorAll('.mermaid').forEach(function(el) {{
          el.addEventListener('click', function() {{
            var rendered = el.querySelector('svg');
            if (rendered) openModal(rendered);
          }});
        }});
      }}, 500);
    }})();
  </script>
</head>
<body>
  <nav class="nav">
    <span class="nav-brand">~/{project}/docs</span>
    <div class="nav-actions">
      {breadcrumbs}
      <a href="index.html" class="nav-btn" title="Back to index">index</a>
      {prev_btn}
      {next_btn}
      <button class="theme-toggle" onclick="toggleTheme()" title="Toggle dark/light theme">theme</button>
    </div>
  </nav>
  <article class="prose">
{content}
  </article>
  <script>function toggleTheme(){{var c=document.documentElement.getAttribute('data-theme'),n=c==='dark'?'light':'dark';document.documentElement.setAttribute('data-theme',n);localStorage.setItem('theme',n);}}</script>
</body>
</html>
"""

INDEX_TEMPLATE = """\
<!DOCTYPE html>
<html lang="en" data-theme="light">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <link rel="stylesheet" href="{prefix}styles.css">
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
    <span class="nav-brand">~/{project}/docs</span>
    <div class="nav-actions">
      {up_link}
      <button class="theme-toggle" onclick="toggleTheme()" title="Toggle dark/light theme">theme</button>
    </div>
  </nav>
  <article class="prose">
  <h1>{heading}</h1>
  <p>{description}</p>
  {sections}
  <hr>
  <p style="color: var(--fg-soft); font-size: 0.85rem;">
    Generated from markdown. Mermaid diagrams require JavaScript. Dark mode supported.
  </p>
  </article>
  <script>function toggleTheme(){{var c=document.documentElement.getAttribute('data-theme'),n=c==='dark'?'light':'dark';document.documentElement.setAttribute('data-theme',n);localStorage.setItem('theme',n);}}</script>
</body>
</html>
"""


class Md2Html:
    """Minimal Markdown to HTML converter for the subset these docs use."""

    def convert(self, text: str) -> str:
        parts = []
        last_end = 0
        for m in re.finditer(
            r"^```(mermaid)?(\w+)?\s*\n(.*?)^```\s*$",
            text,
            re.DOTALL | re.MULTILINE,
        ):
            if m.start() > last_end:
                parts.append(("normal", text[last_end:m.start()]))
            lang = m.group(1) if m.group(1) else m.group(2) or ""
            parts.append(("code", lang, m.group(3)))
            last_end = m.end()
        if last_end < len(text):
            parts.append(("normal", text[last_end:]))

        result_parts = []
        for part in parts:
            if part[0] == "code":
                _, lang, code = part
                escaped = escape(code)
                if lang == "mermaid":
                    result_parts.append(f'<div class="mermaid">\n{escape(code)}\n</div>')
                else:
                    result_parts.append(
                        f'<pre><code class="language-{lang}">{escaped}</code></pre>'
                    )
            else:
                result_parts.append(self._convert_block(part[1]))
        return "\n".join(result_parts)

    def _convert_block(self, text: str) -> str:
        lines = text.split("\n")
        result = []
        i = 0
        in_table = False
        table_rows = []
        in_list = False
        list_type = "ul"
        list_items = []
        in_blockquote = False
        bq_lines = []

        def flush_table():
            if table_rows:
                rows_html = []
                for idx, row in enumerate(table_rows):
                    tag = "th" if idx == 0 else "td"
                    cells = "".join(
                        f"<{tag}>{c.strip()}</{tag}>"
                        for c in row.split("|")
                        if c.strip()
                    )
                    rows_html.append(f"<tr>{cells}</tr>")
                result.append("<table>" + "".join(rows_html) + "</table>")
                table_rows.clear()

        def flush_list():
            if list_items:
                tag = "ul" if list_type == "ul" else "ol"
                result.append(
                    f"<{tag}>"
                    + "".join(f"<li>{self._inline(item)}</li>" for item in list_items)
                    + f"</{tag}>"
                )
                list_items.clear()

        def flush_bq():
            if bq_lines:
                inner = self._inline("\n".join(bq_lines))
                result.append(f"<blockquote>{inner}</blockquote>")
                bq_lines.clear()

        while i < len(lines):
            line = lines[i]

            if not line.strip():
                flush_table()
                flush_list()
                flush_bq()
                in_table = False
                in_list = False
                in_blockquote = False
                i += 1
                continue

            if re.match(r"^-{3,}$", line):
                flush_table()
                flush_list()
                flush_bq()
                result.append("<hr>")
                i += 1
                continue

            h = re.match(r"^(#{1,4})\s+(.+)", line)
            if h:
                flush_table()
                flush_list()
                flush_bq()
                level = len(h.group(1))
                result.append(f"<h{level}>{self._inline(h.group(2))}</h{level}>")
                i += 1
                continue

            if (
                "|" in line
                and i + 1 < len(lines)
                and re.match(r"^\|[\s\-:|]+\|", lines[i + 1])
            ):
                flush_list()
                flush_bq()
                in_table = True
                table_rows.append(line)
                i += 2
                while (
                    i < len(lines)
                    and "|" in lines[i]
                    and not lines[i].strip().startswith("###")
                ):
                    table_rows.append(lines[i])
                    i += 1
                flush_table()
                in_table = False
                continue

            if "|" in line and in_table:
                table_rows.append(line)
                i += 1
                continue

            if line.startswith("> "):
                flush_table()
                flush_list()
                in_blockquote = True
                bq_lines.append(line[2:])
                i += 1
                continue

            list_match = re.match(r"^(\s*)([\*\-]|(\d+)\.)\s+(.*)", line)
            if list_match:
                flush_table()
                flush_bq()
                num = list_match.group(3)
                item = list_match.group(4)
                if num:
                    if list_type != "ol":
                        flush_list()
                        list_type = "ol"
                    in_list = True
                else:
                    if list_type != "ul":
                        flush_list()
                        list_type = "ul"
                    in_list = True
                list_items.append(item)
                i += 1
                continue

            flush_table()
            flush_bq()
            flush_list()
            para_lines = [line]
            i += 1
            while i < len(lines):
                nl = lines[i]
                if (
                    not nl.strip()
                    or re.match(r"^#{1,4}\s", nl)
                    or nl.startswith("> ")
                    or re.match(r"^(\s*)([\*\-]|\d+\.)\s", nl)
                    or re.match(r"^-{3,}$", nl)
                ):
                    break
                para_lines.append(nl)
                i += 1
            result.append(f'<p>{self._inline(" ".join(para_lines))}</p>')

        flush_table()
        flush_list()
        flush_bq()
        return "\n".join(result)

    def _inline(self, text: str) -> str:
        text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
        text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)
        text = re.sub(r"\*(.+?)\*", r"<em>\1</em>", text)
        text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2">\1</a>', text)
        text = re.sub(
            r'href="([^"]*\.md)"',
            lambda m: (
                f'href="{os.path.splitext(m.group(1))[0]}.html"'
                if "://" not in m.group(1)
                else m.group(0)
            ),
            text,
        )
        return text


def extract_title(text: str, fallback: str) -> str:
    fm = re.match(r"^---\s*\n(.*?)\n---\s*\n", text, re.DOTALL)
    if fm:
        title_m = re.search(r'^title:\s*"?([^"\n]+)"?', fm.group(1), re.MULTILINE)
        if title_m:
            return title_m.group(1).strip()
    h = re.search(r"^#\s+(.+)", text, re.MULTILINE)
    if h:
        return h.group(1).strip()
    return fallback


def strip_frontmatter(text: str) -> str:
    fm = re.match(r"^---\s*\n(.*?)\n---\s*\n", text, re.DOTALL)
    return text[fm.end():] if fm else text


def relative_prefix(rel_dir: Path) -> str:
    depth = len(rel_dir.parts) if str(rel_dir) != "." else 0
    return "../" * depth


def build(project_dir: str) -> None:
    project_dir = Path(project_dir).resolve()
    md_dir = project_dir / "markdown"
    html_dir = project_dir / "html"
    if not md_dir.exists():
        print(f"Error: {md_dir} does not exist")
        sys.exit(1)
    html_dir.mkdir(parents=True, exist_ok=True)
    converter = Md2Html()
    project_name = project_dir.name

    md_files = sorted(p for p in md_dir.rglob("*.md") if p.is_file())
    if not md_files:
        print("No markdown files found.")
        return

    print(f"Building {project_name} docs: {md_dir} -> {html_dir}")

    # Group files by directory, in sorted slug order.
    by_dir: dict[Path, list[Path]] = {}
    titles: dict[Path, str] = {}
    for md_file in md_files:
        rel = md_file.relative_to(md_dir)
        by_dir.setdefault(rel.parent, []).append(rel)
        text = md_file.read_text()
        titles[rel] = extract_title(text, md_file.stem)

    for rel_dir, files in by_dir.items():
        files.sort(key=lambda p: p.stem)
        slug_list = [p.stem for p in files]
        for rel in files:
            md_file = md_dir / rel
            text = md_file.read_text()
            body = converter.convert(strip_frontmatter(text))
            slug = rel.stem
            idx = slug_list.index(slug)

            breadcrumbs = ""
            for prev_rel in files[max(0, idx - 3):idx]:
                breadcrumbs += (
                    f'<a href="{prev_rel.stem}.html" class="nav-breadcrumb">'
                    f"{titles[prev_rel]}</a>"
                )

            if idx > 0:
                prev_slug = slug_list[idx - 1]
                prev_btn = (
                    f'<a href="{prev_slug}.html" class="nav-btn" '
                    f'title="Previous: {titles[files[idx - 1]]}">← prev</a>'
                )
            else:
                prev_btn = ""

            if idx < len(slug_list) - 1:
                next_slug = slug_list[idx + 1]
                next_btn = (
                    f'<a href="{next_slug}.html" class="nav-btn nav-btn-next" '
                    f'title="Next: {titles[files[idx + 1]]}">next →</a>'
                )
            else:
                next_btn = ""

            html = HTML_TEMPLATE.format(
                title=f"{titles[rel]} -- {project_name}",
                project=project_name,
                prefix=relative_prefix(rel_dir),
                content=body,
                breadcrumbs=breadcrumbs,
                prev_btn=prev_btn,
                next_btn=next_btn,
            )
            out_file = html_dir / rel.with_suffix(".html")
            out_file.parent.mkdir(parents=True, exist_ok=True)
            out_file.write_text(html)
            print(f"  ✓ {rel.with_suffix('.html')}  ({titles[rel]})")

    # Build an index for every directory that holds markdown, plus the root.
    all_dirs = set(by_dir.keys())
    for rel_dir in sorted(all_dirs, key=lambda p: (len(p.parts), str(p))):
        prefix = relative_prefix(rel_dir)
        up_link = (
            '<a href="../index.html" class="nav-btn" title="Up one level">up</a>'
            if str(rel_dir) != "."
            else ""
        )
        if str(rel_dir) == ".":
            heading = f"{project_name} Documentation"
            description = (
                "Electric and Durable Streams: the Postgres sync engine and the "
                "durable agent runtime built on append-only streams. Start with "
                "00-overview, then read 04-how-the-runtime-sets-up-a-stream."
            )
        else:
            heading = f"{project_name} / {rel_dir.as_posix()}"
            description = f"Documents for the {rel_dir.as_posix()} package."

        files = by_dir[rel_dir]
        file_links = "".join(
            f'<li><a href="{rel.stem}.html">{titles[rel]}</a></li>\n' for rel in files
        )

        child_dirs = sorted(
            {
                d
                for d in all_dirs
                if str(d) != str(rel_dir) and d.parent == rel_dir
            },
            key=str,
        )
        dir_links = ""
        if child_dirs:
            dir_links = "<h2>Packages</h2>\n<ul class=\"doc-index\">\n"
            for child in child_dirs:
                dir_links += (
                    f'<li><a href="{child.name}/index.html">{child.name}</a></li>\n'
                )
            dir_links += "</ul>\n"

        sections = ""
        if str(rel_dir) == "." and child_dirs:
            sections += dir_links
        sections += "<h2>Documents</h2>\n<ul class=\"doc-index\">\n" + file_links + "</ul>\n"
        if str(rel_dir) != "." and child_dirs:
            sections += dir_links

        index_html = INDEX_TEMPLATE.format(
            title=f"{heading} -- Documentation",
            project=project_name,
            prefix=prefix,
            up_link=up_link,
            heading=heading,
            description=description,
            sections=sections,
        )
        out_index = html_dir / rel_dir / "index.html"
        out_index.parent.mkdir(parents=True, exist_ok=True)
        out_index.write_text(index_html)
        print(f"  ✓ {(rel_dir / 'index.html').as_posix()}")

    # Ensure a root index exists even when every document is nested.
    root_index = html_dir / "index.html"
    if not root_index.exists():
        root_index.write_text(
            INDEX_TEMPLATE.format(
                title=f"{project_name} -- Documentation",
                project=project_name,
                prefix="",
                up_link="",
                heading=f"{project_name} Documentation",
                description="Generated documentation.",
                sections="",
            )
        )


if __name__ == "__main__":
    target = sys.argv[1] if len(sys.argv) > 1 else "."
    build(target)
