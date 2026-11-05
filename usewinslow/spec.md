---
source_location: https://usewinslow.com (live public website), with https://usewinslow.com/winslow.md as the master feature list
repository: N/A (the website is closed source; the admin single-page application is a compiled bundle)
explored_at: 2026-11-06
documentation_goal: Explain, from public evidence only, what Winslow is, every page and offering on usewinslow.com, how the product works, and especially what the "Vibe Code Ready" promise on /builders/vibe-code/ means in practice — including the Model Context Protocol (MCP) server, its authorization model, its tool catalog, and the points where the marketing claim outruns the public documentation.
---

# Spec: usewinslow Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Location | `https://usewinslow.com` — 20 public pages downloaded on 2026-11-06 |
| Secondary source | `https://usewinslow.com/winslow.md` — 509 lines, 65,371 bytes, "Agent Dataroom" master feature list |
| Kind | Marketing website and product documentation for a commercial software-as-a-service (SaaS) product |
| Language | TypeScript/JavaScript (compiled), HyperText Markup Language (HTML), Markdown |
| Version | Live production site on 2026-11-06; admin bundle `main.26e32ed8.js` |
| License | Proprietary. © 2026 NFTCo, Inc. |
| Source of truth | The downloaded pages themselves, cited as `page-name:line` in the extracted text; live endpoint observations are marked with the request and the response |

There is no source code to read. The website is the grandfather. Every name, number, and flow in this documentation traces to one of these artifacts:

- the extracted page text in the research corpus (`home`, `about`, `ai-native-hr`, `pricing`, the four product pages, the five builder pages, the two solution pages, `integrations`, `expertise`, `i-hate-demos`, `privacy`, `terms`);
- `/winslow.md`, the machine-readable feature list;
- `/builders/mcp/`, the MCP reference, which lists 185 tools in 31 sections with scopes and rules;
- the compiled admin application routes (`/static/js/main.26e32ed8.js`, 242 routes);
- live endpoint probes: the MCP endpoint, the OAuth metadata document, the protected-resource metadata document, and the missing `/llms.txt` and `/AGENTS.md` files.

Where the documentation shows code, the snippets are real observed requests and responses, or real tool signatures copied from `/builders/mcp/`. No snippet invents a fact.

## 2. What usewinslow Is

Winslow (legal name NFTCO INC., doing business as Winslow) is an artificial-intelligence-native Human Resources (HR) platform. It combines United States payroll, global payroll and Employer of Record (EOR), benefits administration, a Human Resources Information System (HRIS), a Slack-based artificial intelligence co-worker, and a public Model Context Protocol (MCP) server. The company sells to United States startups and small companies, and its distinguishing claim is that the whole product is available to software through MCP: a customer can build an internal application with an AI builder and install it into Winslow's own navigation, where it reads and writes the employee record under Winslow's permissions. That claim is the "Vibe Code Ready" promise.

## 3. Documentation Goal

1. Know what Winslow sells, to whom, at what price, and with what limitations.
2. Know every page on the public website, what each page argues, and how the site is built and made readable to agents.
3. Know the three product surfaces — the admin single-page application, the public MCP server, and the agent runtime — and how they relate.
4. Know the Vibe Code Ready promise in full: its five steps, the request path, the permission inheritance claim, and the "replace parts of Winslow" claim.
5. Know how a builder would actually build and install an application today, using only public interfaces, and where the public documentation stops.
6. Know the MCP server: endpoint, transport, authentication, the three authorization checks, the four projections, the error model, and the session rules.
7. Know the full tool catalog: 185 tools in 31 sections, their scopes, write surface, and constraints.
8. Know the agent runtime: skills, schedules, event subscriptions, sequences, memory, personas, and proactive behavior.
9. Know each product line in depth: payroll and tax, payroll reporting, benefits, HRIS, integrations, artifacts, and audit.
10. Know the security and permission model, including field-level gating and the differences between MCP keys and OAuth tokens.
11. Know the pricing and packaging contradictions, and the gap between "nothing the admin user interface can do that your code cannot" and the documented "not in MCP" list.
12. Be able to give a critical assessment: what is real, what is marketing, what is underspecified, and what a buyer or builder should verify.

## 4. Documentation Structure

