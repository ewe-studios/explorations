---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.HKUSD/LightRAG
repository: https://github.com/HKUDS/LightRAG
explored_at: 2026-03-20T00:00:00Z
language: Python
---

# LightRAG Exploration - Simple and Efficient RAG Framework

## Overview

LightRAG is a simple and efficient Retrieval-Augmented Generation (RAG) framework designed for lightweight deployment and high-performance question answering over custom knowledge bases.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.HKUSD/LightRAG`
- **Remote:** `git@github.com:HKUDS/LightRAG.git`
- **Primary Language:** Python
- **License:** MIT

## Key Features

- **Lightweight Design**: Minimal dependencies and simple architecture
- **Hybrid Retrieval**: Combines dense and sparse retrieval methods
- **Graph-Augmented**: Uses knowledge graphs for better context
- **Multi-Document**: Handles multiple document sources
- **Streaming Support**: Real-time response generation

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     LightRAG Pipeline                    │
│                                                          │
│  Documents ──► Chunking ──► Embedding ──► Vector Store │
│                       │                                  │
│                       ▼                                  │
│  Query ──► Hybrid Retrieval ──► Reranking ──► LLM      │
│                       │                                  │
│                       ▼                                  │
│              Knowledge Graph Integration                │
└─────────────────────────────────────────────────────────┘
```

## Configuration

```ini
[retrieval]
top_k = 5
hybrid = true

[embedding]
model = bge-large-en-v1.5

[llm]
model = qwen-7b
temperature = 0.7
```

## Usage Patterns

1. **Document Ingestion**: Load and chunk documents
2. **Index Building**: Create vector and graph indexes
3. **Query Processing**: Hybrid retrieval with reranking
4. **Response Generation**: LLM synthesis with context
