---
title: RAGatouille — RAG Framework for Python
---

# RAGatouille — RAG Framework for Python

**RAG (Retrieval-Augmented Generation) framework with ColBERT late interaction.**

Source: 27 Python files

## Architecture

```mermaid
flowchart LR
    Docs[Documents] --> Index[ColBERT Index]
    Index --> Retrieval[Retrieval]
    Query[Query] --> Retrieval
    Retrieval --> Rerank[Reranking]
    Rerank --> LLM[LLM Generation]
```

## Features

- ColBERT late interaction indexing
- Document retrieval with embeddings
- Query encoding
- Reranking support

## Docs

https://github.com/bclavie/RAGatouille
