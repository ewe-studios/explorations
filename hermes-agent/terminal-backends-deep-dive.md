---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.NousResearch/hermes-agent
repository: git@github.com:NousResearch/hermes-agent.git
explored_at: 2026-03-25
---

# Terminal Backends Deep Dive

Hermes Agent supports six terminal backends for command execution: **local**, **Docker**, **SSH**, **Modal**, **Daytona**, and **Singularity**. This allows running the agent on a $5 VPS, GPU cluster, or serverless infrastructure with costs near zero when idle.

## Terminal Backend Architecture

```
tools/environments/
├── base.py              # BaseEnvironment ABC
├── local.py             # Local execution
├── docker.py            # Docker containers
├── ssh.py               # SSH remote execution
├── modal.py             # Modal cloud (serverless)
├── daytona.py           # Daytona cloud sandboxes
├── singularity.py       # Singularity containers
└── persistent_shell.py  # Persistent shell mixin
```

## Base Environment

```python
# tools/environments/base.py

class BaseEnvironment(ABC):
    """Abstract base class for all terminal backends."""

    def __init__(self, cwd: str = ".", timeout: int = 60):
        self.cwd = cwd
        self.timeout = timeout
        self._session_id = str(uuid.uuid4())

    @abstractmethod
    def execute(self, command: str, background: bool = False) -> Tuple[int, str, str]:
        """Execute a command and return (exit_code, stdout, stderr)."""

    @abstractmethod
    def close(self):
        """Clean up resources."""

    def send_file(self, local_path: str, remote_path: str):
        """Copy file into environment."""

    def get_file(self, remote_path: str, local_path: str):
        """Retrieve file from environment."""
```

## Local Backend

```python
# tools/environments/local.py

class LocalEnvironment(PersistentShellMixin, BaseEnvironment):
    """Local execution with interrupt support and non-blocking I/O."""

    def __init__(self, cwd: str = ".", timeout: int = 60,
                 persistent: bool = False):
        super().__init__(cwd=cwd, timeout=timeout)
        self.persistent = persistent

        if self.persistent:
            self._init_persistent_shell()

    def _build_env(self) -> dict:
        """Build subprocess environment with Hermes internal vars blocked."""
        env = os.environ.copy()

        # Block Hermes-managed provider API keys from leaking to subprocesses
        # See: https://github.com/NousResearch/hermes-agent/issues/1002
        blocked = _build_provider_env_blocklist()
        for key in blocked:
            env.pop(key, None)

        return env

    def execute(self, command: str, background: bool = False) -> Tuple[int, str, str]:
        """Execute command locally."""
        if background:
            return self._execute_background(command)
        return self._execute_foreground(command)

    def _execute_foreground(self, command: str) -> Tuple[int, str, str]:
        """Execute foreground command with interrupt support."""
        # Use output fences to isolate command output from shell init noise
        fence = "__HERMES_FENCE_a9f7b3__"

        full_command = f"printf '{fence}'; {command}; printf '{fence}'"

        process = subprocess.Popen(
            ["bash", "-c", full_command],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=self._build_env(),
        )

        # Register for interrupt handling
        from tools.interrupt import register_interruptible_process
        register_interruptible_process(process.pid)

        try:
            stdout, stderr = process.communicate(timeout=self.timeout)

            # Strip fence markers from output
            stdout_str = stdout.decode().split(fence)[1]
            stderr_str = stderr.decode()

            return process.returncode, stdout_str, stderr_str

        except subprocess.TimeoutExpired:
            process.kill()
            return -1, "", f"Command timed out after {self.timeout}s"

        finally:
            unregister_interruptible_process(process.pid)

    def _execute_background(self, command: str) -> Tuple[int, str, str]:
        """Execute background command via process registry."""
        from tools.process_registry import process_registry

        # Start process in background
        process = subprocess.Popen(
            ["bash", "-c", command],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=self._build_env(),
            start_new_session=True,  # Detach from process group
        )

        # Register with process registry for tracking
        process_registry.register(
            pid=process.pid,
            command=command,
            session_id=self._session_id,
            mode="background",
        )

        return 0, f"Process started with PID {process.pid}", ""
```

### Provider Environment Blocklist

