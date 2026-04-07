# Session Teleportation (CCR) Deep Dive

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/utils/teleport*`

**Related Modules:** `utils/teleport/`, `hooks/useTeleportResume.tsx`, `components/TeleportProgress.tsx`, `components/TeleportError.tsx`

---

## Module Overview

The Session Teleportation system (internally called **CCR** - Cross-Computer Resume) enables users to seamlessly resume Claude Code sessions across different machines. The system captures session state from one machine and reconstructs it on another, including:

- **Conversation history** - All messages, tool calls, and responses
- **Git context** - Repository state, current branch, uncommitted changes
- **Working directory** - File paths and environment context
- **Environment configuration** - Remote environment provisioning

**Key Architecture Principles:**
1. **Session-as-Source** - Remote sessions stored in Claude.ai cloud with full transcript
2. **Git Bundle Seeding** - Repository state transferred via git bundles with 3-tier fallback
3. **Repository Validation** - Prevents session resume in wrong repository
4. **Progressive Disclosure** - UI shows step-by-step teleport progress
5. **OAuth Authentication** - Bearer token auth for Claude.ai API access

---

## Architecture Diagram

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Local CLI     │     │  Claude.ai API   │     │  Remote Session │
│                 │     │                  │     │  (Cloud Env)    │
│  teleport.tsx   │────▶│  /v1/sessions    │────▶│                 │
│  - validate     │     │  /v1/session_ingress│  │  - Environment  │
│  - fetch logs   │     │  /v1/files       │     │  - Git checkout │
│  - checkout     │     │  (Files API)     │     │  - Session ctx  │
└─────────────────┘     └──────────────────┘     └─────────────────┘
         │                       │                        │
         │ 1. fetchSession()     │                        │
         │ 2. validateRepo()     │                        │
         │ 3. getTeleportEvents()│                        │
         │ 4. checkoutBranch()   │                        │
         │ 5. processMessages()  │                        │
         ▼                       ▼                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Git Bundle Flow (CCR)                        │
│                                                                 │
│  Local Repo → git bundle create → Files API → Remote Session   │
│  (3-tier: --all → HEAD → squashed-root)                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Directory Structure

### Core Teleport Files

| File | Lines | Purpose |
|------|-------|---------|
| `utils/teleport.tsx` | ~900 | Main teleport orchestration, validation, branch checkout |
| `utils/teleport/api.ts` | ~350 | Session API types, OAuth headers, retry logic |
| `utils/teleport/gitBundle.ts` | ~290 | Git bundle creation with 3-tier fallback |
| `utils/teleport/environments.ts` | ~90 | Environment fetching and creation |
| `utils/teleport/environmentSelection.ts` | ~75 | Environment selection UI logic |
| `hooks/useTeleportResume.tsx` | ~150 | React hook for teleport state management |
| `components/TeleportProgress.tsx` | ~200 | Progress UI with animated spinner |
| `components/TeleportError.tsx` | ~250 | Error dialog for teleport failures |
| `utils/conversationRecovery.ts` | ~180 | Message deserialization, teleport resume types |

---

## Key Components

### 1. Main Teleport Function (`utils/teleport.tsx`)

The `teleportResumeCodeSession()` function is the primary entry point for resuming a remote session.

```typescript
export async function teleportResumeCodeSession(
  sessionId: string, 
  onProgress?: TeleportProgressCallback
): Promise<TeleportRemoteResponse> {
  if (!isPolicyAllowed('allow_remote_sessions')) {
    throw new Error("Remote sessions are disabled by your organization's policy.");
  }
  
  logForDebugging(`Resuming code session ID: ${sessionId}`);
  
  try {
    const accessToken = getClaudeAIOAuthTokens()?.accessToken;
    if (!accessToken) {
      logEvent('tengu_teleport_resume_error', {
        error_type: 'no_access_token' as AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS
      });
      throw new Error(
        'Claude Code web sessions require authentication with a Claude.ai account. ' +
        'API key authentication is not sufficient.'
      );
    }

    // Get organization UUID for API calls
    const orgUUID = await getOrganizationUUID();
    if (!orgUUID) {
      throw new Error('Unable to get organization UUID for constructing session URL');
    }

    // Fetch and validate repository matches before resuming
    onProgress?.('validating');
    const sessionData = await fetchSession(sessionId);
    const repoValidation = await validateSessionRepository(sessionData);
    
    // Switch handles: match, no_repo_required, not_in_repo, mismatch, error
    switch (repoValidation.status) {
      case 'match':
      case 'no_repo_required':
        // Proceed with teleport
        break;
      case 'not_in_repo':
        throw new TeleportOperationError(
          `You must run claude --teleport ${sessionId} from a checkout of ${repoValidation.sessionRepo}.`,
          chalk.red(`You must run claude --teleport ${sessionId} from a checkout of ${chalk.bold(repoValidation.sessionRepo)}.\n`)
        );
      case 'mismatch':
        // Handle cross-instance mismatches (GHE vs github.com)
        const hostsDiffer = repoValidation.sessionHost && repoValidation.currentHost && 
          stripPort(repoValidation.sessionHost) !== stripPort(repoValidation.currentHost);
        const sessionDisplay = hostsDiffer 
          ? `${repoValidation.sessionHost}/${repoValidation.sessionRepo}` 
          : repoValidation.sessionRepo;
        throw new TeleportOperationError(
          `You must run claude --teleport ${sessionId} from a checkout of ${sessionDisplay}.`,
          chalk.red(`You must run claude --teleport ${sessionId} from a checkout of ${chalk.bold(sessionDisplay)}.\n`)
        );
      case 'error':
        throw new TeleportOperationError(
          repoValidation.errorMessage || 'Failed to validate session repository',
          chalk.red(`Error: ${repoValidation.errorMessage || 'Failed to validate session repository'}\n`)
        );
    }
    
    return await teleportFromSessionsAPI(sessionId, orgUUID, accessToken, onProgress, sessionData);
  } catch (error) {
    if (error instanceof TeleportOperationError) {
      throw error;
    }
    const err = toError(error);
    logError(err);
    throw new TeleportOperationError(err.message, chalk.red(`Error: ${err.message}\n`));
  }
}
```

**Key Design Decisions:**
1. **OAuth Required** - API keys insufficient; must have Claude.ai Bearer token
2. **Org UUID** - Required for constructing session URLs
3. **Repo Validation Before Fetch** - Fail fast if wrong repository
4. **Host Matching** - Prevents GHE/github.com confusion

---

### 2. Repository Validation (`utils/teleport.tsx`)

Validates that the local git repository matches the session's repository.

```typescript
export async function validateSessionRepository(
  sessionData: SessionResource
): Promise<RepoValidationResult> {
  const currentParsed = await detectCurrentRepositoryWithHost();
  const currentRepo = currentParsed 
    ? `${currentParsed.owner}/${currentParsed.name}` 
    : null;
  
  const gitSource = sessionData.session_context.sources.find(
    (source): source is GitSource => source.type === 'git_repository'
  );
  
  if (!gitSource?.url) {
    // Session has no repo requirement
    return { status: 'no_repo_required' };
  }
  
  const sessionParsed = parseGitRemote(gitSource.url);
  const sessionRepo = sessionParsed 
    ? `${sessionParsed.owner}/${sessionParsed.name}` 
    : parseGitHubRepository(gitSource.url);
  
  if (!sessionRepo) {
    return { status: 'no_repo_required' };
  }
  
  // Compare both owner/repo AND host to avoid cross-instance mismatches
  // Strip ports before comparing — SSH remotes omit port, HTTPS may include non-standard port
  const stripPort = (host: string): string => host.replace(/:\d+$/, '');
  const repoMatch = currentRepo.toLowerCase() === sessionRepo.toLowerCase();
  const hostMatch = !currentParsed || !sessionParsed || 
    stripPort(currentParsed.host.toLowerCase()) === stripPort(sessionParsed.host.toLowerCase());
  
  if (repoMatch && hostMatch) {
    return { status: 'match', sessionRepo, currentRepo };
  }
  
  // Repo mismatch — include host info for GHE users
  return {
    status: 'mismatch',
    sessionRepo,
    currentRepo,
    sessionHost: sessionParsed?.host,
    currentHost: currentParsed?.host
  };
}
```

**Validation Cases:**
| Status | Description | User Action |
|--------|-------------|-------------|
| `match` | Repos and hosts match | Proceed |
| `no_repo_required` | Session has no git context | Proceed |
| `not_in_repo` | User not in git directory | cd to repo |
| `mismatch` | Different repos or hosts | cd to correct repo |
| `error` | Validation failed | Fix git state |

---

### 3. Git Bundle Creation (`utils/teleport/gitBundle.ts`)

Creates a git bundle of the local repository for transfer to the remote session. Uses a **3-tier fallback strategy** to handle repos of different sizes.

```typescript
async function _bundleWithFallback(
  gitRoot: string,
  bundlePath: string,
  maxBytes: number,
  hasStash: boolean,
  signal: AbortSignal | undefined,
): Promise<BundleCreateResult> {
  // Include stash ref if WIP captured
  const extra = hasStash ? ['refs/seed/stash'] : [];
  
  const mkBundle = (base: string) =>
    execFileNoThrowWithCwd(
      gitExe(),
      ['bundle', 'create', bundlePath, base, ...extra],
      { cwd: gitRoot, abortSignal: signal }
    );

  // === TIER 1: --all (full history, all branches) ===
  const allResult = await mkBundle('--all');
  if (allResult.code !== 0) {
    return {
      ok: false,
      error: `git bundle create --all failed (${allResult.code}): ${allResult.stderr.slice(0, 200)}`,
      failReason: 'git_error'
    };
  }
  
  const { size: allSize } = await stat(bundlePath);
  if (allSize <= maxBytes) {
    return { ok: true, size: allSize, scope: 'all' };
  }

  // === TIER 2: HEAD (current branch only, full history) ===
  logForDebugging(
    `[gitBundle] --all bundle is ${(allSize / 1024 / 1024).toFixed(1)}MB (> ${(maxBytes / 1024 / 1024).toFixed(0)}MB), retrying HEAD-only`
  );
  const headResult = await mkBundle('HEAD');
  if (headResult.code !== 0) {
    return {
      ok: false,
      error: `git bundle create HEAD failed (${headResult.code}): ${headResult.stderr.slice(0, 200)}`,
      failReason: 'git_error'
    };
  }
  
  const { size: headSize } = await stat(bundlePath);
  if (headSize <= maxBytes) {
    return { ok: true, size: headSize, scope: 'head' };
  }

  // === TIER 3: squashed-root (single commit, no history) ===
  logForDebugging(
    `[gitBundle] HEAD bundle is ${(headSize / 1024 / 1024).toFixed(1)}MB, retrying squashed-root`
  );
  
  // Create single commit from tree (stash tree if WIP exists)
  const treeRef = hasStash ? 'refs/seed/stash^{tree}' : 'HEAD^{tree}';
  const commitTree = await execFileNoThrowWithCwd(
    gitExe(),
    ['commit-tree', treeRef, '-m', 'seed'],
    { cwd: gitRoot, abortSignal: signal }
  );
  
  if (commitTree.code !== 0) {
    return {
      ok: false,
      error: `git commit-tree failed (${commitTree.code}): ${commitTree.stderr.slice(0, 200)}`,
      failReason: 'git_error'
    };
  }
  
  const squashedSha = commitTree.stdout.trim();
  await execFileNoThrowWithCwd(
    gitExe(),
    ['update-ref', 'refs/seed/root', squashedSha],
    { cwd: gitRoot }
  );
  
  const squashResult = await mkBundle('refs/seed/root');
  if (squashResult.code !== 0) {
    return {
      ok: false,
      error: `git bundle create refs/seed/root failed`,
      failReason: 'git_error'
    };
  }
  
  const { size: squashSize } = await stat(bundlePath);
  if (squashSize <= maxBytes) {
    return { ok: true, size: squashSize, scope: 'squashed' };
  }

  return {
    ok: false,
    error: 'Repo is too large to bundle. Please setup GitHub on https://claude.ai/code',
    failReason: 'too_large'
  };
}
```

**3-Tier Fallback Strategy:**
| Tier | Scope | Size Limit | Use Case |
|------|-------|------------|----------|
| `--all` | All branches, tags, history | 100MB (default) | Small repos, full fidelity |
| `HEAD` | Current branch, full history | 100MB | Medium repos, current work |
| `squashed-root` | Single commit, no history | 100MB | Large repos, snapshot only |

**WIP (Work in Progress) Handling:**
```typescript
// git stash create writes a dangling commit — doesn't touch refs/stash
const stashResult = await execFileNoThrowWithCwd(
  gitExe(),
  ['stash', 'create'],
  { cwd: gitRoot, abortSignal: opts?.signal }
);

