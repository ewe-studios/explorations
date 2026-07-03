---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.jeremychone/vllm/
language: Python
version: 3.8
---

# vllm — High-Throughput LLM Serving Engine

vLLM is a high-throughput, memory-efficient LLM serving engine. It supports hundreds of models, continuous batching, PagedAttention, tensor parallelism, and streaming outputs.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.jeremychone/vllm/`

## Key Features

| Feature | Description |
|---------|-------------|
| PagedAttention | Efficient KV cache management |
| Continuous batching | Dynamic request batching |
| Tensor parallelism | Multi-GPU serving |
| Streaming | Token-by-token streaming |
| OpenAI API | Compatible endpoint |
| Multi-model | 100+ model support |

## Architecture

```
vllm/
├── vllm/                    ← Core Python package (598 files)
│   ├── attention/           ← Attention implementations
│   ├── core/                ← Core scheduler, block manager
│   ├── engine/              ← LLM engine
│   ├── entrypoints/         ← API server, CLI
│   ├── model_executor/      ← Model loading, execution
│   ├── transformers_utils/  ← HuggingFace integration
│   └── worker/              ← GPU worker processes
├── tests/                   ← Test suite
└── benchmarks/              ← Performance benchmarks
```

## Usage

```python
from vllm import LLM, SamplingParams

llm = LLM(model="meta-llama/Llama-3-8b")
outputs = llm.generate(["Hello, my name is"], SamplingParams(temperature=0.8))
```

## OpenAI-Compatible API

```bash
python -m vllm.entrypoints.openai.api_server --model meta-llama/Llama-3-8b
curl http://localhost:8000/v1/completions -d '{"model": "meta-llama/Llama-3-8b", "prompt": "Hello"}'
```