```
usewinslow/
├── spec.md
├── exploration.md
├── build.py                  (shared copy, with a usewinslow description entry)
├── styles.css
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-the-website.md
│   ├── 02-company-and-positioning.md
│   ├── 03-architecture.md
│   ├── 04-the-vibe-code-promise.md
│   ├── 05-building-and-installing-a-vibe-app.md
│   ├── 06-the-mcp-server.md
│   ├── 07-the-tool-catalog.md
│   ├── 08-the-ai-co-worker.md
│   ├── 09-payroll.md
│   ├── 10-payroll-reporting-and-compliance.md
│   ├── 11-benefits.md
│   ├── 12-hris.md
│   ├── 13-integrations-events-and-artifacts.md
│   ├── 14-permissions-security-and-audit.md
│   ├── 15-pricing-and-packaging.md
│   ├── 16-critical-assessment.md
│   └── 17-glossary.md
└── html/                     (generated)
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | markdown/README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-the-website.md | DONE |
| 3 | 02-company-and-positioning.md | DONE |
| 4 | 03-architecture.md | DONE |
| 5 | 04-the-vibe-code-promise.md | DONE |
| 6 | 05-building-and-installing-a-vibe-app.md | DONE |
| 7 | 06-the-mcp-server.md | DONE |
| 8 | 07-the-tool-catalog.md | DONE |
| 9 | 08-the-ai-co-worker.md | DONE |
| 10 | 09-payroll.md | DONE |
| 11 | 10-payroll-reporting-and-compliance.md | DONE |
| 12 | 11-benefits.md | DONE |
| 13 | 12-hris.md | DONE |
| 14 | 13-integrations-events-and-artifacts.md | DONE |
| 15 | 14-permissions-security-and-audit.md | DONE |
| 16 | 15-pricing-and-packaging.md | DONE |
| 17 | 16-critical-assessment.md | DONE |
| 18 | 17-glossary.md | DONE |
| 19 | exploration.md | DONE |
| 20 | Grandfather review (names, numbers, flows, coverage) | DONE |
| 21 | Fresh-reader review, every document | DONE |
| 22 | Generate HyperText Markup Language (HTML) | DONE |

## 6. Build System

Run `python3 -I build.py .` inside this directory. It reads `markdown/*.md`, writes `html/*.html`, generates `html/index.html`, and copies `styles.css`. It uses only the Python standard library.

This copy of `build.py` adds a `usewinslow` entry to the `descriptions` dictionary so the generated index page has a real summary. It also escapes raw angle brackets and processes inline markup inside table cells, as the shared copy does.

## 7. Quality Requirements (Iron Rules)

1. Detailed sections with code snippets (observed requests, responses, and tool signatures).
2. Teach key facts, principles, and ideas quickly.
3. Clear articulation, in ASD-STE100 Simplified Technical English.
4. Mermaid diagrams (minimum 2 per document).
5. Good visual assets (tables, ASCII structure, diagrams).
6. Generated HyperText Markup Language (HTML) through the shared build script.
7. Cross-references between documents.
8. Source references to the exact page and line, or to the exact observed request.
9. Aha moments — the non-obvious design choices and tradeoffs.
10. Navigation: index plus previous/next buttons, automatic.

Additional rules:

- Every abbreviation is spelled out where it is used, not only at the top.
- No sentence is longer than 30 words.
- The source wins over any summary. If a page says one thing and `/winslow.md` says another, both are reported.
- Public evidence only. When a mechanism is not documented, the text says "not documented", not a guess. Any reconstruction is marked **INFERRED**.

## 8. Expected Outcome

A reader can explain Winslow end to end: the company, the website, all product lines, the three surfaces, the Vibe Code Ready promise, the MCP server and its 185 tools, the permission model, and the pricing. A builder can decide whether the promise is usable today, and can follow a minimum working recipe. A buyer can see the contradictions and the questions to ask before signing. A reader can also see how an "agent-readable" website is built, because this exploration documents that layer too.

## 9. Resume Point

The set is complete. All nineteen documents, the README, and `exploration.md` are written. The grandfather review and the fresh-reader review have run, and the reported gaps are fixed. The HyperText Markup Language (HTML) is generated from the Markdown with `python3 -I build.py .` in this directory. To change a page, edit the Markdown file and run the build again; do not edit the HTML by hand.