const wipStashSha = stashResult.code === 0 ? stashResult.stdout.trim() : '';
const hasWip = wipStashSha !== '';

if (hasWip) {
  // Make ref reachable so bundle includes it
  await execFileNoThrowWithCwd(
    gitExe(),
    ['update-ref', 'refs/seed/stash', wipStashSha],
    { cwd: gitRoot }
  );
}
```

---

### 4. Session Log Fetching (`utils/teleport.tsx`)

Fetches session transcript from the Session Ingress API.

```typescript
export async function teleportFromSessionsAPI(
  sessionId: string,
  orgUUID: string,
  accessToken: string,
  onProgress?: TeleportProgressCallback,
  sessionData?: SessionResource
): Promise<TeleportRemoteResponse> {
  const startTime = Date.now();
  
  try {
    logForDebugging(`[teleport] Starting fetch for session: ${sessionId}`);
    onProgress?.('fetching_logs');
    
    const logsStartTime = Date.now();
    
    // Try CCR v2 first (GetTeleportEvents — server dispatches Spanner/threadstore)
    // Fall back to session-ingress if it returns null
    let logs = await getTeleportEvents(sessionId, accessToken, orgUUID);
    
    if (logs === null) {
      logForDebugging('[teleport] v2 endpoint returned null, trying session-ingress');
      logs = await getSessionLogsViaOAuth(sessionId, accessToken, orgUUID);
    }
    
    logForDebugging(`[teleport] Session logs fetched in ${Date.now() - logsStartTime}ms`);
    
    if (logs === null) {
      throw new Error('Failed to fetch session logs');
    }

    // Filter to get only transcript messages, excluding sidechain messages
    const filterStartTime = Date.now();
    const messages = logs.filter(
      entry => isTranscriptMessage(entry) && !entry.isSidechain
    ) as Message[];
    
    logForDebugging(
      `[teleport] Filtered ${logs.length} entries to ${messages.length} messages in ${Date.now() - filterStartTime}ms`
    );

    // Extract branch info from session data
    onProgress?.('fetching_branch');
    const branch = sessionData ? getBranchFromSession(sessionData) : undefined;
    
    if (branch) {
      logForDebugging(`[teleport] Found branch: ${branch}`);
    }
    
    logForDebugging(`[teleport] Total teleportFromSessionsAPI time: ${Date.now() - startTime}ms`);
    
    return {
      log: messages,
      branch
    };
  } catch (error) {
    if (axios.isAxiosError(error) && error.response?.status === 404) {
      logEvent('tengu_teleport_error_session_not_found_404', {
        sessionId: sessionId as AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS
      });
      throw new TeleportOperationError(
        `${sessionId} not found.`,
        `${sessionId} not found.\n${chalk.dim('Run /status in Claude Code to check your account.')}`
      );
    }
    throw new Error(`Failed to fetch session from Sessions API: ${toError(error).message}`);
  }
}
```

**API Retry Logic:**
- CCR v2 (`getTeleportEvents`) tried first
- Falls back to session-ingress (`getSessionLogsViaOAuth`)
- 404 handled specially with helpful error message

---

### 5. Branch Checkout (`utils/teleport.tsx`)

Handles checking out the session's branch with multi-strategy approach.

```typescript
async function checkoutBranch(branchName: string): Promise<void> {
  // First try to checkout the branch as-is (might be local)
  let { code: checkoutCode, stderr: checkoutStderr } = 
    await execFileNoThrow(gitExe(), ['checkout', branchName]);

  // If that fails, try to checkout from origin
  if (checkoutCode !== 0) {
    logForDebugging(`Local checkout failed, trying to checkout from origin: ${checkoutStderr}`);
    
    // Try to checkout the remote branch and create a local tracking branch
    const result = await execFileNoThrow(
      gitExe(), 
      ['checkout', '-b', branchName, '--track', `origin/${branchName}`]
    );
    checkoutCode = result.code;
    checkoutStderr = result.stderr;

    // If that also fails, try without -b in case the branch exists but isn't checked out
    if (checkoutCode !== 0) {
      logForDebugging(`Remote checkout with -b failed, trying without -b: ${checkoutStderr}`);
      const finalResult = await execFileNoThrow(
        gitExe(), 
        ['checkout', '--track', `origin/${branchName}`]
      );
      checkoutCode = finalResult.code;
      checkoutStderr = finalResult.stderr;
    }
  }
  
  if (checkoutCode !== 0) {
    logEvent('tengu_teleport_error_branch_checkout_failed', {});
    throw new TeleportOperationError(
      `Failed to checkout branch '${branchName}': ${checkoutStderr}`,
      chalk.red(`Failed to checkout branch '${branchName}'\n`)
    );
  }

  // After successful checkout, ensure upstream is set
  await ensureUpstreamIsSet(branchName);
}

