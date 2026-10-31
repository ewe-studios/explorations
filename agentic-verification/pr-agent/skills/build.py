#!/usr/bin/env python3
"""Convert markdown files to HTML. Zero dependencies -- stdlib only.

Usage:
    python3 build.py [PROJECT_DIR]

Where PROJECT_DIR is like 'documentation/pi' or 'documentation/hermes'.
Defaults to the directory this script lives in.
"""

import re
import os
import sys
from html import escape
from pathlib import Path


# ── HTML template ────────────────────────────────────────────────────────────

HTML_TEMPLATE = """\
<!DOCTYPE html>
<html lang="en" data-theme="light">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <link rel="stylesheet" href="styles.css">
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
        var bg = s.getPropertyValue(dark ? '--bg-strong' : '--bg-strong').trim();
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
      // ── Diagram zoom/pan modal ──────────────────────────────
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
        var btnIn = mkBtn('+', function(e) {{ e.stopPropagation(); zoom(1.3); }});
        var btnOut = mkBtn('−', function(e) {{ e.stopPropagation(); zoom(0.7); }});
        var btnReset = mkBtn('fit', function(e) {{ e.stopPropagation(); resetZoom(); }});
        var btnClose = mkBtn('✕', function(e) {{ e.stopPropagation(); closeModal(); }});
        toolbar.appendChild(btnIn); toolbar.appendChild(btnOut);
        toolbar.appendChild(btnReset); toolbar.appendChild(btnClose);
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
  <title>{project} -- Documentation</title>
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
    <span class="nav-brand">~/{project}/docs</span>
    <div class="nav-actions">
      <button class="theme-toggle" onclick="toggleTheme()" title="Toggle dark/light theme">theme</button>
    </div>
  </nav>

  <h1>{project} Documentation</h1>
  <p>{description}</p>
  <ul class="doc-index">
    {links}
  </ul>

  <hr>
  <p style="color: var(--fg-soft); font-size: 0.85rem;">
    Generated from markdown. Mermaid diagrams require JavaScript. Dark mode supported.
  </p>
  <script>function toggleTheme(){{var c=document.documentElement.getAttribute('data-theme'),n=c==='dark'?'light':'dark';document.documentElement.setAttribute('data-theme',n);localStorage.setItem('theme',n);}}</script>
</body>
</html>
"""


# ── Markdown → HTML converter ───────────────────────────────────────────────

