# environments/ Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/environments/`

**Status:** complete

---

## Module Overview

The `environments/` module provides the Atropos RL training framework integration for Hermes Agent. This ~7,175 line module enables reinforcement learning training of language models using Hermes's tool-calling capabilities as the action space.

The module implements a layered architecture:
- **Agent Loop** - Reusable multi-turn agent loop with OpenAI-spec tool calling
- **Tool Context** - Per-rollout tool access for reward/verification functions
- **Base Environment** - Abstract base environment (Atropos `BaseEnv` subclass)
- **Tool Call Parsers** - Client-side parsers for various model output formats
- **Concrete Environments** - Task-specific environments (SWE-bench, terminal tests)
- **Benchmark Environments** - Evaluation-only environments (Terminal-Bench, YC Bench)

Key features:
- **Two-mode operation** - OpenAI server (Phase 1) or VLLM ManagedServer (Phase 2)
- **Per-group toolset resolution** - Different tool distributions per training batch
- **Sandbox isolation** - Modal, Daytona, Docker, SSH backends for safe execution
- **Reward computation** - Flexible reward functions with full tool context access

---

## Directory Structure

### Core Files

| File | Lines | Purpose |
|------|-------|---------|
| `__init__.py` | 36 | Package exports |
| `agent_loop.py` | 511 | Multi-turn agent loop |
| `hermes_base_env.py` | 670 | Abstract base environment |
| `tool_context.py` | 474 | Tool access for rewards |
| `patches.py` | 35 | Async compatibility patches |

### Tool Call Parsers

| File | Lines | Purpose |
|------|-------|---------|
| `tool_call_parsers/__init__.py` | 120 | Parser registry |
| `hermes_parser.py` | 73 | Hermes native format |
| `deepseek_v3_parser.py` | 89 | DeepSeek V3 format |
| `deepseek_v3_1_parser.py` | 72 | DeepSeek V3.1 format |
| `qwen_parser.py` | 19 | Qwen format |
| `qwen3_coder_parser.py` | 163 | Qwen3 Coder format |
| `glm45_parser.py` | 109 | GLM-4.5 format |
| `glm47_parser.py` | 35 | GLM-4.7 format |
| `llama_parser.py` | 96 | Llama format |
| `mistral_parser.py` | 135 | Mistral format |
| `kimi_k2_parser.py` | 93 | Kimi K2 format |
| `longcat_parser.py` | 69 | LongCat format |

### Concrete Environments

| File | Lines | Purpose |
|------|-------|---------|
| `terminal_test_env/__init__.py` | 0 | Test env package |
| `terminal_test_env/terminal_test_env.py` | 292 | Simple file-creation tasks |
| `hermes_swe_env/__init__.py` | 0 | SWE-bench package |
| `hermes_swe_env/hermes_swe_env.py` | 229 | SWE-bench tasks |

### Benchmark Environments (Eval-Only)

| File | Lines | Purpose |
|------|-------|---------|
| `benchmarks/__init__.py` | 0 | Benchmarks package |
| `benchmarks/tblite/__init__.py` | 0 | TBLite package |
| `benchmarks/tblite/tblite_env.py` | 119 | TBLite benchmark |
| `benchmarks/terminalbench_2/__init__.py` | 0 | Terminal-Bench 2.0 |
| `benchmarks/terminalbench_2/terminalbench2_env.py` | 958 | Terminal-Bench 2.0 env |
| `benchmarks/yc_bench/__init__.py` | 0 | YC Bench package |
| `benchmarks/yc_bench/yc_bench_env.py` | 847 | YC Benchmark env |

### Additional Core

| File | Lines | Purpose |
|------|-------|---------|
| `tool_context.py` | 474 | Per-rollout tool handle |
| `agent_loop.py` | 511 | Agent interaction loop |
| `hermes_base_env.py` | 670 | Base environment class |

**Total:** ~7,175 lines across 28+ files

---

## Key Components

### 1. Base Environment (`hermes_base_env.py`)

Abstract base class for all Hermes RL environments.

