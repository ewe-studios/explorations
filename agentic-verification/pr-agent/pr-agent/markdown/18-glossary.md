---
title: Glossary
---

# Glossary

Every abbreviation is spelled out here, in the order it first appears in this exploration. PR-Agent's own documentation uses these abbreviations freely; this glossary is the reference for the spelled-out form.

## Artificial intelligence and models

| Abbreviation | Spelled out |
|--------------|-------------|
| AI | artificial intelligence |
| LLM | large language model |
| API | application programming interface |
| SDK | software development kit |
| OpenAI | OpenAI (a company name, not an abbreviation) |
| GPT | generative pre-trained transformer |
| OIDC | OpenID Connect |

## Pull request and source control

| Abbreviation | Spelled out |
|--------------|-------------|
| PR | pull request |
| MR | merge request (the GitLab term for a pull request) |
| SHA | Secure Hash Algorithm |
| SHA-256 | Secure Hash Algorithm, 256-bit |
| Git | Git (a version control system name, not an abbreviation) |
| CI | continuous integration |

## Data formats and protocols

| Abbreviation | Spelled out |
|--------------|-------------|
| JSON | JavaScript Object Notation |
| JSONL | JavaScript Object Notation Lines |
| YAML | YAML Ain't Markup Language (a data format) |
| TOML | Tom's Obvious, Minimal Language |
| HTML | HyperText Markup Language |
| MD | Markdown |
| MDX | Markdown with JSX (JavaScript syntax extension) |
| HTTP | HyperText Transfer Protocol |
| HTTPS | HyperText Transfer Protocol Secure |
| URL | Uniform Resource Locator |
| URI | Uniform Resource Identifier |
| JSON-RPC | JavaScript Object Notation Remote Procedure Call |
| A2A | agent-to-agent |
| gRPC | gRPC Remote Procedure Calls (recursive) |

## Security

| Abbreviation | Spelled out |
|--------------|-------------|
| SSRF | server-side request forgery |
| JWT | JSON Web Token |
| SSL | Secure Sockets Layer |
| TLS | Transport Layer Security |
| CA | certificate authority |
| PAT | personal access token |
| ARN | Amazon Resource Name |
| W3C | World Wide Web Consortium |

## Cloud and platforms

| Abbreviation | Spelled out |
|--------------|-------------|
| AWS | Amazon Web Services |
| GCP | Google Cloud Platform |
| GCS | Google Cloud Storage |
| Azure AD | Azure Active Directory |

## Observability

| Abbreviation | Spelled out |
|--------------|-------------|
| OTel / OTEL | OpenTelemetry |
| OTLP | OpenTelemetry Protocol |
| PID | process identifier |
| UID | user identifier |
| UUID | universally unique identifier |
| UTF-8 | Unicode Transformation Format, 8-bit |
| UTF-16 | Unicode Transformation Format, 16-bit |

## Tooling

| Abbreviation | Spelled out |
|--------------|-------------|
| CLI | command line interface |
| CPU | central processing unit |
| MIT | Massachusetts Institute of Technology |
| pip | pip installs packages (a Python package installer) |
| PyPI | Python Package Index |

## Terms used in the code

| Term | Meaning |
|------|---------|
| handler | the object that turns a system prompt and a user prompt into a model call |
| provider | an object that connects PR-Agent to a hosted service (git, identity, or secret) |
| tool | one command implementation under `pr_agent/tools/` |
| diff | the unified diff of a pull request |
| hunk | one contiguous changed region inside a diff |
| sink | an output destination that is not the git provider |

## Where to go next

Return to the [Overview](00-overview.md) to start again, or read the [Index](README.md).