```python
def _build_provider_env_blocklist() -> frozenset:
    """Build blocklist from provider registry dynamically.

    Automatically picks up api_key_env_vars and base_url_env_var
    from every registered provider, plus tool/messaging env vars.
    """
    blocked: set[str] = set()

    # From provider registry
    try:
        from hermes_cli.auth import PROVIDER_REGISTRY
        for pconfig in PROVIDER_REGISTRY.values():
            blocked.update(pconfig.api_key_env_vars)
            if pconfig.base_url_env_var:
                blocked.add(pconfig.base_url_env_var)
    except ImportError:
        pass

    # From optional env vars config
    try:
        from hermes_cli.config import OPTIONAL_ENV_VARS
        for name, metadata in OPTIONAL_ENV_VARS.items():
            category = metadata.get("category")
            if category in {"tool", "messaging"}:
                blocked.add(name)
    except ImportError:
        pass

    # Additional blocked vars
    blocked.update({
        "OPENAI_BASE_URL", "OPENAI_API_KEY", "OPENAI_ORG_ID",
        "ANTHROPIC_BASE_URL", "ANTHROPIC_TOKEN",
        "GOOGLE_API_KEY", "DEEPSEEK_API_KEY", "MISTRAL_API_KEY",
        "GROQ_API_KEY", "COHERE_API_KEY", "FIREWORKS_API_KEY",
        # Gateway config
        "TELEGRAM_BOT_TOKEN", "DISCORD_BOT_TOKEN", "SLACK_BOT_TOKEN",
    })

    return frozenset(blocked)
```

## Docker Backend

```python
# tools/environments/docker.py

class DockerEnvironment(BaseEnvironment):
    """Docker container execution with security hardening.

    Wraps mini-swe-agent's DockerEnvironment and adds:
    - Cap-drop ALL, no-new-privileges, PID limits
    - Configurable resource limits (CPU, memory, disk)
    - Optional filesystem persistence via bind mounts
    """

    def __init__(self, image: str, cwd: str = "/workspace",
                 timeout: int = 60, cpu_limit: float = 2.0,
                 memory_limit: str = "4g",
                 persistent_volume: Optional[str] = None):
        super().__init__(cwd=cwd, timeout=timeout)

        self.image = image
        self.cpu_limit = cpu_limit
        self.memory_limit = memory_limit
        self.persistent_volume = persistent_volume
        self._container_name = f"hermes-{uuid.uuid4().hex[:8]}"

        # Security flags applied to every container
        self._security_opts = [
            "no-new-privileges:true",  # Prevent privilege escalation
            "apparmor=unconfined",     # Disable AppArmor (can conflict)
        ]

        # Capabilities to drop
        self._cap_drop = [
            "ALL",  # Drop all capabilities
        ]

        # PIDs limit (fork bomb protection)
        self._pids_limit = 100

        self._start_container()

    def _start_container(self):
        """Start container with security hardening."""
        cmd = ["docker", "run", "-d", "--rm"]

        # Container name
        cmd.extend(["--name", self._container_name])

        # Resource limits
        cmd.extend(["--cpus", str(self.cpu_limit)])
        cmd.extend(["--memory", self.memory_limit])
        cmd.extend(["--pids-limit", str(self._pids_limit)])

        # Security options
        for opt in self._security_opts:
            cmd.extend(["--security-opt", opt])

        # Drop capabilities
        for cap in self._cap_drop:
            cmd.extend(["--cap-drop", cap])

        # Persistent volume (bind mount)
        if self.persistent_volume:
            cmd.extend(["-v", f"{self.persistent_volume}:/workspace"])

        # Working directory
        cmd.extend(["-w", self.cwd])

        # Image and command
        cmd.extend([self.image, "sleep", "infinity"])

        # Start container
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            raise RuntimeError(f"Failed to start container: {result.stderr}")

        self._container_id = result.stdout.strip()

    def execute(self, command: str, background: bool = False) -> Tuple[int, str, str]:
        """Execute command inside container."""
        cmd = [
            "docker", "exec",
            "-i",  # Interactive
            "-w", self.cwd,  # Working directory
        ]

        # Forward specific environment variables
        forward_env = self._get_forward_env()
        for key in forward_env:
            if key in os.environ:
                cmd.extend(["-e", f"{key}={os.environ[key]}"])

        cmd.extend([self._container_name, "bash", "-c", command])

        result = subprocess.run(cmd, capture_output=True, text=True, timeout=self.timeout)
        return result.returncode, result.stdout, result.stderr

    def _get_forward_env(self) -> List[str]:
        """Environment variables to forward to container."""
        return [
            "PATH",
            "PYTHONPATH",
            "GIT_AUTHOR_NAME",
            "GIT_AUTHOR_EMAIL",
            "GIT_COMMITTER_NAME",
            "GIT_COMMITTER_EMAIL",
        ]

    def close(self):
        """Stop and remove container."""
        subprocess.run(
            ["docker", "stop", self._container_name],
            capture_output=True,
            timeout=10,
        )
```

