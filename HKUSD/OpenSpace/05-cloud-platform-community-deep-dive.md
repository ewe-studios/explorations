# OpenSpace Cloud Platform and Skill Community - Deep Dive

## Overview

OpenSpace's Cloud Platform (`open-space.cloud`) is a centralized skill registry and community platform that enables autonomous agents to share, discover, and benefit from collectively evolved skills. The platform provides a sophisticated infrastructure for skill upload/download, hybrid search, authentication, embedding-based similarity matching, and group-based access control.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    OPENSPACE CLOUD PLATFORM ARCHITECTURE                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐         ┌─────────────────┐         ┌───────────────┐ │
│  │   Agent Host    │         │   Agent Host    │         │  Agent Host   │ │
│  │  (Claude Code)  │         │  (OpenClaw)     │         │  (nanobot)    │ │
│  │        │        │         │        │        │         │       │       │ │
│  │   OpenSpace     │         │   OpenSpace     │         │  OpenSpace    │ │
│  │   Client        │         │   Client        │         │  Client       │ │
│  └────────┬────────┘         └────────┬────────┘         └───────┬───────┘ │
│           │                           │                          │         │
│           └───────────────────────────┼──────────────────────────┘         │
│                                       │                                     │
│                                       ▼                                     │
│           ┌───────────────────────────────────────────────────────────┐    │
│           │              open-space.cloud Platform                     │    │
│           │  ┌─────────────────────────────────────────────────────┐  │    │
│           │  │              API Gateway / Load Balancer             │  │    │
│           │  └─────────────────────────────────────────────────────┘  │    │
│           │                          │                                 │    │
│           │     ┌────────────────────┼────────────────────┐           │    │
│           │     ▼                    ▼                    ▼           │    │
│           │  ┌─────────┐      ┌─────────────┐     ┌─────────────┐    │    │
│           │  │  Auth   │      │   Skills    │     │  Embedding  │    │    │
│           │  │ Service │      │   Service   │     │   Service   │    │    │
│           │  └─────────┘      └─────────────┘     └─────────────┘    │    │
│           │                          │                                 │    │
│           │     ┌────────────────────┼────────────────────┐           │    │
│           │     ▼                    ▼                    ▼           │    │
│           │  ┌─────────┐      ┌─────────────┐     ┌─────────────┐    │    │
│           │  │  Users  │      │ PostgreSQL  │     │    S3 /     │    │    │
│           │  │  & API  │      │  + pgvector │     │   Blob      │    │    │
│           │  │  Keys   │      │  (Skills)   │     │  Storage    │    │    │
│           │  └─────────┘      └─────────────┘     └─────────────┘    │    │
│           │                                                           │    │
│           └───────────────────────────────────────────────────────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Table of Contents