async function ensureUpstreamIsSet(branchName: string): Promise<void> {
  // Check if upstream is already set
  const { code: upstreamCheckCode } = await execFileNoThrow(
    gitExe(), 
    ['rev-parse', '--abbrev-ref', `${branchName}@{upstream}`]
  );
  
  if (upstreamCheckCode === 0) {
    logForDebugging(`Branch '${branchName}' already has upstream set`);
    return;
  }

  // Check if origin/<branchName> exists
  const { code: remoteCheckCode } = await execFileNoThrow(
    gitExe(), 
    ['rev-parse', '--verify', `origin/${branchName}`]
  );
  
  if (remoteCheckCode === 0) {
    logForDebugging(`Setting upstream for '${branchName}' to 'origin/${branchName}'`);
    const { code: setUpstreamCode } = await execFileNoThrow(
      gitExe(), 
      ['branch', '--set-upstream-to', `origin/${branchName}`, branchName]
    );
    if (setUpstreamCode !== 0) {
      logForDebugging(`Failed to set upstream for '${branchName}': ${setUpstreamStderr}`);
    }
  }
}
```

**Checkout Strategy:**
1. Local checkout (`git checkout branch`)
2. Create tracking branch (`git checkout -b branch --track origin/branch`)
3. Attach to existing branch (`git checkout --track origin/branch`)
4. Set upstream if not set

---

### 6. Message Processing (`utils/teleport.tsx`)

Processes messages for teleport resume, handling incomplete tool calls.

```typescript
export function processMessagesForTeleportResume(
  messages: Message[], 
  error: Error | null
): Message[] {
  // Deserialize messages (shared logic with resume)
  const deserializedMessages = deserializeMessages(messages);

  // Add user message about teleport resume (visible to model)
  const messagesWithTeleportNotice = [
    ...deserializedMessages, 
    createTeleportResumeUserMessage(),
    createTeleportResumeSystemMessage(error)
  ];
  
  return messagesWithTeleportNotice;
}