## SSH Backend

```python
# tools/environments/ssh.py

class SSHEnvironment(PersistentShellMixin, BaseEnvironment):
    """SSH remote execution with ControlMaster persistence.

    Uses SSH ControlMaster for connection persistence so subsequent
    commands are fast. Security benefit: agent cannot modify its own
    code since execution happens on separate machine.
    """

    def __init__(self, host: str, user: str, cwd: str = "~",
                 timeout: int = 60, port: int = 22,
                 key_path: str = "", persistent: bool = False):
        super().__init__(cwd=cwd, timeout=timeout)

        self.host = host
        self.user = user
        self.port = port
        self.key_path = key_path
        self.persistent = persistent

        # ControlMaster socket for connection persistence
        self.control_dir = Path(tempfile.gettempdir()) / "hermes-ssh"
        self.control_dir.mkdir(parents=True, exist_ok=True)
        self.control_socket = self.control_dir / f"{user}@{host}:{port}.sock"

        self._establish_connection()

        if self.persistent:
            self._init_persistent_shell()

    def _build_ssh_command(self, extra_args: list = None) -> list:
        """Build SSH command with ControlMaster options."""
        cmd = ["ssh"]

        # ControlMaster for connection persistence
        cmd.extend(["-o", f"ControlPath={self.control_socket}"])
        cmd.extend(["-o", "ControlMaster=auto"])
        cmd.extend(["-o", "ControlPersist=300"])  # Keep alive 5 min

        # Security options
        cmd.extend(["-o", "BatchMode=yes"])  # No interactive prompts
        cmd.extend(["-o", "StrictHostKeyChecking=accept-new"])
        cmd.extend(["-o", "ConnectTimeout=10"])

        if self.port != 22:
            cmd.extend(["-p", str(self.port)])
        if self.key_path:
            cmd.extend(["-i", self.key_path])

        if extra_args:
            cmd.extend(extra_args)

        cmd.append(f"{self.user}@{self.host}")
        return cmd

    def _establish_connection(self):
        """Establish initial SSH connection to create ControlMaster socket."""
        cmd = self._build_ssh_command()
        cmd.append("echo 'SSH connection established'")

        result = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
        if result.returncode != 0:
            raise RuntimeError(f"SSH connection failed: {result.stderr.strip()}")

    def execute(self, command: str, background: bool = False) -> Tuple[int, str, str]:
        """Execute command over SSH."""
        if background:
            return self._execute_background(command)

        # For foreground commands, use file-based IPC on remote host
        # This allows clean output capture even with shell init noise

        # Create remote temp files for output capture
        stdout_file = f"/tmp/hermes-ssh-{self._session_id}-stdout"
        stderr_file = f"/tmp/hermes-ssh-{self._session_id}-stderr"
        exit_file = f"/tmp/hermes-ssh-{self._session_id}-exit"

        # Wrap command to capture outputs
        wrapped_command = f"""
            {command} > {stdout_file} 2> {stderr_file}
            echo $? > {exit_file}
        """

        # Execute via SSH
        cmd = self._build_ssh_command(["bash", "-c", wrapped_command])
        subprocess.run(cmd, capture_output=True, timeout=self.timeout)

        # Read outputs via fast ControlMaster one-shot reads
        stdout_cmd = self._build_ssh_command(["cat", stdout_file])
        stderr_cmd = self._build_ssh_command(["cat", stderr_file])
        exit_cmd = self._build_ssh_command(["cat", exit_file])

        stdout = subprocess.run(stdout_cmd, capture_output=True, text=True).stdout
        stderr = subprocess.run(stderr_cmd, capture_output=True, text=True).stdout
        exit_code = int(subprocess.run(exit_cmd, capture_output=True, text=True).stdout.strip())

        # Cleanup temp files
        cleanup_cmd = self._build_ssh_command([
            "rm", "-f", stdout_file, stderr_file, exit_file
        ])
        subprocess.run(cleanup_cmd, capture_output=True)

        return exit_code, stdout, stderr
```

## Modal Backend