1. [Cloud Platform Architecture](#1-cloud-platform-architecture)
2. [HTTP Client Implementation](#2-http-client-implementation)
3. [Skill Upload Flow](#3-skill-upload-flow)
4. [Skill Download Flow](#4-skill-download-flow)
5. [Cloud Search System](#5-cloud-search-system)
6. [Authentication System](#6-authentication-system)
7. [Embedding System](#7-embedding-system)
8. [Group System](#8-group-system)
9. [API Reference](#9-api-reference)

---

## 1. Cloud Platform Architecture

### 1.1 Platform Overview

**open-space.cloud** serves as the central hub for the OpenSpace skill community, providing:

| Feature | Description |
|---------|-------------|
| **Skill Registry** | Centralized storage for evolved skills from all agents |
| **Hybrid Search** | BM25 + embedding-based semantic search with server-side ranking |
| **Version Lineage** | Complete parent-child relationship tracking across skill versions |
| **Access Control** | Public, private, and group-based visibility controls |
| **Diff Storage** | Unified diff storage for version comparison |
| **Quality Metrics** | Usage statistics and success rate tracking |

### 1.2 Client-Server Architecture

The platform follows a **client-server architecture** with the client being the `OpenSpaceClient` class and the server being the cloud API:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      CLIENT-SERVER INTERACTION                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Client (OpenSpaceClient)                Server (open-space.cloud)      │
│                                                                         │
│  ┌──────────────────┐                          ┌──────────────────┐    │
│  │  HTTPClient      │◄──── HTTP/HTTPS ────────►│  API Gateway     │    │
│  │  - Request build │    (JSON + Auth)         │  - Rate limiting │    │
│  │  - Response parse│                          │  - Request routing│   │
│  │  - Error handle  │                          └────────┬─────────┘    │
│  └────────┬─────────┘                                   │              │
│           │                                             ▼              │
│  ┌────────▼─────────┐                          ┌──────────────────┐    │
│  │  Cloud Methods   │                          │  Auth Service    │    │
│  │  - upload_skill  │                          │  - API key valid │    │
│  │  - download_skill│                          │  - User lookup   │    │
│  │  - search        │                          └──────────────────┘    │
│  │  - fetch_record  │                                   │              │
│  └────────┬─────────┘                          ┌────────▼─────────┐    │
│           │                                    │  Skills Service  │    │
│  ┌────────▼─────────┐                          │  - CRUD ops      │    │
│  │  Embedding       │                          │  - Search        │    │
│  │  - Generate      │                          │  - Diff storage  │    │
│  │  - Cache         │                          └────────┬─────────┘    │
│  └──────────────────┘                          ┌────────▼─────────┐    │
│                                                 │  PostgreSQL DB   │    │
│                                                 │  + pgvector      │    │
│                                                 │  - Skills table  │    │
│                                                 │  - Users table   │    │
│                                                 │  - Groups table  │    │
│                                                 └──────────────────┘    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.3 API Design Principles

The cloud API follows RESTful principles with JSON payloads:

**Request Format:**
```http
POST /records/embeddings/search HTTP/1.1
Host: open-space.cloud
Authorization: Bearer sk_xxxxxxxxxxxxxxxx
Content-Type: application/json

{
    "query": "Docker container monitoring",
    "limit": 300,
    "level": "workflow",
    "tags": ["docker", "monitoring"]
}
```

**Response Format:**
```json
{
    "results": [
        {
            "record_id": "docker-monitor__clo_abc123",
            "name": "Docker Container Monitor",
            "description": "Monitor and restart high-memory containers",
            "visibility": "public",
            "similarity_score": 0.94,
            "metadata": {...}
        }
    ]
}
```

### 1.4 Authentication Model

The platform uses **API key-based authentication**:

```python
# Authentication header format
headers = {
    "Authorization": f"Bearer {api_key}",
    "Content-Type": "application/json"
}
```

**API Key Characteristics:**
- Format: `sk_<random_string>` (typically 20+ characters)
- Stored locally in environment variable `OPENSPACE_API_KEY`
- Transmitted with every API request
- Validated server-side for each request

---

## 2. HTTP Client Implementation

### 2.1 OpenSpaceClient Class

The `OpenSpaceClient` class is the primary interface for interacting with the cloud platform:

```python
class OpenSpaceClient:
    """Client for the OpenSpace cloud API.
    
    Handles all HTTP communication with open-space.cloud including:
    - Skill upload/download
    - Hybrid search
    - Record management
    - Artifact handling
    """
    
    def __init__(
        self,
        auth_headers: Dict[str, str],
        api_base: str = "https://open-space.cloud/api",
    ):
        """Initialize the cloud client.
        
        Args:
            auth_headers: Authorization headers (e.g., {"Authorization": "Bearer sk_..."})
            api_base: Base URL for the API
        """
        self._headers = auth_headers
        self._base_url = api_base.rstrip("/")
        self._session: Optional[aiohttp.ClientSession] = None
```

### 2.2 HTTP Transport Layer

The client uses `aiohttp` for async HTTP communication:

```python
async def _get_session(self) -> aiohttp.ClientSession:
    """Get or create the aiohttp session."""
    if self._session is None or self._session.closed:
        self._session = aiohttp.ClientSession(
            headers=self._headers,
            timeout=aiohttp.ClientTimeout(total=30),
        )
    return self._session
```

### 2.3 Request Building

All requests follow a consistent pattern:

```python
async def _request(
    self,
    method: str,
    path: str,
    body: Optional[bytes] = None,
    extra_headers: Optional[Dict[str, str]] = None,
    timeout: int = 30,
) -> Tuple[Dict[str, str], bytes]:
    """Execute an HTTP request and return (headers, body).
    
    Args:
        method: HTTP method (GET, POST, etc.)
        path: API path (appended to base URL)
        body: Optional request body
        extra_headers: Additional headers to merge
        timeout: Request timeout in seconds
        
    Returns:
        Tuple of (response_headers, response_body)
        
    Raises:
        CloudError: On HTTP errors or network failures
    """
    url = f"{self._base_url}{path}"
    headers = dict(self._headers)
    
    if extra_headers:
        headers.update(extra_headers)
    
    session = await self._get_session()
    
    try:
        async with session.request(
            method,
            url,
            headers=headers,
            data=body,
            timeout=aiohttp.ClientTimeout(total=timeout),
        ) as response:
            response_body = await response.read()
            
            if response.status >= 400:
                raise CloudError(
                    f"API request failed: {method} {path} - {response.status}",
                    status_code=response.status,
                )
            
            return dict(response.headers), response_body
            
    except aiohttp.ClientError as e:
        raise CloudError(f"Network error: {e}") from e
```

### 2.4 Response Handling

Responses are parsed based on content type:

```python
def _parse_response(
    headers: Dict[str, str],
    body: bytes,
) -> Any:
    """Parse response based on content type."""
    content_type = headers.get("Content-Type", "")
    
    if "application/json" in content_type:
        return json.loads(body.decode("utf-8"))
    elif "application/zip" in content_type:
        return body  # Return raw bytes for downloads
    else:
        return body.decode("utf-8")
```

### 2.5 Error Handling

Comprehensive error handling with specific error types:

```python
class CloudError(Exception):
    """Base exception for cloud-related errors."""
    
    def __init__(
        self,
        message: str,
        status_code: Optional[int] = None,
        original_error: Optional[Exception] = None,
    ):
        self.message = message
        self.status_code = status_code
        self.original_error = original_error
        super().__init__(self.message)


class AuthenticationError(CloudError):
    """Raised when API key is invalid or expired."""
    pass


class NotFoundError(CloudError):
    """Raised when a requested resource doesn't exist."""
    pass


class RateLimitError(CloudError):
    """Raised when rate limit is exceeded."""
    pass
```

### 2.6 Retry Logic

The client implements exponential backoff for transient failures:

```python
class HTTPClient:
    """HTTP client with retry logic."""
    
    def __init__(self):
        self._retry_delays = [1.0, 2.0, 4.0, 8.0]  # Exponential backoff
        self._max_retries = 4
        self._retryable_status_codes = {429, 500, 502, 503, 504}
    
    async def execute_with_retry(
        self,
        operation: Callable,
        *args,
        **kwargs,
    ) -> Any:
        """Execute an HTTP operation with retry logic.
        
        Args:
            operation: Async callable to execute
            *args, **kwargs: Arguments to pass to operation
            
        Returns:
            Result of the operation
            
        Raises:
            CloudError: If all retries fail
        """
        last_error: Optional[Exception] = None
        
        for attempt in range(self._max_retries):
            try:
                return await operation(*args, **kwargs)
                
            except CloudError as e:
                last_error = e
                
                # Check if error is retryable
                if e.status_code not in self._retryable_status_codes:
                    raise
                
                # Check if we have retries left
                if attempt >= self._max_retries - 1:
                    raise
                
                # Wait before retry with exponential backoff
                delay = self._retry_delays[attempt]
                logger.warning(
                    f"Request failed (attempt {attempt + 1}/{self._max_retries}), "
                    f"retrying in {delay}s: {e}"
                )
                await asyncio.sleep(delay)
                
            except Exception as e:
                last_error = e
                if attempt >= self._max_retries - 1:
                    raise CloudError(
                        f"Operation failed after {self._max_retries} attempts",
                        original_error=e,
                    ) from e
                
                delay = self._retry_delays[attempt]
                await asyncio.sleep(delay)
        
        raise CloudError(
            f"Operation failed after {self._max_retries} attempts",
            original_error=last_error,
        )
```

---

## 3. Skill Upload Flow

### 3.1 Upload Overview

The skill upload process involves multiple steps to ensure data integrity and proper metadata handling:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SKILL UPLOAD FLOW                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  1. PREPARATION                                                         │
│     ┌──────────────────┐                                                │
│     │ Read SKILL.md    │                                                │
│     │ Parse frontmatter│                                                │
│     │ Read .skill_id   │                                                │
│     │ Read upload_meta │                                                │
│     └────────┬─────────┘                                                │
│              │                                                          │
│              ▼                                                          │
│  2. VALIDATION                                                          │
│     ┌──────────────────┐                                                │
│     │ Validate name    │                                                │
│     │ Check parents    │                                                │
│     │ Safety check     │                                                │
│     └────────┬─────────┘                                                │
│              │                                                          │
│              ▼                                                          │
│  3. STAGING (Artifact Creation)                                         │
│     ┌──────────────────┐                                                │
│     │ Collect files    │────────┐                                       │
│     │ Create zip       │        │                                       │
│     │ Upload artifact  │        ▼                                       │
│     │ Get artifact_id  │  ┌──────────────┐                             │
│     └────────┬─────────┘  │ S3 Storage   │                             │
│              │            └──────────────┘                             │
│              │                                                          │
│              ▼                                                          │
│  4. DIFF COMPUTATION                                                    │
│     ┌──────────────────┐                                                │
│     │ Fetch parent     │                                                │
│     │ Compare files    │                                                │
│     │ Generate diff    │                                                │
│     └────────┬─────────┘                                                │
│              │                                                          │
│              ▼                                                          │
│  5. RECORD CREATION                                                     │
│     ┌──────────────────┐                                                │
│     │ Build payload    │                                                │
│     │ POST /records    │                                                │
│     │ Get record_id    │                                                │
│     └────────┬─────────┘                                                │
│              │                                                          │
│              ▼                                                          │
│  6. LOCAL METADATA                                                      │
│     ┌──────────────────┐                                                │
│     │ Write .upload_meta.json                                          │
│     └──────────────────┘                                                │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Metadata Preparation

Before upload, metadata is gathered from multiple sources:

```python
@dataclass
class UploadMetadata:
    """Metadata for skill upload."""
    origin: str  # "imported", "derived", "fixed", "captured"
    visibility: str  # "public", "private", "group"
    parent_skill_ids: List[str] = field(default_factory=list)
    tags: List[str] = field(default_factory=list)
    created_by: str = ""
    change_summary: str = ""


def _read_upload_meta(skill_dir: Path) -> Dict[str, Any]:
    """Read upload metadata from .upload_meta.json or generate defaults."""
    meta_file = skill_dir / ".upload_meta.json"
    
    if meta_file.exists():
        return json.loads(meta_file.read_text(encoding="utf-8"))
    
    # Generate defaults
    return {
        "origin": "imported",
        "visibility": "public",
        "parent_skill_ids": [],
        "tags": [],
        "created_by": "",
        "change_summary": "",
    }


def _read_skill_id(skill_dir: Path) -> str:
    """Read skill_id from .skill_id sidecar file."""
    id_file = skill_dir / ".skill_id"
    
    if id_file.exists():
        return id_file.read_text(encoding="utf-8").strip()
    
    # Generate from directory name
    return f"{skill_dir.name}__imp_{uuid.uuid4().hex[:8]}"
```

### 3.3 Visibility Decision

Skills can be uploaded with three visibility levels:

```python
class SkillVisibility(str, Enum):
    """Cloud visibility options."""
    PUBLIC = "public"    # Visible to all users
    PRIVATE = "private"  # Only visible to uploader
    GROUP = "group"      # Visible to group members only
```

**Visibility Mapping:**
```python
def _map_visibility(visibility: str) -> str:
    """Map client visibility to API visibility."""
    if visibility == "private":
        return "group_only"  # Private = group with only uploader
    return "public"
```

**Visibility Decision Factors:**

| Factor | Public | Private | Group |
|--------|--------|---------|-------|
| Skill is experimental | No | Yes | Maybe |
| Contains sensitive info | No | Yes | Depends |
| Team-only workflow | No | No | Yes |
| General utility | Yes | No | No |
| Evolution in progress | No | Yes | Yes |

### 3.4 Artifact Staging

The skill directory is packaged as a zip artifact:

```python
def stage_artifact(self, skill_dir: Path) -> Tuple[str, int]:
    """Stage skill files and get artifact_id.
    
    Args:
        skill_dir: Path to skill directory
        
    Returns:
        Tuple of (artifact_id, file_count)
    """
    # Collect all files
    files: List[Tuple[str, bytes]] = []
    
    for file_path in skill_dir.rglob("*"):
        if file_path.is_file() and not file_path.name.startswith("."):
            relative_path = file_path.relative_to(skill_dir)
            content = file_path.read_bytes()
            files.append((str(relative_path), content))
    
    # Create zip in memory
    zip_buffer = io.BytesIO()
    with zipfile.ZipFile(zip_buffer, "w", zipfile.ZIP_DEFLATED) as zf:
        for path, content in files:
            zf.writestr(path, content)
    
    zip_data = zip_buffer.getvalue()
    
    # Upload to server
    artifact_id = self._upload_artifact(zip_data)
    
    return artifact_id, len(files)


def _upload_artifact(self, zip_data: bytes) -> str:
    """Upload artifact to server.
    
    Args:
        zip_data: Zip file contents as bytes
        
    Returns:
        artifact_id from server
    """
    # Multipart form upload
    boundary = f"----WebKitFormBoundary{uuid.uuid4().hex}"
    
    body = self._build_multipart_body(
        boundary=boundary,
        file_data=zip_data,
        file_name="skill.zip",
    )
    
    headers, response_body = self._request(
        "POST",
        "/artifacts/upload",
        body=body,
        extra_headers={
            "Content-Type": f"multipart/form-data; boundary={boundary}",
        },
    )
    
    response = json.loads(response_body.decode("utf-8"))
    return response["artifact_id"]
```

### 3.5 Server-Side Processing

On the server, the upload is processed as follows:

```python
# Server-side pseudocode for record creation

async def create_record_handler(request: Request) -> Response:
    """Handle POST /records endpoint."""
    
    # 1. Authenticate user
    user = await authenticate(request.headers.get("Authorization"))
    
    # 2. Parse payload
    payload = await request.json()
    
    # 3. Validate
    await validate_record_payload(payload, user)
    
    # 4. Verify artifact exists
    artifact = await get_artifact(payload["artifact_id"])
    if not artifact:
        return JSONResponse({"error": "Artifact not found"}, status=404)
    
    # 5. Generate embedding
    embedding = await generate_embedding(
        payload["name"],
        payload["description"],
        artifact.content,
    )
    
    # 6. Create database record
    record = SkillRecord(
        record_id=payload["record_id"],
        artifact_id=payload["artifact_id"],
        skill_id=payload["skill_id"],
        name=payload["name"],
        description=payload["description"],
        visibility=payload["visibility"],
        origin=payload["origin"],
        parent_skill_ids=payload.get("parent_skill_ids", []),
        tags=payload.get("tags", []),
        level=payload.get("level", "workflow"),
        content_diff=payload.get("content_diff", ""),
        created_by=user.id,
        embedding=embedding,  # pgvector vector
    )
    
    await db.execute(
        """
        INSERT INTO skill_records (...)
        VALUES (...record fields...)
        ON CONFLICT (record_id) DO UPDATE SET
            embedding = EXCLUDED.embedding,
            updated_at = NOW()
        """,
    )
    
    # 7. Update lineage relationships
    for parent_id in payload.get("parent_skill_ids", []):
        await db.execute(
            """
            INSERT INTO skill_lineage_parents (skill_id, parent_skill_id)
            VALUES ($1, $2)
            """,
            payload["record_id"],
            parent_id,
        )
    
    return JSONResponse({"record_id": payload["record_id"]})
```

### 3.6 Conflict Resolution

When uploading a skill that may already exist:

```python
def _check_conflicts(
    self,
    skill_id: str,
    name: str,
) -> Optional[Dict[str, Any]]:
    """Check for existing skills with same ID or name.
    
    Returns:
        Existing record if found, None otherwise
    """
    try:
        # Check by skill_id first
        existing = self.fetch_record(skill_id)
        if existing:
            return existing
        
        # Check by name (fuzzy match)
        search_results = self.search_skills(
            query=name,
            limit=10,
        )
        
        for result in search_results:
            if result.get("name", "").lower() == name.lower():
                return result
                
    except NotFoundError:
        pass
    
    return None


def _resolve_conflict(
    self,
    existing: Dict[str, Any],
    strategy: str = "new_version",
) -> str:
    """Resolve upload conflict.
    
    Args:
        existing: Existing record metadata
        strategy: Resolution strategy ("new_version", "overwrite", "abort")
        
    Returns:
        New record_id or existing record_id
    """
    if strategy == "abort":
        raise CloudError(
            f"Skill already exists: {existing['record_id']}. "
            "Use a different name or set strategy='new_version'"
        )
    
    elif strategy == "overwrite":
        # Delete existing and create new
        self._delete_record(existing["record_id"])
        return self._create_new_record()
    
    else:  # new_version
        # Create as child of existing
        return self._create_new_record(
            parent_skill_ids=[existing["record_id"]],
            origin="derived",
        )
```

### 3.7 Complete Upload Implementation

```python
async def upload_skill(
    self,
    skill_dir: Path,
    *,
    visibility: str = "public",
    origin: Optional[str] = None,
    tags: Optional[List[str]] = None,
    change_summary: Optional[str] = None,
) -> str:
    """Full workflow: stage → diff → create record.
    
    Args:
        skill_dir: Path to skill directory
        visibility: "public", "private", or "group"
        origin: "imported", "derived", "fixed", or "captured"
        tags: Optional list of tags
        change_summary: Description of changes (for derived skills)
        
    Returns:
        record_id of the created record
    """
    from openspace.skill_engine.skill_utils import parse_frontmatter
    
    skill_path = Path(skill_dir)
    skill_file = skill_path / SKILL_FILENAME
    
    if not skill_file.exists():
        raise CloudError(f"SKILL.md not found in {skill_dir}")
    
    # Read skill content
    content = skill_file.read_text(encoding="utf-8")
    fm = parse_frontmatter(content)
    name = fm.get("name", skill_path.name)
    description = fm.get("description", "")
    
    if not name:
        raise CloudError("SKILL.md frontmatter missing 'name' field")
    
    # Read or generate metadata
    metadata = _read_upload_meta(skill_dir)
    
    # Override with provided values
    if origin:
        metadata["origin"] = origin
    if tags:
        metadata["tags"] = tags
    if change_summary:
        metadata["change_summary"] = change_summary
    
    # Read skill_id
    skill_id = _read_skill_id(skill_dir)
    
    # Validate origin/parent relationships
    parents = metadata.get("parent_skill_ids", [])
    self._validate_origin_parents(metadata["origin"], parents)
    
    # Map visibility
    api_visibility = _map_visibility(visibility)
    
    # Step 1: Stage files (upload artifact)
    artifact_id, file_count = self.stage_artifact(skill_path)
    logger.info(f"Staged {file_count} files as artifact {artifact_id}")
    
    # Step 2: Compute diff vs parent (if derived/fixed)
    content_diff = None
    if parents:
        try:
            parent_record = self.fetch_record(parents[0])
            content_diff = self._compute_unified_diff(
                parent_record,
                skill_path,
            )
        except NotFoundError:
            logger.warning(f"Parent skill {parents[0]} not found, skipping diff")
    
    # Step 3: Create record
    record_id = f"{name}__clo_{uuid.uuid4().hex[:8]}"
    
    payload: Dict[str, Any] = {
        "record_id": record_id,
        "artifact_id": artifact_id,
        "skill_id": skill_id,
        "name": name,
        "description": description,
        "origin": metadata["origin"],
        "visibility": api_visibility,
        "parent_skill_ids": parents,
        "tags": metadata.get("tags", []),
        "level": "workflow",
    }
    
    if metadata.get("created_by"):
        payload["created_by"] = metadata["created_by"]
    if metadata.get("change_summary"):
        payload["change_summary"] = metadata["change_summary"]
    if content_diff is not None:
        payload["content_diff"] = content_diff
    
    record_data, status_code = self.create_record(payload)
    logger.info(f"Created record {record_id} with status {status_code}")
    
    # Step 4: Write local metadata
    _write_upload_meta(skill_dir, {
        **metadata,
        "record_id": record_id,
        "uploaded_at": datetime.utcnow().isoformat(),
    })
    
    return record_id
```

---

## 4. Skill Download Flow

### 4.1 Download Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SKILL DOWNLOAD FLOW                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  1. FETCH METADATA                                                      │
│     ┌──────────────────┐                                                │
│     │ GET /records/{id}│                                                │
│     │ Get record info  │                                                │
│     │ Get artifact_id  │                                                │
│     └────────┬─────────┘                                                │
│              │                                                          │
│              ▼                                                          │
│  2. DOWNLOAD ARTIFACT                                                   │
│     ┌──────────────────┐                                                │
│     │ GET /artifacts/  │                                                │
│     │     {artifact_id}│                                                │
│     │ Receive zip      │                                                │
│     └────────┬─────────┘                                                │
│              │                                                          │
│              ▼                                                          │
│  3. EXTRACT LOCALLY                                                     │
│     ┌──────────────────┐                                                │
│     │ Create dir       │                                                │
│     │ Extract zip      │                                                │
│     │ Write .skill_id  │                                                │
│     └────────┬─────────┘                                                │
│              │                                                          │
│              ▼                                                          │
│  4. REGISTER LOCALLY                                                    │
│     ┌──────────────────┐                                                │
│     │ Add to registry  │                                                │
│     │ Update store     │                                                │
│     └──────────────────┘                                                │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Download Implementation

```python
def import_skill(
    self,
    skill_id: str,
    target_dir: Path,
) -> Dict[str, Any]:
    """Download a cloud skill and extract to local directory.
    
    Args:
        skill_id: Record ID of the skill to download
        target_dir: Base directory for extraction
        
    Returns:
        Result dict with status, local_path, files, etc.
    """
    # Step 1: Fetch metadata
    logger.info(f"import_skill: fetching metadata for {skill_id}")
    record_data = self.fetch_record(skill_id)
    
    skill_name = record_data.get("name", skill_id)
    
    # Sanitize skill name for filesystem
    if "/" in skill_name or "\\" in skill_name or skill_name.startswith("."):
        skill_name = skill_id
    
    skill_dir = (target_dir / skill_name).resolve()
    
    # Security: ensure target is within base directory
    if not skill_dir.is_relative_to(target_dir.resolve()):
        raise CloudError(
            f"Skill name {skill_name!r} escapes target directory"
        )
    
    # Check if already exists locally
    if skill_dir.exists() and (skill_dir / SKILL_FILENAME).exists():
        return {
            "status": "already_exists",
            "skill_id": skill_id,
            "name": skill_name,
            "local_path": str(skill_dir),
        }
    
    # Step 2: Download artifact
    logger.info(f"import_skill: downloading artifact for {skill_id}")
    zip_data = self.download_artifact(skill_id)
    
    # Step 3: Extract
    skill_dir.mkdir(parents=True, exist_ok=True)
    extracted_files = self._extract_zip(zip_data, skill_dir)
    
    # Step 4: Write .skill_id sidecar
    (skill_dir / SKILL_ID_FILENAME).write_text(
        skill_id + "\n",
        encoding="utf-8",
    )
    
    # Step 5: Write download metadata
    download_meta = {
        "source": "cloud",
        "record_id": skill_id,
        "downloaded_at": datetime.utcnow().isoformat(),
        "original_name": record_data.get("name"),
        "visibility": record_data.get("visibility"),
    }
    (skill_dir / ".download_meta.json").write_text(
        json.dumps(download_meta, indent=2),
        encoding="utf-8",
    )
    
    return {
        "status": "success",
        "skill_id": skill_id,
        "name": skill_name,
        "description": record_data.get("description", ""),
        "local_path": str(skill_dir),
        "files": extracted_files,
    }


def _extract_zip(
    self,
    zip_data: bytes,
    target_dir: Path,
) -> List[str]:
    """Extract zip data to target directory.
    
    Args:
        zip_data: Zip file contents
        target_dir: Extraction directory
        
    Returns:
        List of extracted file paths
    """
    extracted = []
    
    with zipfile.ZipFile(io.BytesIO(zip_data), "r") as zf:
        for info in zf.infolist():
            # Security: prevent path traversal
            if not self._is_safe_path(target_dir, info.filename):
                logger.warning(f"Skipping unsafe path: {info.filename}")
                continue
            
            output_path = target_dir / info.filename
            
            if info.is_dir():
                output_path.mkdir(parents=True, exist_ok=True)
            else:
                output_path.parent.mkdir(parents=True, exist_ok=True)
                output_path.write_bytes(zf.read(info))
                extracted.append(str(output_path))
    
    return extracted


def _is_safe_path(self, base_dir: Path, file_path: str) -> bool:
    """Check if file path is safe (no path traversal)."""
    try:
        resolved = (base_dir / file_path).resolve()
        return resolved.is_relative_to(base_dir.resolve())
    except ValueError:
        return False
```

### 4.3 Dependency Handling

When downloading skills, dependencies may need to be resolved:

```python
def _resolve_dependencies(
    self,
    record_data: Dict[str, Any],
) -> List[str]:
    """Resolve skill dependencies.
    
    Args:
        record_data: Record metadata
        
    Returns:
        List of dependency record_ids to download
    """
    dependencies = []
    
    # Check parent skills (for derived skills)
    parent_ids = record_data.get("parent_skill_ids", [])
    for parent_id in parent_ids:
        if not self._is_locally_available(parent_id):
            dependencies.append(parent_id)
    
    # Check explicit dependencies in metadata
    explicit_deps = record_data.get("metadata", {}).get("depends_on", [])
    for dep_id in explicit_deps:
        if not self._is_locally_available(dep_id):
            dependencies.append(dep_id)
    
    return dependencies


def download_with_dependencies(
    self,
    skill_id: str,
    target_dir: Path,
) -> Dict[str, Any]:
    """Download skill and all dependencies.
    
    Args:
        skill_id: Record ID to download
        target_dir: Base directory
        
    Returns:
        Result dict with all downloaded skills
    """
    downloaded = []
    
    # First, fetch metadata
    record_data = self.fetch_record(skill_id)
    
    # Resolve dependencies
    dependencies = self._resolve_dependencies(record_data)
    
    # Download dependencies first
    for dep_id in dependencies:
        try:
            result = self.import_skill(dep_id, target_dir)
            downloaded.append(result)
            logger.info(f"Downloaded dependency: {dep_id}")
        except NotFoundError:
            logger.warning(f"Dependency not found: {dep_id}")
    
    # Download main skill
    result = self.import_skill(skill_id, target_dir)
    downloaded.append(result)
    
    return {
        "status": "success",
        "main_skill": result,
        "dependencies": downloaded[:-1],
        "total_downloaded": len(downloaded),
    }
```

---

## 5. Cloud Search System

### 5.1 Hybrid Search Architecture

The cloud platform uses a **hybrid search** approach combining BM25 lexical matching with embedding-based semantic search:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      HYBRID SEARCH ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Client Query: "Docker container monitoring"                            │
│       │                                                                 │
│       ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ STAGE 1: Client-Side Preparation                             │       │
│  │ - Generate query embedding (local or API)                   │       │
│  │ - Build search payload                                      │       │
│  └─────────────────────────────────────────────────────────────┘       │
│       │                                                                 │
│       ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ POST /records/embeddings/search                              │       │
│  │ - query: "Docker container monitoring"                      │       │
│  │ - query_embedding: [0.123, -0.456, ...] (1536 dims)         │       │
│  │ - limit: 300                                                 │       │
│  │ - filters: {visibility: "public", tags: ["docker"]}         │       │
│  └─────────────────────────────────────────────────────────────┘       │
│       │                                                                 │
│       ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ STAGE 2: Server-Side Processing                              │       │
│  │ ┌───────────────────────────────────────────────────────┐   │       │
│  │ │ PostgreSQL + pgvector                                  │   │       │
│  │ │ - Cosine similarity search on embedding column        │   │       │
│  │ │ - WHERE visibility = 'public' AND tags && ['docker']  │   │       │
│  │ │ - ORDER BY embedding <-> query_embedding DESC          │   │       │
│  │ │ - LIMIT 300                                            │   │       │
│  │ └───────────────────────────────────────────────────────┘   │       │
│  └─────────────────────────────────────────────────────────────┘       │
│       │                                                                 │
│       ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ STAGE 3: Response                                           │       │
│  │ - Ranked results with similarity scores                     │       │
│  │ - Metadata for each record                                  │       │
│  └─────────────────────────────────────────────────────────────┘       │
│       │                                                                 │
│       ▼                                                                 │
│  Final Results: [Skill1 (0.94), Skill2 (0.89), ...]                    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Search Implementation

```python
RECORD_EMBEDDING_SEARCH_MAX_LIMIT = 300


def search_record_embeddings(
    self,
    *,
    query: str,
    limit: int = RECORD_EMBEDDING_SEARCH_MAX_LIMIT,
    level: Optional[str] = None,
    tags: Optional[List[str]] = None,
) -> List[Dict[str, Any]]:
    """POST /records/embeddings/search - fetch server-ranked embedding rows.
    
    Args:
        query: Search query string
        limit: Maximum number of results
        level: Optional filter by skill level (e.g., "workflow", "pattern")
        tags: Optional list of tags to filter by
        
    Returns:
        List of ranked skill records
    """
    search_request_payload: Dict[str, Any] = {
        "query": query,
        "limit": limit,
    }
    
    if level:
        search_request_payload["level"] = level
    if tags:
        search_request_payload["tags"] = tags
    
    _, response_body = self._request(
        "POST",
        "/records/embeddings/search",
        body=json.dumps(search_request_payload).encode("utf-8"),
        extra_headers={"Content-Type": "application/json"},
        timeout=30,  # Longer timeout for search
    )
    
    return json.loads(response_body.decode("utf-8"))
```

### 5.3 Server-Side Ranking

The server performs ranking using PostgreSQL's pgvector extension:

```sql
-- Server-side SQL for embedding search
SELECT 
    r.record_id,
    r.name,
    r.description,
    r.visibility,
    r.tags,
    r.origin,
    r.created_by,
    r.embedding <-> $query_embedding AS similarity_score
FROM skill_records r
WHERE r.visibility = $visibility_filter
  AND ($tags::text[] IS NULL OR r.tags && $tags)
  AND ($level::text IS NULL OR r.level = $level)
ORDER BY r.embedding <-> $query_embedding
LIMIT $limit;
```

**Ranking Algorithm:**

The `<->` operator computes **cosine distance** (1 - cosine similarity):
```python
# Cosine similarity
similarity = dot(a, b) / (norm(a) * norm(b))

# Cosine distance (used for ordering)
distance = 1 - similarity
```

### 5.4 Filters

The search supports multiple filter types:

```python
@dataclass
class SearchFilters:
    """Filters for skill search."""
    visibility: Optional[str] = None  # "public", "private", "group"
    level: Optional[str] = None       # "workflow", "pattern", "reference"
    tags: Optional[List[str]] = None  # Tag filter (AND logic)
    any_tags: Optional[List[str]] = None  # Tag filter (OR logic)
    created_after: Optional[datetime] = None
    created_before: Optional[datetime] = None
    origin: Optional[str] = None      # "imported", "derived", "fixed", "captured"
    created_by: Optional[str] = None  # User ID filter


def search_with_filters(
    self,
    query: str,
    filters: SearchFilters,
    limit: int = 100,
) -> List[Dict[str, Any]]:
    """Search with comprehensive filtering."""
    payload = {
        "query": query,
        "limit": limit,
    }
    
    if filters.visibility:
        payload["visibility"] = filters.visibility
    if filters.level:
        payload["level"] = filters.level
    if filters.tags:
        payload["tags"] = filters.tags
    if filters.any_tags:
        payload["any_tags"] = filters.any_tags
    if filters.created_after:
        payload["created_after"] = filters.created_after.isoformat()
    if filters.created_before:
        payload["created_before"] = filters.created_before.isoformat()
    if filters.origin:
        payload["origin"] = filters.origin
    if filters.created_by:
        payload["created_by"] = filters.created_by
    
    return self.search_record_embeddings(**payload)
```

### 5.5 Pagination

For large result sets, pagination is supported:

```python
def search_paginated(
    self,
    query: str,
    *,
    page: int = 1,
    page_size: int = 50,
    **filters,
) -> Dict[str, Any]:
    """Paginated search results.
    
    Args:
        query: Search query
        page: Page number (1-indexed)
        page_size: Results per page
        **filters: Additional filter arguments
        
    Returns:
        Dict with results, page info, and total count
    """
    offset = (page - 1) * page_size
    
    # Get page of results
    payload = {
        "query": query,
        "limit": page_size,
        "offset": offset,
        **filters,
    }
    
    _, response_body = self._request(
        "POST",
        "/records/embeddings/search/paginated",
        body=json.dumps(payload).encode("utf-8"),
        extra_headers={"Content-Type": "application/json"},
    )
    
    return json.loads(response_body.decode("utf-8"))


# Response format:
# {
#     "results": [...],
#     "page": 1,
#     "page_size": 50,
#     "total_results": 234,
#     "total_pages": 5,
#     "has_next": true,
#     "has_prev": false,
# }
```

---

## 6. Authentication System

### 6.1 API Key Management

API keys are the primary authentication mechanism:

```python
class AuthenticationManager:
    """Manages API key storage and retrieval."""
    
    def __init__(self):
        self._key_file = Path.home() / ".openspace" / "api_key"
        self._env_var = "OPENSPACE_API_KEY"
    
    def get_api_key(self) -> Optional[str]:
        """Get API key from environment or file.
        
        Priority:
        1. Environment variable OPENSPACE_API_KEY
        2. File ~/.openspace/api_key
        3. None (not authenticated)
        """
        # Check environment first
        env_key = os.environ.get(self._env_var)
        if env_key:
            return env_key.strip()
        
        # Check file
        if self._key_file.exists():
            try:
                return self._key_file.read_text(encoding="utf-8").strip()
            except OSError:
                pass
        
        return None
    
    def set_api_key(self, api_key: str, persist: bool = True) -> None:
        """Store API key.
        
        Args:
            api_key: The API key to store
            persist: If True, save to file; if False, only set env var
        """
        if persist:
            self._key_file.parent.mkdir(parents=True, exist_ok=True)
            self._key_file.write_text(api_key, encoding="utf-8")
            # Set restrictive permissions (owner read/write only)
            self._key_file.chmod(0o600)
        
        os.environ[self._env_var] = api_key
    
    def clear_api_key(self) -> None:
        """Remove stored API key."""
        if self._key_file.exists():
            self._key_file.unlink()
        os.environ.pop(self._env_var, None)
```

### 6.2 Key Storage

API keys can be stored in multiple locations:

| Location | Priority | Use Case |
|----------|----------|----------|
| Environment variable | 1 | Temporary/session auth, CI/CD |
| ~/.openspace/api_key | 2 | Persistent local auth |
| Project .env file | 3 | Project-specific auth |

**Secure Storage:**
```python
def _secure_write(path: Path, content: str) -> None:
    """Write content with secure permissions."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    
    # Owner read/write only
    path.chmod(0o600)
```

### 6.3 Key Validation

API keys are validated on each request:

```python
def get_openspace_auth() -> Tuple[Optional[Dict[str, str]], str]:
    """Get authentication headers and API base URL.
    
    Returns:
        Tuple of (auth_headers, api_base)
        auth_headers is None if not authenticated
    """
    auth_manager = AuthenticationManager()
    api_key = auth_manager.get_api_key()
    
    if not api_key:
        return None, "https://open-space.cloud/api"
    
    headers = {
        "Authorization": f"Bearer {api_key}",
    }
    
    # Get API base from environment or use default
    api_base = os.environ.get(
        "OPENSPACE_API_BASE",
        "https://open-space.cloud/api",
    )
    
    return headers, api_base


def validate_api_key(self, api_key: str) -> bool:
    """Validate an API key.
    
    Args:
        api_key: Key to validate
        
    Returns:
        True if valid, False otherwise
    """
    try:
        headers = {"Authorization": f"Bearer {api_key}"}
        _, response = self._request(
            "GET",
            "/auth/validate",
            extra_headers=headers,
        )
        return response.get("valid", False)
    except CloudError:
        return False
```

### 6.4 Refresh Logic

API keys typically don't expire, but the client handles re-authentication:

```python
class RefreshableClient:
    """Client with automatic re-authentication support."""
    
    def __init__(self):
        self._auth_manager = AuthenticationManager()
        self._token: Optional[str] = None
        self._token_expiry: Optional[datetime] = None
    
    def _get_valid_token(self) -> str:
        """Get a valid token, refreshing if necessary."""
        if self._token and self._token_expiry:
            if datetime.utcnow() < self._token_expiry:
                return self._token
        
        # Token expired or missing - get fresh
        self._token = self._auth_manager.get_api_key()
        
        if not self._token:
            raise AuthenticationError("No API key configured")
        
        # Assume 24-hour validity (or get from server)
        self._token_expiry = datetime.utcnow() + timedelta(hours=24)
        
        return self._token
    
    def _request_with_refresh(
        self,
        method: str,
        path: str,
        **kwargs,
    ) -> Any:
        """Execute request with automatic token refresh."""
        try:
            headers = {
                "Authorization": f"Bearer {self._get_valid_token()}",
            }
            headers.update(kwargs.get("extra_headers", {}))
            kwargs["extra_headers"] = headers
            
            return self._request(method, path, **kwargs)
            
        except AuthenticationError:
            # Token invalid - try to refresh
            self._token = None
            self._token_expiry = None
            
            # Retry with fresh token
            headers = {
                "Authorization": f"Bearer {self._get_valid_token()}",
            }
            headers.update(kwargs.get("extra_headers", {}))
            kwargs["extra_headers"] = headers
            
            return self._request(method, path, **kwargs)
```

---

## 7. Embedding System

### 7.1 Embedding Generation

The platform uses embeddings for semantic search:

```python
from sentence_transformers import SentenceTransformer

class EmbeddingClient:
    """Client for generating text embeddings."""
    
    DEFAULT_MODEL = "BAAI/bge-small-en-v1.5"
    DEFAULT_DIMENSION = 512
    
    def __init__(
        self,
        model_name: str = DEFAULT_MODEL,
        device: str = "cpu",
        cache_dir: Optional[Path] = None,
    ):
        self._model_name = model_name
        self._device = device
        self._model: Optional[SentenceTransformer] = None
        self._cache_dir = cache_dir or Path.home() / ".openspace" / "embeddings"
        self._cache_dir.mkdir(parents=True, exist_ok=True)
    
    def _get_model(self) -> SentenceTransformer:
        """Lazy-load the embedding model."""
        if self._model is None:
            self._model = SentenceTransformer(
                self._model_name,
                device=self._device,
                cache_folder=str(self._cache_dir),
            )
        return self._model
    
    def generate(self, text: str) -> List[float]:
        """Generate embedding for text.
        
        Args:
            text: Input text to embed
            
        Returns:
            List of floats (embedding vector)
        """
        model = self._get_model()
        embedding = model.encode(text, convert_to_numpy=True)
        return embedding.tolist()
    
    def generate_batch(
        self,
        texts: List[str],
        batch_size: int = 32,
        show_progress: bool = False,
    ) -> List[List[float]]:
        """Generate embeddings for multiple texts.
        
        Args:
            texts: List of texts to embed
            batch_size: Batch size for encoding
            show_progress: Show progress bar
            
        Returns:
            List of embedding vectors
        """
        model = self._get_model()
        embeddings = model.encode(
            texts,
            batch_size=batch_size,
            show_progress_bar=show_progress,
            convert_to_numpy=True,
        )
        return embeddings.tolist()
```

### 7.2 Embedding Cache

To avoid redundant API calls, embeddings are cached:

```python
class CachedEmbeddingClient:
    """Embedding client with persistent caching."""
    
    def __init__(
        self,
        api_key: Optional[str] = None,
        cache_dir: Optional[Path] = None,
    ):
        self._api_key = api_key
        self._cache_dir = cache_dir or Path.home() / ".openspace" / "embedding_cache"
        self._cache_dir.mkdir(parents=True, exist_ok=True)
        self._cache: Dict[str, List[float]] = {}
        self._load_cache()
    
    def _cache_key(self, text: str) -> str:
        """Generate cache key from text."""
        return hashlib.sha256(text.encode("utf-8")).hexdigest()
    
    def _load_cache(self) -> None:
        """Load cache from disk."""
        cache_file = self._cache_dir / "cache.json"
        if cache_file.exists():
            try:
                self._cache = json.loads(cache_file.read_text())
            except (json.JSONDecodeError, OSError):
                self._cache = {}
    
    def _save_cache(self) -> None:
        """Save cache to disk."""
        cache_file = self._cache_dir / "cache.json"
        cache_file.write_text(json.dumps(self._cache), encoding="utf-8")
    
    def generate(self, text: str) -> List[float]:
        """Generate embedding with caching."""
        key = self._cache_key(text)
        
        # Check in-memory cache
        if key in self._cache:
            return self._cache[key]
        
        # Generate new embedding
        if self._api_key:
            embedding = self._generate_api(text)
        else:
            embedding = self._generate_local(text)
        
        # Cache result
        self._cache[key] = embedding
        
        # Periodically save to disk
        if len(self._cache) % 100 == 0:
            self._save_cache()
        
        return embedding
    
    def _generate_api(self, text: str) -> List[float]:
        """Generate embedding using cloud API."""
        response = requests.post(
            "https://api.openai.com/v1/embeddings",
            headers={"Authorization": f"Bearer {self._api_key}"},
            json={
                "input": text,
                "model": "text-embedding-3-small",
            },
        )
        response.raise_for_status()
        return response.json()["data"][0]["embedding"]
    
    def _generate_local(self, text: str) -> List[float]:
        """Generate embedding using local model."""
        from sentence_transformers import SentenceTransformer
        model = SentenceTransformer("BAAI/bge-small-en-v1.5")
        embedding = model.encode(text)
        return embedding.tolist()
```

### 7.3 Similarity Search

Cosine similarity is used for matching:

```python
import math
from typing import List

def cosine_similarity(a: List[float], b: List[float]) -> float:
    """Compute cosine similarity between two vectors.
    
    Args:
        a: First vector
        b: Second vector
        
    Returns:
        Similarity score between -1 and 1
    """
    if len(a) != len(b) or not a:
        return 0.0
    
    dot_product = sum(x * y for x, y in zip(a, b))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(x * x for x in b))
    
    if norm_a == 0 or norm_b == 0:
        return 0.0
    
    return dot_product / (norm_a * norm_b)


def cosine_distance(a: List[float], b: List[float]) -> float:
    """Compute cosine distance (1 - similarity)."""
    return 1.0 - cosine_similarity(a, b)


def find_most_similar(
    query: List[float],
    candidates: List[List[float]],
    top_k: int = 10,
) -> List[tuple]:
    """Find most similar vectors to query.
    
    Args:
        query: Query vector
        candidates: List of candidate vectors
        top_k: Number of results to return
        
    Returns:
        List of (index, score) tuples sorted by similarity
    """
    scores = [
        (i, cosine_similarity(query, candidate))
        for i, candidate in enumerate(candidates)
    ]
    
    # Sort by score descending
    scores.sort(key=lambda x: x[1], reverse=True)
    
    return scores[:top_k]
```

### 7.4 Dimension Handling

Different embedding models produce different dimensions:

```python
EMBEDDING_DIMENSIONS = {
    "text-embedding-3-small": 1536,
    "text-embedding-3-large": 3072,
    "BAAI/bge-small-en-v1.5": 512,
    "BAAI/bge-base-en-v1.5": 768,
    "BAAI/bge-large-en-v1.5": 1024,
}


def normalize_embedding(
    embedding: List[float],
    target_dim: int,
) -> List[float]:
    """Normalize embedding to target dimension.
    
    If embedding is larger, truncate.
    If embedding is smaller, pad with zeros.
    """
    current_dim = len(embedding)
    
    if current_dim == target_dim:
        return embedding
    elif current_dim > target_dim:
        return embedding[:target_dim]
    else:
        return embedding + [0.0] * (target_dim - current_dim)


def get_embedding_dimension(model_name: str) -> int:
    """Get expected dimension for embedding model."""
    return EMBEDDING_DIMENSIONS.get(model_name, 512)
```

---

## 8. Group System

### 8.1 Group Overview

Groups enable private skill sharing within teams:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           GROUP ARCHITECTURE                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │   Group A    │    │   Group B    │    │   Group C    │              │
│  │  (Team Alpha)│    │  (Team Beta) │    │ (Research)   │              │
│  │              │    │              │    │              │              │
│  │ - Member 1   │    │ - Member 3   │    │ - Member 5   │              │
│  │ - Member 2   │    │ - Member 4   │    │ - Member 6   │              │
│  │ - Member 5   │    │              │    │ - Member 7   │              │
│  │              │    │              │    │              │              │
│  │ Skills:      │    │ Skills:      │    │ Skills:      │              │
│  │ - skill-a1   │    │ - skill-b1   │    │ - skill-r1   │              │
│  │ - skill-a2   │    │ - skill-b2   │    │ - skill-r2   │              │
│  │   (group)    │    │   (group)    │    │   (group)    │              │
│  └──────────────┘    └──────────────┘    └──────────────┘              │
│                                                                         │
│  Permission Levels:                                                     │
│  - owner: Full control (add/remove members, delete group)              │
│  - admin: Add/remove skills, invite members                           │
│  - member: View and use skills                                        │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Group Creation

```python
@dataclass
class GroupConfig:
    """Configuration for group creation."""
    name: str
    description: str = ""
    visibility: str = "private"  # "private", "internal"
    member_limit: int = 50


def create_group(
    self,
    config: GroupConfig,
) -> Dict[str, Any]:
    """Create a new group.
    
    Args:
        config: Group configuration
        
    Returns:
        Created group metadata
    """
    payload = {
        "name": config.name,
        "description": config.description,
        "visibility": config.visibility,
        "member_limit": config.member_limit,
    }
    
    _, response_body = self._request(
        "POST",
        "/groups/create",
        body=json.dumps(payload).encode("utf-8"),
        extra_headers={"Content-Type": "application/json"},
    )
    
    return json.loads(response_body.decode("utf-8"))


# Example response:
# {
#     "group_id": "grp_abc123",
#     "name": "Team Alpha",
#     "description": "Alpha team skills",
#     "owner_id": "user_xyz",
#     "created_at": "2024-01-15T10:30:00Z",
#     "member_count": 1,
#     "skill_count": 0,
# }
```

### 8.3 Member Management

```python
class GroupMemberManager:
    """Manages group membership."""
    
    def __init__(self, client: "OpenSpaceClient", group_id: str):
        self._client = client
        self._group_id = group_id
    
    def add_member(
        self,
        user_id: str,
        role: str = "member",
    ) -> Dict[str, Any]:
        """Add a member to the group.
        
        Args:
            user_id: User to add
            role: "owner", "admin", or "member"
            
        Returns:
            Updated group info
        """
        payload = {
            "user_id": user_id,
            "role": role,
        }
        
        return self._client._post(f"/groups/{self._group_id}/members", payload)
    
    def remove_member(self, user_id: str) -> Dict[str, Any]:
        """Remove a member from the group."""
        return self._client._delete(
            f"/groups/{self._group_id}/members/{user_id}"
        )
    
    def list_members(self) -> List[Dict[str, Any]]:
        """List all group members."""
        return self._client._get(f"/groups/{self._group_id}/members")
    
    def update_role(
        self,
        user_id: str,
        new_role: str,
    ) -> Dict[str, Any]:
        """Update a member's role."""
        payload = {"role": new_role}
        return self._client._patch(
            f"/groups/{self._group_id}/members/{user_id}",
            payload,
        )


# Permission levels:
MEMBER_ROLES = {
    "owner": {
        "can_delete_group": True,
        "can_transfer_ownership": True,
        "can_add_members": True,
        "can_remove_members": True,
        "can_add_skills": True,
        "can_remove_skills": True,
        "can_view_skills": True,
    },
    "admin": {
        "can_delete_group": False,
        "can_transfer_ownership": False,
        "can_add_members": True,
        "can_remove_members": True,
        "can_add_skills": True,
        "can_remove_skills": True,
        "can_view_skills": True,
    },
    "member": {
        "can_delete_group": False,
        "can_transfer_ownership": False,
        "can_add_members": False,
        "can_remove_members": False,
        "can_add_skills": False,
        "can_remove_skills": False,
        "can_view_skills": True,
    },
}
```

### 8.4 Skill Sharing Within Groups

```python
def upload_to_group(
    self,
    skill_dir: Path,
    group_id: str,
    **kwargs,
) -> str:
    """Upload a skill to a specific group.
    
    Args:
        skill_dir: Path to skill directory
        group_id: Target group ID
        **kwargs: Additional upload arguments
        
    Returns:
        record_id of created record
    """
    # Stage artifact
    artifact_id, file_count = self.stage_artifact(skill_dir)
    
    # Read skill metadata
    metadata = _read_upload_meta(skill_dir)
    skill_id = _read_skill_id(skill_dir)
    
    # Create record with group visibility
    record_id = f"{metadata.get('name', skill_id)}__clo_{uuid.uuid4().hex[:8]}"
    
    payload = {
        "record_id": record_id,
        "artifact_id": artifact_id,
        "skill_id": skill_id,
        "visibility": "group",
        "group_id": group_id,
        "origin": metadata.get("origin", "imported"),
        "parent_skill_ids": metadata.get("parent_skill_ids", []),
        "tags": metadata.get("tags", []),
        "level": "workflow",
    }
    
    if metadata.get("change_summary"):
        payload["change_summary"] = metadata["change_summary"]
    
    self.create_record(payload)
    
    return record_id


def get_group_skills(
    self,
    group_id: str,
    limit: int = 100,
) -> List[Dict[str, Any]]:
    """Get all skills shared with a group."""
    _, response_body = self._request(
        "GET",
        f"/groups/{group_id}/skills?limit={limit}",
    )
    return json.loads(response_body.decode("utf-8"))
```

### 8.5 Cross-Group Patterns

Skills can be shared across multiple groups:

```python
def share_skill_with_groups(
    self,
    record_id: str,
    group_ids: List[str],
) -> Dict[str, Any]:
    """Share an existing skill with multiple groups.
    
    Args:
        record_id: Existing skill record ID
        group_ids: List of group IDs to share with
        
    Returns:
        Sharing result
    """
    payload = {"group_ids": group_ids}
    
    _, response_body = self._request(
        "POST",
        f"/records/{record_id}/share",
        body=json.dumps(payload).encode("utf-8"),
        extra_headers={"Content-Type": "application/json"},
    )
    
    return json.loads(response_body.decode("utf-8"))


def get_shared_groups(
    self,
    record_id: str,
) -> List[Dict[str, Any]]:
    """Get groups that have access to a skill."""
    _, response_body = self._request(
        "GET",
        f"/records/{record_id}/shared-with",
    )
    return json.loads(response_body.decode("utf-8"))


# Cross-group skill visibility matrix:
# 
# Skill Visibility | Group A | Group B | Public Users
# -----------------|---------|---------|-------------
# public           | Yes     | Yes     | Yes
# private          | No      | No      | No
# group (A only)   | Yes     | No      | No
# group (A + B)    | Yes     | Yes     | No
```

---

## 9. API Reference

### 9.1 Endpoints Summary

| Method | Endpoint | Description |
|--------|----------|-------------|
| **Auth** |
| GET | `/auth/validate` | Validate API key |
| GET | `/auth/user` | Get current user info |
| **Records (Skills)** |
| GET | `/records/{record_id}` | Get skill record |
| POST | `/records` | Create skill record |
| DELETE | `/records/{record_id}` | Delete skill record |
| POST | `/records/embeddings/search` | Search by embedding |
| GET | `/records/{record_id}/shared-with` | Get sharing info |
| POST | `/records/{record_id}/share` | Share with groups |
| **Artifacts** |
| POST | `/artifacts/upload` | Upload skill artifact |
| GET | `/artifacts/{artifact_id}` | Download artifact |
| **Groups** |
| POST | `/groups/create` | Create group |
| GET | `/groups/{group_id}` | Get group info |
| GET | `/groups/{group_id}/members` | List members |
| POST | `/groups/{group_id}/members` | Add member |
| DELETE | `/groups/{group_id}/members/{user_id}` | Remove member |
| GET | `/groups/{group_id}/skills` | List group skills |

### 9.2 Error Codes

| Status Code | Meaning |
|-------------|---------|
| 200 | Success |
| 400 | Bad Request - Invalid payload |
| 401 | Unauthorized - Invalid/missing API key |
| 403 | Forbidden - Insufficient permissions |
| 404 | Not Found - Resource doesn't exist |
| 409 | Conflict - Resource already exists |
| 429 | Too Many Requests - Rate limited |
| 500 | Internal Server Error |
| 503 | Service Unavailable |

### 9.3 Rate Limits

| Tier | Requests/minute | Requests/day |
|------|-----------------|--------------|
| Free | 60 | 1,000 |
| Pro | 300 | 10,000 |
| Enterprise | 1,000 | Unlimited |

---

## Appendix: Complete Code Reference

### A.1 OpenSpaceClient Full Implementation

```python
"""OpenSpace Cloud Client - Complete Implementation."""

import asyncio
import io
import json
import logging
import os
import uuid
import zipfile
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import aiohttp

logger = logging.getLogger(__name__)


class CloudError(Exception):
    """Base exception for cloud errors."""
    
    def __init__(
        self,
        message: str,
        status_code: Optional[int] = None,
        original_error: Optional[Exception] = None,
    ):
        self.message = message
        self.status_code = status_code
        self.original_error = original_error
        super().__init__(self.message)


class OpenSpaceClient:
    """Client for the OpenSpace cloud platform."""
    
    def __init__(
        self,
        auth_headers: Dict[str, str],
        api_base: str = "https://open-space.cloud/api",
    ):
        self._headers = auth_headers
        self._base_url = api_base.rstrip("/")
        self._session: Optional[aiohttp.ClientSession] = None
        self._retry_delays = [1.0, 2.0, 4.0]
    
    async def _get_session(self) -> aiohttp.ClientSession:
        """Get or create aiohttp session."""
        if self._session is None or self._session.closed:
            self._session = aiohttp.ClientSession(
                headers=self._headers,
                timeout=aiohttp.ClientTimeout(total=30),
            )
        return self._session
    
    async def _request(
        self,
        method: str,
        path: str,
        body: Optional[bytes] = None,
        extra_headers: Optional[Dict[str, str]] = None,
        timeout: int = 30,
    ) -> Tuple[Dict[str, str], bytes]:
        """Execute HTTP request with retry logic."""
        url = f"{self._base_url}{path}"
        headers = dict(self._headers)
        
        if extra_headers:
            headers.update(extra_headers)
        
        last_error: Optional[Exception] = None
        
        for attempt, delay in enumerate(self._retry_delays + [None]):
            try:
                session = await self._get_session()
                
                async with session.request(
                    method,
                    url,
                    headers=headers,
                    data=body,
                    timeout=aiohttp.ClientTimeout(total=timeout),
                ) as response:
                    response_body = await response.read()
                    
                    if response.status >= 400:
                        raise CloudError(
                            f"API request failed: {method} {path} - {response.status}",
                            status_code=response.status,
                        )
                    
                    return dict(response.headers), response_body
                    
            except aiohttp.ClientError as e:
                last_error = e
                
                if delay is None:  # No more retries
                    break
                
                logger.warning(
                    f"Request failed (attempt {attempt + 1}), retrying in {delay}s: {e}"
                )
                await asyncio.sleep(delay)
        
        raise CloudError(
            f"Request failed after {len(self._retry_delays)} attempts",
            original_error=last_error,
        )
    
    # ==================== RECORD OPERATIONS ====================
    
    def create_record(
        self,
        payload: Dict[str, Any],
    ) -> Tuple[Dict[str, Any], int]:
        """Create a skill record.
        
        POST /records
        """
        loop = asyncio.get_event_loop()
        return loop.run_until_complete(self._create_record_async(payload))
    
    async def _create_record_async(
        self,
        payload: Dict[str, Any],
    ) -> Tuple[Dict[str, Any], int]:
        """Async record creation."""
        headers, body = await self._request(
            "POST",
            "/records",
            body=json.dumps(payload).encode("utf-8"),
            extra_headers={"Content-Type": "application/json"},
        )
        
        status = int(headers.get("X-Status-Code", 200))
        return json.loads(body.decode("utf-8")), status
    
    def fetch_record(self, record_id: str) -> Dict[str, Any]:
        """Fetch a skill record by ID.
        
        GET /records/{record_id}
        """
        loop = asyncio.get_event_loop()
        return loop.run_until_complete(self._fetch_record_async(record_id))
    
    async def _fetch_record_async(self, record_id: str) -> Dict[str, Any]:
        """Async record fetch."""
        _, body = await self._request("GET", f"/records/{record_id}")
        return json.loads(body.decode("utf-8"))
    
    def search_record_embeddings(
        self,
        query: str,
        limit: int = 300,
        level: Optional[str] = None,
        tags: Optional[List[str]] = None,
    ) -> List[Dict[str, Any]]:
        """Search skills by embedding similarity.
        
        POST /records/embeddings/search
        """
        loop = asyncio.get_event_loop()
        return loop.run_until_complete(
            self._search_embeddings_async(query, limit, level, tags)
        )
    
    async def _search_embeddings_async(
        self,
        query: str,
        limit: int,
        level: Optional[str],
        tags: Optional[List[str]],
    ) -> List[Dict[str, Any]]:
        """Async embedding search."""
        payload = {"query": query, "limit": limit}
        
        if level:
            payload["level"] = level
        if tags:
            payload["tags"] = tags
        
        _, body = await self._request(
            "POST",
            "/records/embeddings/search",
            body=json.dumps(payload).encode("utf-8"),
            extra_headers={"Content-Type": "application/json"},
            timeout=30,
        )
        
        return json.loads(body.decode("utf-8"))
    
    # ==================== ARTIFACT OPERATIONS ====================
    
    def stage_artifact(self, skill_dir: Path) -> Tuple[str, int]:
        """Stage skill files as an artifact.
        
        Returns:
            Tuple of (artifact_id, file_count)
        """
        loop = asyncio.get_event_loop()
        return loop.run_until_complete(self._stage_artifact_async(skill_dir))
    
    async def _stage_artifact_async(
        self,
        skill_dir: Path,
    ) -> Tuple[str, int]:
        """Async artifact staging."""
        # Collect files
        files: List[Tuple[str, bytes]] = []
        
        for file_path in skill_dir.rglob("*"):
            if file_path.is_file() and not file_path.name.startswith("."):
                relative_path = file_path.relative_to(skill_dir)
                content = file_path.read_bytes()
                files.append((str(relative_path), content))
        
        # Create zip
        zip_buffer = io.BytesIO()
        with zipfile.ZipFile(zip_buffer, "w", zipfile.ZIP_DEFLATED) as zf:
            for path, content in files:
                zf.writestr(path, content)
        
        zip_data = zip_buffer.getvalue()
        
        # Upload
        artifact_id = await self._upload_artifact(zip_data)
        
        return artifact_id, len(files)
    
    async def _upload_artifact(self, zip_data: bytes) -> str:
        """Upload artifact to server."""
        boundary = f"----WebKitFormBoundary{uuid.uuid4().hex}"
        
        body = self._build_multipart_body(boundary, zip_data, "skill.zip")
        
        _, response_body = await self._request(
            "POST",
            "/artifacts/upload",
            body=body,
            extra_headers={
                "Content-Type": f"multipart/form-data; boundary={boundary}",
            },
        )
        
        response = json.loads(response_body.decode("utf-8"))
        return response["artifact_id"]
    
    def _build_multipart_body(
        self,
        boundary: str,
        file_data: bytes,
        file_name: str,
    ) -> bytes:
        """Build multipart form body."""
        lines = [
            f"--{boundary}".encode(),
            f'Content-Disposition: form-data; name="file"; filename="{file_name}"'.encode(),
            b"Content-Type: application/zip",
            b"",
            file_data,
            b"",
            f"--{boundary}--".encode(),
        ]
        return b"\r\n".join(lines)
    
    def download_artifact(self, record_id: str) -> bytes:
        """Download skill artifact.
        
        GET /artifacts/{record_id}
        """
        loop = asyncio.get_event_loop()
        return loop.run_until_complete(self._download_artifact_async(record_id))
    
    async def _download_artifact_async(self, record_id: str) -> bytes:
        """Async artifact download."""
        _, body = await self._request("GET", f"/artifacts/{record_id}")
        return body
    
    # ==================== IMPORT/EXPORT ====================
    
    def import_skill(
        self,
        skill_id: str,
        target_dir: Path,
    ) -> Dict[str, Any]:
        """Download and extract a skill locally."""
        loop = asyncio.get_event_loop()
        return loop.run_until_complete(
            self._import_skill_async(skill_id, target_dir)
        )
    
    async def _import_skill_async(
        self,
        skill_id: str,
        target_dir: Path,
    ) -> Dict[str, Any]:
        """Async skill import."""
        # Fetch metadata
        record_data = await self._fetch_record_async(skill_id)
        skill_name = record_data.get("name", skill_id)
        
        # Sanitize name
        if "/" in skill_name or "\\" in skill_name or skill_name.startswith("."):
            skill_name = skill_id
        
        skill_dir = (target_dir / skill_name).resolve()
        
        if not skill_dir.is_relative_to(target_dir.resolve()):
            raise CloudError(f"Skill name {skill_name!r} escapes target directory")
        
        # Check if exists
        if skill_dir.exists() and (skill_dir / "SKILL.md").exists():
            return {
                "status": "already_exists",
                "skill_id": skill_id,
                "name": skill_name,
                "local_path": str(skill_dir),
            }
        
        # Download artifact
        zip_data = await self._download_artifact_async(skill_id)
        
        # Extract
        skill_dir.mkdir(parents=True, exist_ok=True)
        extracted = self._extract_zip(zip_data, skill_dir)
        
        # Write .skill_id
        (skill_dir / ".skill_id").write_text(skill_id + "\n", encoding="utf-8")
        
        return {
            "status": "success",
            "skill_id": skill_id,
            "name": skill_name,
            "description": record_data.get("description", ""),
            "local_path": str(skill_dir),
            "files": extracted,
        }
    
    def _extract_zip(
        self,
        zip_data: bytes,
        target_dir: Path,
    ) -> List[str]:
        """Extract zip to directory."""
        extracted = []
        
        with zipfile.ZipFile(io.BytesIO(zip_data), "r") as zf:
            for info in zf.infolist():
                if not self._is_safe_path(target_dir, info.filename):
                    continue
                
                output_path = target_dir / info.filename
                
                if info.is_dir():
                    output_path.mkdir(parents=True, exist_ok=True)
                else:
                    output_path.parent.mkdir(parents=True, exist_ok=True)
                    output_path.write_bytes(zf.read(info))
                    extracted.append(str(output_path))
        
        return extracted
    
    def _is_safe_path(self, base_dir: Path, file_path: str) -> bool:
        """Check for path traversal."""
        try:
            resolved = (base_dir / file_path).resolve()
            return resolved.is_relative_to(base_dir.resolve())
        except ValueError:
            return False


# ==================== AUTHENTICATION HELPERS ====================

class AuthenticationManager:
    """Manages API key storage and retrieval."""
    
    def __init__(self):
        self._key_file = Path.home() / ".openspace" / "api_key"
        self._env_var = "OPENSPACE_API_KEY"
    
    def get_api_key(self) -> Optional[str]:
        """Get API key from environment or file."""
        # Check environment
        env_key = os.environ.get(self._env_var)
        if env_key:
            return env_key.strip()
        
        # Check file
        if self._key_file.exists():
            try:
                return self._key_file.read_text(encoding="utf-8").strip()
            except OSError:
                pass
        
        return None
    
    def set_api_key(self, api_key: str, persist: bool = True) -> None:
        """Store API key."""
        if persist:
            self._key_file.parent.mkdir(parents=True, exist_ok=True)
            self._key_file.write_text(api_key, encoding="utf-8")
            self._key_file.chmod(0o600)
        
        os.environ[self._env_var] = api_key
    
    def clear_api_key(self) -> None:
        """Remove stored API key."""
        if self._key_file.exists():
            self._key_file.unlink()
        os.environ.pop(self._env_var, None)


def get_openspace_auth() -> Tuple[Optional[Dict[str, str]], str]:
    """Get authentication headers and API base URL."""
    auth_manager = AuthenticationManager()
    api_key = auth_manager.get_api_key()
    
    if not api_key:
        return None, "https://open-space.cloud/api"
    
    headers = {"Authorization": f"Bearer {api_key}"}
    
    api_base = os.environ.get(
        "OPENSPACE_API_BASE",
        "https://open-space.cloud/api",
    )
    
    return headers, api_base
```

### A.2 Embedding Client Implementation

```python
"""Embedding Client - Complete Implementation."""

import hashlib
import json
import math
from pathlib import Path
from typing import Dict, List, Optional

try:
    from sentence_transformers import SentenceTransformer
    HAS_SENTENCE_TRANSFORMERS = True
except ImportError:
    HAS_SENTENCE_TRANSFORMERS = False


class EmbeddingClient:
    """Client for generating text embeddings."""
    
    DEFAULT_MODEL = "BAAI/bge-small-en-v1.5"
    DEFAULT_DIMENSION = 512
    
    def __init__(
        self,
        model_name: str = DEFAULT_MODEL,
        device: str = "cpu",
        cache_dir: Optional[Path] = None,
    ):
        self._model_name = model_name
        self._device = device
        self._model: Optional[SentenceTransformer] = None
        self._cache_dir = cache_dir or Path.home() / ".openspace" / "embeddings"
        self._cache_dir.mkdir(parents=True, exist_ok=True)
    
    def _get_model(self) -> "SentenceTransformer":
        """Lazy-load the embedding model."""
        if self._model is None:
            if not HAS_SENTENCE_TRANSFORMERS:
                raise ImportError(
                    "sentence-transformers is required. "
                    "Install with: pip install sentence-transformers"
                )
            self._model = SentenceTransformer(
                self._model_name,
                device=self._device,
                cache_folder=str(self._cache_dir),
            )
        return self._model
    
    def generate(self, text: str) -> List[float]:
        """Generate embedding for text."""
        model = self._get_model()
        embedding = model.encode(text, convert_to_numpy=True)
        return embedding.tolist()
    
    def generate_batch(
        self,
        texts: List[str],
        batch_size: int = 32,
        show_progress: bool = False,
    ) -> List[List[float]]:
        """Generate embeddings for multiple texts."""
        model = self._get_model()
        embeddings = model.encode(
            texts,
            batch_size=batch_size,
            show_progress_bar=show_progress,
            convert_to_numpy=True,
        )
        return embeddings.tolist()


class CachedEmbeddingClient:
    """Embedding client with persistent caching."""
    
    def __init__(
        self,
        api_key: Optional[str] = None,
        cache_dir: Optional[Path] = None,
    ):
        self._api_key = api_key
        self._cache_dir = cache_dir or Path.home() / ".openspace" / "embedding_cache"
        self._cache_dir.mkdir(parents=True, exist_ok=True)
        self._cache: Dict[str, List[float]] = {}
        self._load_cache()
    
    def _cache_key(self, text: str) -> str:
        """Generate cache key from text."""
        return hashlib.sha256(text.encode("utf-8")).hexdigest()
    
    def _load_cache(self) -> None:
        """Load cache from disk."""
        cache_file = self._cache_dir / "cache.json"
        if cache_file.exists():
            try:
                self._cache = json.loads(cache_file.read_text())
            except (json.JSONDecodeError, OSError):
                self._cache = {}
    
    def _save_cache(self) -> None:
        """Save cache to disk."""
        cache_file = self._cache_dir / "cache.json"
        cache_file.write_text(json.dumps(self._cache), encoding="utf-8")
    
    def generate(self, text: str) -> List[float]:
        """Generate embedding with caching."""
        key = self._cache_key(text)
        
        if key in self._cache:
            return self._cache[key]
        
        # Generate new
        if self._api_key:
            embedding = self._generate_api(text)
        else:
            embedding = self._generate_local(text)
        
        self._cache[key] = embedding
        
        if len(self._cache) % 100 == 0:
            self._save_cache()
        
        return embedding
    
    def _generate_api(self, text: str) -> List[float]:
        """Generate using OpenAI API."""
        import requests
        
        response = requests.post(
            "https://api.openai.com/v1/embeddings",
            headers={"Authorization": f"Bearer {self._api_key}"},
            json={
                "input": text,
                "model": "text-embedding-3-small",
            },
        )
        response.raise_for_status()
        return response.json()["data"][0]["embedding"]
    
    def _generate_local(self, text: str) -> List[float]:
        """Generate using local model."""
        client = EmbeddingClient()
        return client.generate(text)


def cosine_similarity(a: List[float], b: List[float]) -> float:
    """Compute cosine similarity."""
    if len(a) != len(b) or not a:
        return 0.0
    
    dot = sum(x * y for x, y in zip(a, b))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(x * x for x in b))
    
    if norm_a == 0 or norm_b == 0:
        return 0.0
    
    return dot / (norm_a * norm_b)


def find_similar(
    query: List[float],
    candidates: List[List[float]],
    top_k: int = 10,
) -> List[tuple]:
    """Find most similar candidates."""
    scores = [
        (i, cosine_similarity(query, cand))
        for i, cand in enumerate(candidates)
    ]
    scores.sort(key=lambda x: x[1], reverse=True)
    return scores[:top_k]
```