function createTeleportResumeUserMessage() {
  return createUserMessage({
    content: `This session is being continued from another machine. Application state may have changed. The updated working directory is ${getOriginalCwd()}`,
    isMeta: true
  });
}

function createTeleportResumeSystemMessage(branchError: Error | null) {
  if (branchError === null) {
    return createSystemMessage('Session resumed', 'suggestion');
  }
  const formattedError = branchError instanceof TeleportOperationError 
    ? branchError.formattedMessage 
    : branchError.message;
  return createSystemMessage(
    `Session resumed without branch: ${formattedError}`, 
    'warning'
  );
}
```

---

### 7. React Hook (`hooks/useTeleportResume.tsx`)

Manages teleport state in React components.

```typescript
export function useTeleportResume(source: TeleportSource) {
  const [isResuming, setIsResuming] = useState(false);
  const [error, setError] = useState<TeleportResumeError | null>(null);
  const [selectedSession, setSelectedSession] = useState<CodeSession | null>(null);
  
  const resumeSession = useCallback(async (session: CodeSession) => {
    setIsResuming(true);
    setError(null);
    
    logEvent("tengu_teleport_resume_session", { 
      source, 
      session_id: session.id 
    });
    
    try {
      const result = await teleportResumeCodeSession(session.id);
      setTeleportedSessionInfo({ sessionId: session.id });
      return result;
    } catch (err) {
      setError(mapTeleportError(err));
      throw err;
    } finally {
      setIsResuming(false);
    }
  }, [source]);
  
  return {
    isResuming,
    error,
    selectedSession,
    setSelectedSession,
    resumeSession
  };
}
```

---

### 8. Progress UI (`components/TeleportProgress.tsx`)

Renders animated progress indicator during teleport.

```typescript
const STEPS = [
  { key: 'validating', label: 'Validating session' },
  { key: 'fetching_logs', label: 'Fetching session logs' },
  { key: 'fetching_branch', label: 'Getting branch info' },
  { key: 'checking_out', label: 'Checking out branch' }
];