```python
# tools/environments/modal.py

class ModalEnvironment(BaseEnvironment):
    """Modal cloud execution with filesystem persistence.

    Wraps mini-swe-agent's SwerexModalEnvironment and adds:
    - Persistent filesystem snapshots across sessions
    - Configurable resources (CPU, memory, disk)
    - Sandbox lifecycle management
    """

    def __init__(self, image: str, cwd: str = "/root",
                 timeout: int = 60, persistent_filesystem: bool = True,
                 task_id: str = "default"):
        super().__init__(cwd=cwd, timeout=timeout)

        self._persistent = persistent_filesystem
        self._task_id = task_id
        self._base_image = image

        # Snapshot storage for persistent filesystems
        self._snapshot_store = get_hermes_home() / "modal_snapshots.json"

        # Try to restore from snapshot
        restored_image = None
        if self._persistent:
            snapshot_id = self._load_snapshots().get(self._task_id)
            if snapshot_id:
                try:
                    import modal
                    restored_image = modal.Image.from_id(snapshot_id)
                    logger.info("Modal: restored snapshot %s", snapshot_id[:20])
                except Exception as e:
                    logger.warning("Modal: failed to restore snapshot: %s", e)

        effective_image = restored_image if restored_image else image

        from minisweagent.environments.extra.swerex_modal import SwerexModalEnvironment
        self._inner = SwerexModalEnvironment(
            image=effective_image,
            cwd=cwd,
            timeout=timeout,
            modal_sandbox_kwargs={
                "cpu": (2, 4),  # Min 2, max 4 CPUs
                "memory": (4096, 8192),  # Min 4GB, max 8GB
            },
            install_pipx=True,  # Required: installs pipx + swe-rex runtime
        )

    def execute(self, command: str, background: bool = False) -> Tuple[int, str, str]:
        """Execute command in Modal sandbox."""
        # Add sudo -S support for password-less sudo
        if command.startswith("sudo"):
            command = command.replace("sudo", "sudo -S", 1)

        return self._inner.execute(command)

    def close(self):
        """Cleanup and optionally snapshot filesystem."""
        if self._persistent:
            # Create filesystem snapshot
            try:
                snapshot_id = self._inner._sandbox.filesystem.get_snapshot_id()
                self._save_snapshot(snapshot_id)
                logger.info("Modal: saved snapshot for task %s", self._task_id)
            except Exception as e:
                logger.warning("Modal: failed to save snapshot: %s", e)

        self._inner.close()

    def _load_snapshots(self) -> Dict[str, str]:
        """Load snapshot ID mapping from disk."""
        if self._snapshot_store.exists():
            return json.loads(self._snapshot_store.read_text())
        return {}

    def _save_snapshot(self, snapshot_id: str):
        """Persist snapshot ID mapping to disk."""
        snapshots = self._load_snapshots()
        snapshots[self._task_id] = snapshot_id
        self._snapshot_store.parent.mkdir(parents=True, exist_ok=True)
        self._snapshot_store.write_text(json.dumps(snapshots, indent=2))
```

## Daytona Backend

```python
# tools/environments/daytona.py

class DaytonaEnvironment(BaseEnvironment):
    """Daytona cloud sandbox execution.

    Uses stopped/started sandbox lifecycle for filesystem persistence
    instead of snapshots — faster and stateless on the host.
    """

    def __init__(self, image: str, cwd: str = "/home/daytona",
                 timeout: int = 60, cpu: int = 1,
                 memory: int = 5120,  # MB
                 disk: int = 10240,   # MB (10GB max on Daytona)
                 persistent_filesystem: bool = True,
                 task_id: str = "default"):
        super().__init__(cwd=cwd, timeout=timeout)

        self._persistent = persistent_filesystem
        self._task_id = task_id

        from daytona import Daytona, Resources

        # Convert to GiB for Daytona API
        memory_gib = max(1, math.ceil(memory / 1024))
        disk_gib = max(1, math.ceil(disk / 1024))

        # Cap disk to Daytona platform limit
        if disk_gib > 10:
            disk_gib = 10

        resources = Resources(cpu=cpu, memory=memory_gib, disk=disk_gib)

        self._daytona = Daytona()
        self._sandbox = None
        self._lock = threading.Lock()

        sandbox_name = f"hermes-{task_id}"

        # Try to resume existing sandbox
        if self._persistent:
            try:
                # Name-based lookup (new path)
                self._sandbox = self._daytona.get(sandbox_name)
                self._sandbox.start()
                logger.info("Daytona: resumed sandbox %s for task %s",
                            self._sandbox.id, task_id)
            except Exception as e:
                logger.warning("Daytona: failed to resume sandbox: %s", e)
                self._sandbox = None

        # Create new sandbox if no existing one
        if self._sandbox is None:
            self._sandbox = self._daytona.create(
                image=image,
                resources=resources,
                name=sandbox_name,
            )

    def execute(self, command: str, background: bool = False) -> Tuple[int, str, str]:
        """Execute command in Daytona sandbox."""
        with self._lock:
            # Ensure sandbox is running
            if self._sandbox.state == "stopped":
                self._sandbox.start()

            result = self._sandbox.process.exec(command)
            return result.exit_code, result.stdout, result.stderr

    def close(self):
        """Stop sandbox (don't remove) for persistence."""
        if self._persistent and self._sandbox:
            self._sandbox.stop()
        else:
            self._daytona.remove(self._sandbox.id)
```