**Key Classes:**
```python
class HermesAgentEnvConfig(BaseEnvConfig):
    """Configuration for hermes-agent Atropos environments."""
    
    # Toolset configuration
    enabled_toolsets: Optional[List[str]] = None
    distribution: Optional[str] = None  # e.g., 'development', 'terminal_tasks'
    
    # Agent loop configuration
    max_agent_turns: int = 30
    system_prompt: Optional[str] = None
    agent_temperature: float = 1.0
    
    # Terminal backend
    terminal_backend: str = "local"  # or 'docker', 'modal', 'daytona', 'ssh'
    terminal_timeout: int = 120
    terminal_lifetime: int = 3600
    
    # Dataset
    dataset_name: Optional[str] = None
    prompt_field: str = "prompt"

class HermesAgentBaseEnv(BaseEnv):
    """Abstract base environment for Hermes + Atropos.
    
    Subclasses must implement:
        setup()           -- Load dataset, initialize state
        get_next_item()   -- Return next item from dataset
        format_prompt()   -- Convert dataset item to user message
        compute_reward()  -- Score the rollout (has ToolContext access)
        evaluate()        -- Periodic evaluation
    """
```

**Two-Mode Operation:**
```python
# Phase 1: OpenAI-compatible server
server_type = "openai"

# Phase 2: VLLM ManagedServer
server_type = "vllm_managed"
```

**Subclass Interface:**
```python
class MyEnv(HermesAgentBaseEnv):
    def setup(self):
        """Load dataset, initialize state."""
        self.dataset = load_dataset(self.config.dataset_name)
    
    def get_next_item(self) -> Item:
        """Return the next task from the dataset."""
        return next(self.data_iterator)
    
    def format_prompt(self, item: Item) -> str:
        """Convert dataset item to user prompt."""
        return item[self.config.prompt_field]
    
    def compute_reward(
        self, 
        trajectory: list, 
        tool_context: ToolContext
    ) -> float:
        """Score the rollout. Returns 0.0-1.0."""
        # Full tool context access for verification
        pass
    
    def evaluate(self) -> Dict[str, float]:
        """Periodic evaluation metrics."""
        return {"pass_rate": self.pass_rate}
```

### 2. Agent Loop (`agent_loop.py`)

Reusable multi-turn agent loop with tool calling.

**Key Classes:**
```python
@dataclass
class AgentResult:
    """Result of an agent rollout."""
    messages: List[dict]
    tool_calls: List[dict]
    tool_outputs: List[str]
    completed: bool
    error: Optional[str] = None

class HermesAgentLoop:
    """Multi-turn agent loop for RL training.
    
    Handles:
    - OpenAI-spec tool calling
    - Tool execution and result collection
    - Turn limiting
    - Error handling
    """
    
    def run_rollout(
        self,
        prompt: str,
        tools: list,
        max_turns: int = 30,
    ) -> AgentResult:
        """Run a complete agent rollout."""
```

**Rollout Flow:**
1. Send prompt to model with tools
2. Parse tool calls from response
3. Execute tools via ToolContext
4. Feed results back to model
5. Repeat until completion or max_turns

### 3. Tool Context (`tool_context.py`)

Per-rollout tool access handle for reward functions.

**Key Class:**
```python
class ToolContext:
    """Provides tool access within a rollout.
    
    Reward functions receive ToolContext to:
    - Execute verification commands
    - Check file states
    - Query sandbox state
    - Access execution results
    """
    
    def __init__(self, terminal_backend: str, task_id: str):
        self.backend = terminal_backend
        self.task_id = task_id
        self._sandbox = None
    
    def run_command(self, cmd: str, timeout: int = 60) -> Tuple[int, str, str]:
        """Run a command in the sandbox.
        
        Returns: (exit_code, stdout, stderr)
        """
    
    def read_file(self, path: str) -> str:
        """Read a file from the sandbox."""
    
    def write_file(self, path: str, content: str) -> None:
        """Write a file to the sandbox."""
    
    def list_files(self, dir: str = ".") -> List[str]:
        """List files in a directory."""
    
    def cleanup(self) -> None:
        """Clean up sandbox resources."""
```

### 4. Tool Call Parsers