const SPINNER_FRAMES = ['◐', '◓', '◑', '◒'];

export async function teleportWithProgress(
  root: Root, 
  sessionId: string
): Promise<TeleportResult> {
  let setStep: (step: TeleportProgressStep) => void = () => {};
  
  function TeleportProgressWrapper() {
    const [step, _setStep] = useState<TeleportProgressStep>('validating');
    setStep = _setStep;
    return <TeleportProgress currentStep={step} sessionId={sessionId} />;
  }
  
  root.render(
    <AppStateProvider>
      <TeleportProgressWrapper />
    </AppStateProvider>
  );
  
  const result = await teleportResumeCodeSession(sessionId, setStep);
  return result;
}

// TeleportProgress component renders:
export function TeleportProgress({ 
  currentStep, 
  sessionId 
}: { 
  currentStep: TeleportProgressStep;
  sessionId: string;
}) {
  const currentStepIndex = STEPS.findIndex(s => s.key === currentStep);
  
  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text>Resuming session: {sessionId.slice(0, 8)}...</Text>
      </Box>
      {STEPS.map((step, index) => (
        <Box key={step.key} flexDirection="row">
          <Text>
            {index < currentStepIndex ? '✓' : 
             index === currentStepIndex ? SPINNER_FRAMES[frame] : '○'}
          </Text>
          <Text dimColor={index !== currentStepIndex}>
            {step.label}
          </Text>
        </Box>
      ))}
    </Box>
  );
}
```

**Progress Steps:**
1. `validating` - Repository validation
2. `fetching_logs` - Session transcript download
3. `fetching_branch` - Branch info extraction
4. `checking_out` - Git branch checkout

---

## API Types (`utils/teleport/api.ts`)

### SessionResource Type

```typescript
export type SessionResource = {
  id: string;
  title: string;
  session_context: SessionContext;
  session_status: 'idle' | 'running' | 'requires_action' | 'archived';
  created_at: string;
  updated_at: string;
};