## Singularity Backend

```python
# tools/environments/singularity.py

class SingularityEnvironment(BaseEnvironment):
    """Singularity container execution.

    Similar to Docker but with better HPC integration
    and native user namespace support.
    """

    def __init__(self, image: str, cwd: str = ".",
                 timeout: int = 60, bind_mounts: List[str] = None):
        super().__init__(cwd=cwd, timeout=timeout)

        self.image = image
        self.bind_mounts = bind_mounts or []

        # Check singularity availability
        self._singularity_bin = self._find_singularity()

    def _find_singularity(self) -> str:
        """Find singularity binary."""
        singularity = shutil.which("singularity")
        if not singularity:
            raise RuntimeError(
                "Singularity not found. Install with: apt install singularity-container"
            )
        return singularity

    def execute(self, command: str, background: bool = False) -> Tuple[int, str, str]:
        """Execute command inside Singularity container."""
        cmd = [self._singularity_bin, "exec"]

        # Add bind mounts
        for mount in self.bind_mounts:
            cmd.extend(["--bind", mount])

        # Container image
        cmd.append(self.image)

        # Command to execute
        cmd.extend(["bash", "-c", command])

        result = subprocess.run(cmd, capture_output=True, text=True, timeout=self.timeout)
        return result.returncode, result.stdout, result.stderr
```

## Persistent Shell Mixin

```python
# tools/environments/persistent_shell.py

class PersistentShellMixin:
    """Mixin for persistent shell execution.

    Keeps a single bash shell alive across execute() calls,
    preserving cwd, env vars, and shell variables.
    """

    def _init_persistent_shell(self):
        """Initialize persistent bash shell process."""
        self._shell_process = self._spawn_shell_process()
        self._shell_lock = threading.Lock()

        # Wait for shell to be ready
        self._read_until_prompt()

    def _spawn_shell_process(self) -> subprocess.Popen:
        """Spawn bash shell process."""
        return subprocess.Popen(
            ["bash", "--norc", "--noprofile", "-i"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0,
        )

    def _read_until_prompt(self, prompt: str = "__HERMES_PROMPT__") -> str:
        """Read shell output until prompt marker."""
        output = []
        while True:
            char = self._shell_process.stdout.read(1).decode()
            output.append(char)
            if prompt in "".join(output[-len(prompt):]):
                break
        return "".join(output)

    def _execute_persistent(self, command: str) -> Tuple[int, str, str]:
        """Execute command in persistent shell."""
        with self._shell_lock:
            # Send command with output capture
            wrapped = f"{command}; echo __EXIT_CODE__:$?; echo __HERMES_PROMPT__"
            self._shell_process.stdin.write(wrapped.encode())
            self._shell_process.stdin.flush()

            # Read output until prompt
            output = self._read_until_prompt()

            # Parse exit code
            exit_code_match = re.search(r"__EXIT_CODE__:(\d+)", output)
            exit_code = int(exit_code_match.group(1)) if exit_code_match else 0

            # Remove markers from output
            output = output.split("__HERMES_PROMPT__")[0]
            output = output.split("__EXIT_CODE__")[0]

            return exit_code, output, ""
```

## Backend Comparison

| Backend | Isolation | Startup | Persistence | Cost Model | Best For |
|---------|-----------|---------|-------------|------------|----------|
| **Local** | None | Instant | Manual | Free | Development |
| **Docker** | Container | ~1s | Volume mount | Free | Production |
| **SSH** | Remote host | ~100ms | Remote FS | VPS cost | VPS/Cloud |
| **Modal** | Sandbox | ~5s | Snapshot | Pay-per-use | Burst workloads |
| **Daytona** | Sandbox | ~3s | Stop/start | Pay-per-use | Dev sandboxes |
| **Singularity** | Container | ~1s | Bind mount | Free | HPC clusters |

## Summary

The terminal backends provide:

1. **Six backends** for different deployment scenarios
2. **Security hardening** (capability dropping, PID limits for Docker)
3. **Connection persistence** (SSH ControlMaster, persistent shells)
4. **Filesystem persistence** (snapshots for Modal, stop/start for Daytona)
5. **Resource limits** (CPU, memory, disk configuration)
6. **Environment isolation** (blocking Hermes internal env vars)
7. **Interrupt support** (foreground command cancellation)
8. **Background process tracking** (via process registry)