class Md2Html:
    """Minimal markdown-to-HTML converter. Handles the subset our docs use."""

    def convert(self, text: str) -> str:
        self._mermaid_counter = 0

        # Extract fenced code blocks first to protect them from other processing
        parts = []
        last_end = 0
        for m in re.finditer(r'^```(mermaid)?(\w+)?\s*\n(.*?)^```\s*$', text, re.DOTALL | re.MULTILINE):
            if m.start() > last_end:
                parts.append(('normal', text[last_end:m.start()]))
            lang = m.group(1) if m.group(1) else m.group(2) or ''
            code = m.group(3)
            parts.append(('code', lang, code))
            last_end = m.end()
        if last_end < len(text):
            parts.append(('normal', text[last_end:]))

        result_parts = []
        for part in parts:
            if part[0] == 'code':
                _, lang, code = part
                escaped = escape(code)
                if lang == 'mermaid':
                    self._mermaid_counter += 1
                    result_parts.append(f'<div class="mermaid">\n{escape(code)}\n</div>')
                else:
                    result_parts.append(
                        f'<pre><code class="language-{lang}">{escaped}</code></pre>'
                    )
            else:
                result_parts.append(self._convert_block(part[1]))

        return '\n'.join(result_parts)

    def _convert_block(self, text: str) -> str:
        lines = text.split('\n')
        result = []
        i = 0
        in_table = False
        table_rows = []
        in_list = False
        list_type = 'ul'
        list_items = []
        in_blockquote = False
        bq_lines = []

        def flush_table():
            if table_rows:
                rows_html = []
                for idx, row in enumerate(table_rows):
                    tag = 'th' if idx == 0 else 'td'
                    cells = ''.join(f'<{tag}>{self._inline(c.strip())}</{tag}>' for c in row.split('|') if c.strip())
                    rows_html.append(f'<tr>{cells}</tr>')
                result.append('<table>' + ''.join(rows_html) + '</table>')
                table_rows.clear()

        def flush_list():
            if list_items:
                tag = 'ul' if list_type == 'ul' else 'ol'
                result.append(f'<{tag}>{"".join(f"<li>{self._inline(item)}</li>" for item in list_items)}</{tag}>')
                list_items.clear()

        def flush_bq():
            if bq_lines:
                inner = self._inline('\n'.join(bq_lines))
                result.append(f'<blockquote>{inner}</blockquote>')
                bq_lines.clear()

        while i < len(lines):
            line = lines[i]

            # Empty line -- flush any open block
            if not line.strip():
                flush_table()
                flush_list()
                flush_bq()
                in_table = False
                in_list = False
                in_blockquote = False
                i += 1
                continue

            # Horizontal rule
            if re.match(r'^-{3,}$', line):
                flush_table(); flush_list(); flush_bq()
                result.append('<hr>')
                i += 1
                continue

            # Headings
            h = re.match(r'^(#{1,4})\s+(.+)', line)
            if h:
                flush_table(); flush_list(); flush_bq()
                level = len(h.group(1))
                result.append(f'<h{level}>{self._inline(h.group(2))}</h{level}>')
                i += 1
                continue

            # Table row
            if '|' in line and i + 1 < len(lines) and re.match(r'^\|[\s\-:|]+\|', lines[i + 1]):
                flush_list(); flush_bq()
                in_table = True
                table_rows.append(line)
                i += 2  # skip separator line
                # Collect remaining rows
                while i < len(lines) and '|' in lines[i] and not lines[i].strip().startswith('###'):
                    table_rows.append(lines[i])
                    i += 1
                flush_table()
                in_table = False
                continue

            if '|' in line and in_table:
                table_rows.append(line)
                i += 1
                continue

            # Blockquote
            if line.startswith('> '):
                flush_table(); flush_list()
                in_blockquote = True
                bq_lines.append(line[2:])
                i += 1
                continue

            # List item
            list_match = re.match(r'^(\s*)([\*\-]|(\d+)\.)\s+(.*)', line)
            if list_match:
                flush_table(); flush_bq()
                indent = list_match.group(1)
                num = list_match.group(3)
                item = list_match.group(4)
                if num:
                    if list_type != 'ol':
                        flush_list()
                        list_type = 'ol'
                    in_list = True
                else:
                    if list_type != 'ul':
                        flush_list()
                        list_type = 'ul'
                    in_list = True
                list_items.append(item)
                i += 1
                continue

            # Normal paragraph text
            flush_table(); flush_bq()
            flush_list()
            # Collect consecutive non-special lines into one paragraph
            para_lines = [line]
            i += 1
            while i < len(lines):
                nl = lines[i]
                if (not nl.strip() or re.match(r'^#{1,4}\s', nl) or
                    nl.startswith('> ') or re.match(r'^(\s*)([\*\-]|\d+\.)\s', nl) or
                    re.match(r'^-{3,}$', nl)):
                    break
                para_lines.append(nl)
                i += 1
            result.append(f'<p>{self._inline(" ".join(para_lines))}</p>')

        flush_table(); flush_list(); flush_bq()
        return '\n'.join(result)

    def _inline(self, text: str) -> str:
        # lunora copy: escape raw HTML so angle brackets in prose and code spans render as text.
        text = escape(text, quote=False)
        # Inline code
        text = re.sub(r'`([^`]+)`', r'<code>\1</code>', text)
        # Bold
        text = re.sub(r'\*\*(.+?)\*\*', r'<strong>\1</strong>', text)
        # Italic
        text = re.sub(r'\*(.+?)\*', r'<em>\1</em>', text)
        # Links
        text = re.sub(r'\[([^\]]+)\]\(([^)]+)\)', r'<a href="\2">\1</a>', text)
        # Relative .md links point at source docs; in the rendered tree they
        # are .html. Leave absolute (scheme-bearing) URLs alone.
        text = re.sub(r'href="([^"]*\.md)"',
                      lambda m: (f'href="{os.path.splitext(m.group(1))[0]}.html"'
                                 if "://" not in m.group(1) else m.group(0)),
                      text)
        return text


# ── Build logic ──────────────────────────────────────────────────────────────