export type SessionContext = {
  sources: SessionContextSource[];
  cwd: string;
  outcomes: Outcome[] | null;
  custom_system_prompt: string | null;
  seed_bundle_file_id?: string; // Git bundle on Files API
  github_pr?: { owner: string; repo: string; number: number };
  environment_variables?: Record<string, string>;
};

export type SessionContextSource = {
  type: 'git_repository';
  url: string;
  branch?: string;
  commit?: string;
} | {
  type: 'environment';
  environment_id: string;
  environment_name: string;
};

export type GitSource = {
  type: 'git_repository';
  url: string;
  branch?: string;
  commit?: string;
};
```

### OAuth Headers

```typescript
export function getOAuthHeaders(accessToken: string): Record<string, string> {
  return {
    'Authorization': `Bearer ${accessToken}`,
    'Content-Type': 'application/json'
  };
}

// Beta header for CCR features
const CCR_BYOC_BETA = 'ccr-byoc-2025-07-29';
```

---

## Environment Management (`utils/teleport/environments.ts`)

Creates and manages cloud environments for remote sessions.

```typescript
export async function createDefaultCloudEnvironment(
  name: string
): Promise<EnvironmentResource> {
  const accessToken = getClaudeAIOAuthTokens()?.accessToken;
  if (!accessToken) {
    throw new Error('No access token');
  }
  
  const orgUUID = await getOrganizationUUID();
  if (!orgUUID) {
    throw new Error('No org UUID');
  }
  
  const url = `${getOauthConfig().BASE_API_URL}/v1/environments`;
  
  const response = await axios.post(url, {
    name,
    kind: 'anthropic_cloud',
    config: {
      environment_type: 'anthropic',
      cwd: '/home/user',
      init_script: null,
      languages: [
        { name: 'python', version: '3.11' },
        { name: 'node', version: '20' }
      ],
      network_config: { 
        allowed_hosts: [], 
        allow_default_hosts: true 
      }
    }
  }, {
    headers: { 
      ...getOAuthHeaders(accessToken), 
      'anthropic-beta': 'ccr-byoc-2025-07-29' 
    }
  });
  
  return response.data;
}