Client-side parsers for extracting tool calls from model outputs.

**Parser Interface:**
```python
class ToolCallParser(ABC):
    """Abstract base for tool call parsers."""
    
    @abstractmethod
    def parse(self, content: str) -> List[Dict]:
        """Extract tool calls from model output."""
    
    @abstractmethod
    def format_result(self, result: str) -> str:
        """Format tool result for model."""
```

**Supported Formats:**
| Parser | Model Family | Format |
|--------|-------------|--------|
| `hermes_parser` | Hermes-tuned | Native Hermes XML |
| `deepseek_v3_parser` | DeepSeek V3 | JSON tool calls |
| `qwen_parser` | Qwen | Special tokens |
| `glm45_parser` | GLM-4.5 | Function call format |
| `llama_parser` | Llama 3.x | JSON with special tags |
| `mistral_parser` | Mistral | Tool call blocks |
| `kimi_k2_parser` | Kimi K2 | Custom format |

**Example Parser Usage:**
```python
from environments.tool_call_parsers import get_parser

parser = get_parser("qwen3-coder")
tool_calls = parser.parse(model_output)

for call in tool_calls:
    name = call["name"]
    args = call["arguments"]
    result = execute_tool(name, args)
    formatted = parser.format_result(result)
```

### 5. Terminal Test Environment

Simple environment for testing the RL stack.

**Task:** Create files with specific content.

**Reward Computation:**
```python
def compute_reward(self, trajectory, tool_context):
    # Check if file was created
    exit_code, _, _ = tool_context.run_command("test -f /tmp/test.txt")
    if exit_code != 0:
        return 0.0
    
    # Check file content
    content = tool_context.read_file("/tmp/test.txt")
    if "expected content" in content:
        return 1.0
    return 0.5
```

### 6. SWE-Bench Environment

SWE-bench style tasks with Modal sandboxes.

**Features:**
- GitHub issue reproduction
- Patch application and testing
- Test suite execution for reward

### 7. Benchmark Environments

Evaluation-only environments for standardized benchmarks.

**Terminal-Bench 2.0:**
- Terminal interaction tasks
- Command-line problem solving
- File manipulation challenges

**YC Bench:**
- Startup evaluation tasks
- Due diligence automation
- Application processing

---

## Terminal Backends

### Local Backend
```python
terminal_backend = "local"
# Uses local subprocess
# Fast for development, unsafe for production RL
```

### Docker Backend
```python
terminal_backend = "docker"
# Per-rollout container isolation
# Good for production RL
```

### Modal Backend
```python
terminal_backend = "modal"
# Cloud sandbox via Modal.com
# Best for large-scale RL training
# Automatic cleanup after lifetime
```

### Daytona Backend
```python
terminal_backend = "daytona"
# Daytona sandbox cloud
# Alternative to Modal
```

### SSH Backend
```python
terminal_backend = "ssh"
# Remote SSH server
# For dedicated training machines
```

### Singularity Backend
```python
terminal_backend = "singularity"
# HPC cluster compatibility
# For academic/research clusters
```

---

## Integration Points

### With AtroposLib
- `HermesAgentBaseEnv` extends `atroposlib.envs.base.BaseEnv`
- Uses `ServerManager` for VLLM/OpenAI server hosting
- Returns `ScoredDataGroup` with rollout results

### With Tools System
- Tool definitions from `model_tools.get_tool_definitions()`
- Tool distributions from `toolset_distributions.sample_toolsets_from_distribution()`
- Tool execution via `ToolContext`

### With Terminal Tool
- Reuses `tools.terminal_tool` sandbox management
- Same backend support (Docker, Modal, Daytona, SSH)

---

## Related Files

**Individual File Explorations:**
- [agent_loop.md](./environments/agent_loop.md)
- [hermes_base_env.md](./environments/hermes_base_env.md)
- [tool_context.md](./environments/tool_context.md)

**Related Modules:**
- [tools/rl_training_tool.md](../tools/rl_training_tool.md) - RL training interface
- [tools/environments/](../tools/environments/) - Sandbox backends

---

*Deep dive created: 2026-04-07*