def build(project_dir: str):
    project_dir = Path(project_dir).resolve()
    md_dir = project_dir / 'markdown'
    html_dir = project_dir / 'html'
    if not md_dir.exists():
        print(f"Error: {md_dir} does not exist")
        sys.exit(1)

    html_dir.mkdir(parents=True, exist_ok=True)
    converter = Md2Html()
    project_name = project_dir.name
    print(f"Building {project_name} docs: {md_dir} → {html_dir}")

    # Discover markdown files
    md_files = sorted(md_dir.glob('*.md'))
    if not md_files:
        print("No markdown files found.")
        return

    # Build each markdown file to HTML
    file_map = {}  # slug → (filename, title)
    for md_file in md_files:
        with open(md_file) as f:
            content = f.read()

        # Extract title from frontmatter or first heading
        fm = re.match(r'^---\s*\n(.*?)\n---\s*\n', content, re.DOTALL)
        title = ''
        if fm:
            title_m = re.search(r'^title:\s*"?([^"\n]+)"?', fm.group(1), re.MULTILINE)
            if title_m:
                title = title_m.group(1).strip()
        if not title:
            h = re.search(r'^#\s+(.+)', content, re.MULTILINE)
            if h:
                title = h.group(1).strip()
        if not title:
            title = md_file.stem

        slug = md_file.stem  # e.g., "00-overview"
        file_map[slug] = (md_file.name, title)

    # Pre-compute ordered slug list for prev/next navigation
    all_slugs = list(file_map.keys())

    for md_file in md_files:
        with open(md_file) as f:
            content = f.read()

        # Extract title (already computed above, but needed for content)
        fm = re.match(r'^---\s*\n(.*?)\n---\s*\n', content, re.DOTALL)
        slug = md_file.stem
        title = file_map[slug][1]

        # Convert markdown to HTML
        body = content
        if fm:
            body = content[fm.end():]  # Strip frontmatter

        html_body = converter.convert(body)

        # Navigation: breadcrumbs, index btn, prev btn, next btn
        idx = all_slugs.index(slug)

        # Breadcrumbs: last 3 pages before current one
        breadcrumbs = ''
        for p in all_slugs[max(0, idx-3):idx]:
            breadcrumbs += f'<a href="{p}.html" class="nav-breadcrumb">{file_map[p][1]}</a>'

        # Previous button
        if idx > 0:
            prev_slug = all_slugs[idx - 1]
            prev_btn = f'<a href="{prev_slug}.html" class="nav-btn" title="Previous: {file_map[prev_slug][1]}">← prev</a>'
        else:
            prev_btn = ''

        # Next button
        if idx < len(all_slugs) - 1:
            next_slug = all_slugs[idx + 1]
            next_btn = f'<a href="{next_slug}.html" class="nav-btn nav-btn-next" title="Next: {file_map[next_slug][1]}">next →</a>'
        else:
            next_btn = ''

        html_content = HTML_TEMPLATE.format(
            title=f"{title} -- {project_name}",
            project=project_name,
            content=html_body,
            breadcrumbs=breadcrumbs,
            prev_btn=prev_btn,
            next_btn=next_btn,
        )

        out_file = html_dir / f"{slug}.html"
        with open(out_file, 'w') as f:
            f.write(html_content)
        print(f"  ✓ {slug}.html  ({title})")

    # Build index page
    links_html = ''
    for slug, (_, title) in file_map.items():
        links_html += f'<li><a href="{slug}.html">{title}</a></li>\n'

    descriptions = {
        'lunora': "Lunora: type-safe real-time backend on Cloudflare. Durable Object shards, D1, R2, Queues, change data capture, replicas and relays.",
        'pi': "Modular AI agent framework. 7 TypeScript packages for LLM APIs, agent runtimes, and applications.",
        'pi_latest': "Durable multi-process agent harness (durable) built on the chord typed host runtime, plus the codemode code-execution sandbox. Deep dive into durable execution, storage, scheduling, and live reload.",
        'hermes': "Self-improving AI agent. Python framework with 40+ tools, 10+ messaging platforms, and pluggable memory.",
        'autoresearch': "Autonomous AI research system. AI agent experiments with LLM training code overnight, ~100 experiments/night.",
        'open-pencil': "Open-source design editor. Opens .fig/.pen files, 100+ AI tools, MCP server, WebRTC collaboration, headless CLI + Vue SDK.",
        'paperclip': "Open-source AI company orchestration. Org charts, budgets, governance, and coordination for multi-agent teams.",
        'graphify': "Knowledge graph extraction tool. Turns mixed-media corpora into queryable graphs via tree-sitter AST, Whisper transcription, and Claude semantic extraction. 71.5x token reduction, 25 languages, 14 platform integrations.",
        'mastra': "TypeScript-first AI agent framework. Unified model router, workflow-based agentic loop, built-in memory, processor pipeline, and multi-model fallbacks.",
        'resonate': "Distributed computing framework. Durable execution, task coordination, and event-driven workflows for reliable background processing.",
        'webgl': "3D browser flight adventure game. Three.js rendering, procedural terrain, multiplayer via Socket.IO, quests, progression, atmospheric VFX.",
        'rust-authz': "Four Rust crates: Zanzibar-style FGA authorization engine (authz-core), PostgreSQL extension (pgauthz), auto-generated REST API for databases (dbrest), and telemetry ingestion platform (zradar).",
        'aipack': "Jeremy Chone's Rust crate collection: genai (19 AI providers), rpc-router (JSON-RPC 2.0), sqlb (SQL builder), modql (query language), agentic (MCP protocol), udiffx (diff parser), and 7 utility crates.",
        'voice-agent-server': "Voice AI assistant server. Express.js REST API managing Vapi voice assistants and phone numbers with 11Labs synthesis.",
        'mirage': "Unified Virtual File System for AI agents. Mounts 30+ services (S3, GDrive, Slack, GitHub, MongoDB, SSH) as a single filesystem tree. Agents use Unix commands (ls, grep, cat) across all backends. Python + TypeScript SDKs with two-layer cache.",
        'iii-engine': "Rust-based serverless engine collapsing queues, cron, HTTP, state, pub/sub, observability, and sandboxed execution into one system. Built on Worker/Function/Trigger primitives. microVM sandbox with userspace TCP/IP.",
        'agentmemory': "Persistent memory for AI coding agents. 53 MCP tools, triple-stream retrieval (BM25+Vector+Graph), 4-tier memory consolidation. Built on iii engine.",
        'spec-forge': "UI spec generation worker. Natural language to UI via Claude API with component catalogs, dual cache (SHA-256 + TF-IDF), collaborative sessions.",
        'iii-workers': "Collection of self-contained iii worker modules: database, shell, storage, MCP bridge, LSP, image-resize, ACP, and more. Rust primary, some Node.js/Python.",
        'iii-cli-tooling': "Rust workspace providing iii-tools and motia CLIs for project management. Shared scaffolder-core library with template support and optional TUI.",
        'iii-skills-validation': "Doc rendering and validation for iii workers. Renders markdown partials, validates with Vale (prose) and Anthropic AI (semantic). GitHub Action + pre-commit.",
        'iii-examples': "Standalone iii SDK examples across TypeScript (Bun, pnpm) and Python (uv): AI chat, human-in-the-loop, property search, todo app.",
        'moqdev': "Media over QUIC (MoQ) ecosystem. IETF draft protocol for low-latency live streaming over WebTransport. 18-crate Rust workspace with Go/Swift bindings, OBS plugin, and relay server.",
        'n0-computer': "Iroh P2P networking ecosystem. Dial-by-public-key QUIC connections with NAT hole-punching. 60+ projects including blobs, gossip, docs sync, RPC, and CLI tools.",
        'websocket': "WebSocket implementations in Rust. tungstenite-rs (RFC6455 core), tokio-tungstenite (async), websocat (CLI tool), plus deprecated rust-websocket and Sunrise reactive library.",
        'zcoder': "Parallel-first AI coding harness in Rust. A terminal UI client talks to a self-spawning daemon over a Unix socket; the daemon owns SQLite persistence and a single-writer executor that drives model calls via genai.",
        'proa': "Rust server-side rendering framework. Compiles Rust-authored HTML templates to pre-escaped byte sequences at compile time, streams over a chained-bump buffer, and hydrates interactive islands via the RSJS Rust-to-JavaScript compiler.",
        'durable-execution': "Resonate's durable-execution runtime. One primitive (the durable promise), a pure kernel, two ports, and a push model: the full Python engine in code/final, from the programming model to the conformance catalogue.",
        'divkit': "Yandex's Server-Driven UI framework. One JSON card renders as native views on iOS, Android, Web, and Flutter. 192 JSON Schema files are the single source of truth; a Python generator compiles them into four renderers' models and three builders' classes.",
        'ajdc': "Active Job Durable Continuation (AJ/DC) by Vladimir Dementyev. A Rails gem that makes ActiveJob::Continuable jobs durable: the run and its steps are database rows, so progress survives a crash, a run can wait for a time or a signal without holding a worker, and every run is identified, queryable, and resumable.",
        'activejob': "Ruby on Rails' ActiveJob: one job interface over many queue backends. A job is a Ruby object that is serialized (arguments via ActiveJob::Arguments, models via GlobalID), handed to a queue adapter, and later rebuilt and run. Covers the enqueue-to-perform lifecycle, adapters, callbacks, retry/discard, and the Continuable step machinery.",
        'activerecord': "Ruby on Rails' ActiveRecord ORM: a model is data plus behavior mapped to a database table, and a lazy immutable Relation chains into SQL through the Arel AST. Covers connection adapters and the pool, SQL generation, the query interface, persistence, associations, callbacks, migrations, attribute types, eager loading, and GlobalID/ActiveJob integration.",
        'feature_toggles': "feature_toggles: a Ruby gem (Evil Martians) for pending features that take longer than a single release cycle. Features are declared in a Ruby DSL config file (env/feature methods evaluated via instance_eval), so a team can see at a glance what is pending, done, or stalled and gate code on the list. Small: a single entry point over a few classes.",
        'flipper': "Flipper: beautiful, performant feature flags for Ruby and Rails. A feature flag is a boolean gate answered through a uniform API and stored by a pluggable adapter (memory, ActiveRecord, Redis, Sequel, Mongo, Moneta, Dalli, Rollout, cloud). Groups and percentage-of-actors rollouts build on top of the same gate. A monorepo of twelve gems.",
        'skooma': "Skooma: a Ruby library for validating API implementations against OpenAPI 3.1.0 documents. Each operation in an OpenAPI document carries a JSON Schema for its request and response; Skooma validates a real request or response body, headers, and parameters against that schema and returns the list of violations.",
        'anycable': "AnyCable-Go: the Go implementation of the AnyCable WebSocket server. It speaks the AnyCable protocol (protobuf over WebSocket), keeps a hub of connections per stream, fans broadcasts out over NATS or Redis pub/sub, and embeds an mruby engine (mrb/) to run Rails-side Ruby code for RPC callbacks. Built for high throughput and low memory.",
        'anycable-delivery': "anycable-delivery: how AnyCable-Go guarantees message ordering and at-least-once delivery. A deep dive into the stream model, the sharded in-memory hub and its fan-out, the broker's history and session cache, resume and offset replay, and where Redis/NATS pub/sub bounds cross-node ordering.",
        'logux': "Logux: a JavaScript/TypeScript framework for client-server synchronization. Every change is an action appended to a shared, persistent, replicated log. Covers three core packages: logux-core (the action log, node primitives, and the Logux protocol), logux-server (the WebSocket server that syncs actions and tracks subscriptions), and logux-actions (built-in actions such as sync-map, processing, subscriptions, zero-knowledge, and crdt-table).",
        'ai-iso-skill': "iso-figure: a Claude skill (by MrBongoC/Tolga Cohce, MIT) that draws interactive isometric line-art figures. Name an object and get one self-contained HTML/SVG file: a hairline isometric drawing on a numbered plate with parts that work when pressed. About 15 lines of projection code, no libraries.",
        'anatomy': "Anatomy: a Claude skill (by Ryan, MIT) for technical exploded-view figures of real objects. A kit of JavaScript modules (iso-kit, lathe, turn, tube, WebGL, canvas painter, React, PDF), nine references, many worked examples (raptor engine, ripple tank, turning dial, arcade cabinet), and a capture/verify/publish toolchain with an extensive test suite.",
        'awesome-opus-5.5-video': "Awesome Claude 5.5 Video: a curated catalog of 2,094 videos, motion graphics, 3D scenes and games made with Claude Opus 5.5, Sonnet 5.5, Haiku 5.5 and Fable 5.5, each with 5,000+ views on X. 515 works carry a prompt (235 full). Structured data in cases.json plus eight per-category lists in English and Chinese.",
        'colorfigures': "Colorfigures: a Claude skill (by April Zhu, MIT) that draws hand-inked, flat-coloured isometric figures of everyday objects as one HTML page. Real 3D bodies shaded in flat light, heavy ink silhouette, flat blue cast shadow, warm paper. Drag to turn all the way round; See-through reveals the working mechanism inside with labelled parts. Builds on iso-glow.",
        'iso-glow': "ISO-GLOW: a Claude skill (by April Zhu, MIT) for interactive isometric line-art figures with glow lighting. A dark hairline drawing where only what is live glows in one accent colour, plus a palette picker, a gallery shell, a three.js 3D mode, and a solid lit Lumen style. Builds on iso-figure.",
        'celld': "celld: a self-hosted, distributed Durable Objects daemon (Rust). Runs a Cloudflare Workers application — Workers, Durable Objects, KV, Queues, D1, R2, Workflows, Cron Triggers, and static assets — on your own machines from the wrangler.json you already have. A Cargo workspace where crates/logic owns behavior and crates/celld is the effect executor and adapter host.",
        'clawpatrol': "clawpatrol: an agent-security firewall (Go) that sits between agents and production, parses their traffic at the wire, and gates each action against rules written in HCL — for example blocking destructive SQL or pausing kubectl delete until a human approves. Ships a plugin SDK, a dashboard, and a gateway with brokered dials.",
        'clawpatrol-plugin-aws': "clawpatrol-plugin-aws: an external clawpatrol plugin (Go) that gates AWS API calls across many accounts. Terminates TLS, parses each request into an aws facet for rule matching, selects the target account, assumes a per-account role with a single base key, re-signs with SigV4, and forwards to AWS through the gateway brokered dial.",
        'vnotify': "vnotify: a lightweight Rust library (by the Deno authors) for monitoring S3 changes without external dependencies. Detects changes across millions of objects with one HeadObject and sometimes one ListObjectsV2 call by using a fixed-size S3 keyspace as a hash map to signal bucket-wide changes.",
        'walgit': "walgit: a git server that is one binary in front of an object store (Rust). Hosts git repositories with no database, no leader, and no local state that matters — point it at an S3 or GCS bucket and get smart HTTP fetch and push, Git LFS, a web UI, a JSON API with an SDK, per-repository push policy, and webhooks. Every machine is a disposable cache; the bucket is the repository.",
        'walgit-celld-replication': "walgit vs celld on S3: a comparative study of how two Deno-authored Rust systems replicate state to S3/GCS. walgit is a git server that treats the bucket as the repository (content-addressed objects, packs, a WAL, compare-and-swap keys, every machine a disposable cache); celld is a Durable Objects daemon that replicates each cell's SQLite state as LTX segments and fences ownership with compare-and-swap, gating every egress on a durability ticket for RPO=0. Side by side: the unit of state, the write path, consistency, and efficiency.",
        'durable-streams': "Durable Streams: the data primitive for the agent loop. A protocol and set of implementations for persistent, addressable, resumable event streams over plain HTTP — append-only logs with offset-based catch-up reads and live tailing (long-poll and SSE), exactly-once appends via idempotent producers, JSON mode with array flattening, and stream forking. A pnpm monorepo with a Go Caddy-plugin production server (memory and file store backends with bbolt metadata), a TypeScript dev server, a Cloudflare Workers plus Durable Objects server, ten client libraries, and higher-level packages (Durable Proxy, Durable State, StreamDB, StreamFS, Yjs, AI SDK transports).",
        'skills': "PR-Agent skills: a single maintainer skill (Markdown) for Claude and Cursor.",
    }
    desc = descriptions.get(project_name, f"Documentation for {project_name}.")

    index_html = INDEX_TEMPLATE.format(
        project=project_name,
        description=desc,
        links=links_html,
    )

    with open(html_dir / 'index.html', 'w') as f:
        f.write(index_html)
    print(f"  ✓ index.html")

    # Copy shared CSS if not already present
    css_file = html_dir / 'styles.css'
    if not css_file.exists():
        css = (Path(__file__).resolve().parent / 'styles.css').read_text()
        css_file.write_text(css)
        print(f"  ✓ styles.css")

    print(f"\nDone. Open {html_dir / 'index.html'} in a browser.")


if __name__ == '__main__':
    if len(sys.argv) > 1:
        build(sys.argv[1])
    else:
        # Default: build pi, hermes, open-pencil, and autoresearch from this directory
        base = Path(__file__).resolve().parent
        for proj in ['pi', 'hermes', 'open-pencil', 'autoresearch', 'paperclip', 'voice-agent-server', 'graphify', 'mastra', 'resonate', 'rust-authz', 'aipack', 'webgl', 'mirage', 'iii-engine', 'agentmemory', 'spec-forge', 'iii-workers', 'iii-cli-tooling', 'iii-skills-validation', 'iii-examples', 'moqdev', 'n0-computer', 'websocket']:
            p = base / proj
            if p.exists():
                build(str(p))
                print()