export async function fetchEnvironments(): Promise<EnvironmentResource[]> {
  const accessToken = getClaudeAIOAuthTokens()?.accessToken;
  if (!accessToken) {
    return [];
  }
  
  const orgUUID = await getOrganizationUUID();
  if (!orgUUID) {
    return [];
  }
  
  const url = `${getOauthConfig().BASE_API_URL}/v1/environments`;
  
  const response = await axios.get(url, {
    headers: { 
      ...getOAuthHeaders(accessToken),
      'x-organization-uuid': orgUUID 
    }
  });
  
  return response.data.data || [];
}
```

---

## Integration Points

### With Git System
- `findGitRoot()` - Locates git repository root
- `gitExe()` - Gets git executable path
- `execFileNoThrow()` - Executes git commands
- `detectCurrentRepositoryWithHost()` - Parses remote URL

### With OAuth System
- `getClaudeAIOAuthTokens()` - Gets stored tokens
- `getOrganizationUUID()` - Gets org UUID
- `getOAuthHeaders()` - Builds auth headers

### With Analytics
- `logEvent()` - Tracks teleport events
- `getFeatureValue_CACHED_MAY_BE_STALE()` - GrowthBook feature flags

### With React/Ink
- `AppStateProvider` - State management
- `Root.render()` - UI rendering
- `setToolJSX()` - Dialog display

---

## Feature Flags (GrowthBook)

| Flag | Description | Default |
|------|-------------|---------|
| `tengu_ccr_bundle_max_bytes` | Max bundle size | 100MB |
| `tengu_malort_pedway` | Computer Use gate | Varies |

---

## Error Handling

### TeleportOperationError

```typescript
export class TeleportOperationError extends Error {
  formattedMessage: string;
  
  constructor(message: string, formattedMessage: string) {
    super(message);
    this.formattedMessage = formattedMessage;
  }
}
```

### Error Types Tracked

| Error Type | Description | User Message |
|------------|-------------|--------------|
| `no_access_token` | OAuth token missing | "API key authentication is not sufficient" |
| `no_org_uuid` | Org UUID fetch failed | "Unable to get organization UUID" |
| `repo_mismatch` | Wrong git directory | "You must run claude --teleport from..." |
| `branch_checkout_failed` | Git checkout failed | "Failed to checkout branch" |
| `session_not_found_404` | Session doesn't exist | "{sessionId} not found" |
| `bundle_too_large` | Repo exceeds size limit | "Repo is too large to bundle" |

---

## Related Files

**Module Documentation:**
- [conversationRecovery.md](../utils/conversationRecovery.md) - Message deserialization
- [auth.md](../utils/auth.md) - OAuth authentication

**Related Modules:**
- [hooks/useTeleportResume.md](../hooks/useTeleportResume.md) - React hook
- [components/TeleportProgress.md](../components/TeleportProgress.md) - Progress UI
- [components/TeleportError.md](../components/TeleportError.md) - Error dialog

**API Documentation:**
- Session API: `/v1/sessions/{id}`
- Session Ingress: `/v1/session_ingress/{id}/logs`
- Files API: `/v1/files` (bundle upload)

---

## Code Flow Summary

### Teleport Resume Flow

```
1. User runs: claude --teleport {sessionId}
                │
2. teleportResumeCodeSession(sessionId)
                │
3. validateGitState() ────┐ (if dirty, abort)
                │
4. fetchSession(sessionId)
                │
5. validateSessionRepository() ──┬── match ──┐
                │                │           │
                │                └── mismatch ──▶ throw TeleportOperationError
                │
6. getTeleportEvents() / getSessionLogsViaOAuth()
                │
7. processMessagesForTeleportResume()
                │
8. checkOutTeleportedSessionBranch() ──┬── success
                │                      │
                │                      └── error (log, continue)
                │
9. Return { log: messages[], branch: string }
                │
10. Session continues with recovered state
```

### Git Bundle Flow

```
1. createAndUploadGitBundle(config)
                │
2. git stash create ──┬── WIP exists ──▶ refs/seed/stash
                │      │
                │      └── No WIP ──▶ proceed
                │
3. _bundleWithFallback()
                │
    ┌─────────────┴──────────────┬──────────────┐
    │                            │              │
    ▼                            ▼              ▼
--all (100MB?)              HEAD (100MB?)   squashed-root
    │                            │              │
    └────────────┬───────────────┴──────────────┘
                 │
4. uploadFile(bundle, '_source_seed.bundle')
                 │
5. Return { fileId, size, scope, hasWip }
                 │
6. Cleanup: unlink bundle, delete refs/seed/*
```

---

*Deep dive created: 2026-04-07*
