# Auth Service - Authentication & Authorization Deep Dive

## Overview

**Auth** is Taubyte's authentication and authorization service. It handles GitHub OAuth integration, domain validation with ACME/Let's Encrypt, certificate management, and project/repository access control.

---

## Service Architecture

### Core Components

```
tau/services/auth/
├── service.go           # Main service implementation
├── type.go              # Service type definitions
├── http.go              # HTTP service
├── http_auth.go         # HTTP auth endpoints
├── stream.go            # P2P stream handling
├── deploy_key.go        # Deploy key management
├── domains_http_endpoints.go  # Domain HTTP endpoints
├── github.go            # GitHub integration
├── github_client.go     # GitHub client
├── github_http_endpoints.go   # GitHub HTTP endpoints
├── api_hooks.go         # Webhook API
├── api_acme.go          # ACME API
├── api_stats.go         # Statistics API
├── e2e_test.go          # E2E tests
├── common/
│   └── [shared code]
├── crypto/
│   └── helpers.go       # Crypto utilities
├── dream/
│   └── init.go          # Dream integration
├── hooks/
│   └── hooks.go         # Webhook handlers
├── projects/
│   ├── new.go           # Project creation
│   ├── methods.go       # Project methods
│   └── type.go          # Project types
├── repositories/
│   ├── new.go           # Repository handling
│   └── repositories.go  # Repository management
└── acme/
    └── store/
        └── store.go     # Certificate store
```

### Service Structure

```go
// tau/services/auth/type.go
type Service struct {
    ctx           context.Context
    node          peer.Node
    clientNode    peer.Node
    config        *tauConfig.Node
    dev           bool
    stream        *streams.Service
    http          http.Service
    kv            kvdb.KVDB
    github        *GitHubClient
    acmeStore     *ACMEStore
    projects      *ProjectManager
    repositories  *RepositoryManager
    hooks         *HookManager
    dvPublicKey   crypto.PublicKey
}

type GitHubClient struct {
    clientID     string
    clientSecret string
    httpClient   *http.Client
}

type ACMEStore struct {
    kv       kvdb.KVDB
    certs    sync.Map  // map[string]*Certificate
    resolver *acme.Resolver
}
```

---

## Service Initialization

```go
// tau/services/auth/service.go
func New(ctx context.Context, config *tauConfig.Node) (*Service, error) {
    srv := &Service{
        ctx:    ctx,
        dev:    config.DevMode,
        config: config,
    }

    // Initialize P2P node
    if config.Node == nil {
        srv.node, err = tauConfig.NewLiteNode(ctx, config,
            path.Join(config.Root, protocolCommon.Auth))
    } else {
        srv.node = config.Node
    }

    // Initialize database
    srv.kv, err = pebbleds.NewDatastore(
        path.Join(config.Root, "storage", config.Shape, "auth"),
        nil,
    )

    // Setup P2P stream
    srv.stream, err = streams.New(srv.node, protocolCommon.Auth,
        protocolCommon.AuthProtocol)
    srv.setupStreamRoutes()
    srv.stream.Start()

    // Initialize HTTP
    srv.http, err = auto.New(ctx, srv.node, config)
    srv.setupHTTPRoutes()
    srv.http.Start()

    // Initialize GitHub client
    srv.github = &GitHubClient{
        clientID:     config.GitHub.ClientID,
        clientSecret: config.GitHub.ClientSecret,
        httpClient:   &http.Client{},
    }

    // Initialize ACME store
    srv.acmeStore, err = NewACMEStore(srv.kv)

    // Initialize managers
    srv.projects = NewProjectManager(srv.kv)
    srv.repositories = NewRepositoryManager(srv.kv)
    srv.hooks = NewHookManager(srv.kv)

    return srv, nil
}

func (srv *Service) Close() error {
    srv.stream.Stop()
    srv.http.Close()
    srv.kv.Close()
    return nil
}
```

---

## GitHub OAuth Integration

### OAuth Flow

```go
// tau/services/auth/github.go
type GitHubClient struct {
    clientID     string
    clientSecret string
    httpClient   *http.Client
}

// Step 1: Redirect to GitHub
func (g *GitHubClient) AuthURL(state string) string {
    params := url.Values{
        "client_id":     []string{g.clientID},
        "redirect_uri":  []string{g.redirectURI},
        "scope":         []string{"repo read:org"},
        "state":         []string{state},
    }

    return fmt.Sprintf("https://github.com/login/oauth/authorize?%s", params.Encode())
}

// Step 2: Exchange code for token
func (g *GitHubClient) ExchangeToken(code string) (*GitHubToken, error) {
    resp, err := g.httpClient.PostForm(
        "https://github.com/login/oauth/access_token",
        url.Values{
            "client_id":     []string{g.clientID},
            "client_secret": []string{g.clientSecret},
            "code":          []string{code},
        },
    )
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var result struct {
        AccessToken string `json:"access_token"`
        TokenType   string `json:"token_type"`
        Scope       string `json:"scope"`
    }

    json.NewDecoder(resp.Body).Decode(&result)

    return &GitHubToken{
        AccessToken: result.AccessToken,
        TokenType:   result.TokenType,
        Scope:       result.Scope,
    }, nil
}

// Step 3: Get user info
func (g *GitHubClient) GetUser(token string) (*GitHubUser, error) {
    req, _ := http.NewRequest("GET", "https://api.github.com/user", nil)
    req.Header.Set("Authorization", "Bearer "+token)

    resp, err := g.httpClient.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var user GitHubUser
    json.NewDecoder(resp.Body).Decode(&user)
    return &user, nil
}

// Step 4: Get repositories
func (g *GitHubClient) GetRepositories(token string) ([]GitHubRepo, error) {
    req, _ := http.NewRequest("GET",
        "https://api.github.com/user/repos", nil)
    req.Header.Set("Authorization", "Bearer "+token)

    resp, err := g.httpClient.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var repos []GitHubRepo
    json.NewDecoder(resp.Body).Decode(&repos)
    return repos, nil
}
```

### GitHub HTTP Endpoints

```go
// tau/services/auth/github_http_endpoints.go
func (srv *Service) setupGitHubRoutes() {
    srv.http.HandleFunc("/api/auth/github/connect", srv.handleGitHubConnect)
    srv.http.HandleFunc("/api/auth/github/callback", srv.handleGitHubCallback)
    srv.http.HandleFunc("/api/auth/github/repos", srv.handleGitHubRepos)
    srv.http.HandleFunc("/api/auth/github/disconnect", srv.handleGitHubDisconnect)
}

func (srv *Service) handleGitHubConnect(w http.ResponseWriter, r *http.Request) {
    state := generateState()

    // Store state in session
    srv.saveState(state)

    authURL := srv.github.AuthURL(state)
    http.Redirect(w, r, authURL, http.StatusFound)
}

func (srv *Service) handleGitHubCallback(w http.ResponseWriter, r *http.Request) {
    code := r.URL.Query().Get("code")
    state := r.URL.Query().Get("state")

    // Verify state
    if !srv.verifyState(state) {
        http.Error(w, "Invalid state", http.StatusBadRequest)
        return
    }

    // Exchange code for token
    token, err := srv.github.ExchangeToken(code)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }

    // Get user info
    user, err := srv.github.GetUser(token.AccessToken)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }

    // Store token
    srv.storeGitHubToken(user.Login, token)

    // Redirect to success page
    http.Redirect(w, r, "/auth/success", http.StatusFound)
}
```

---

## ACME/Let's Encrypt Integration

### Certificate Store

```go
// tau/services/auth/acme/store/store.go
type ACMEStore struct {
    kv       kvdb.KVDB
    certs    sync.Map  // map[string]*Certificate
    resolver *acme.Resolver
    email    string
}

type Certificate struct {
    Domain     string
    CertPEM    []byte
    KeyPEM     []byte
    IssuedAt   time.Time
    ExpiresAt  time.Time
    AutoRenew  bool
}

func NewACMEStore(kv kvdb.KVDB) (*ACMEStore, error) {
    store := &ACMEStore{
        kv:    kv,
        email: "admin@tau.local",
    }

    // Configure ACME resolver
    store.resolver = &acme.Resolver{
        Prompt:     acme.AcceptTOS,
        Email:      store.email,
        Cache:      store,
        DirectoryURL: acme.LetsEncryptURL,
    }

    // Load existing certificates
    store.loadCertificates()

    return store, nil
}

func (s *ACMEStore) GetCertificate(domain string) (*Certificate, error) {
    // Check memory cache
    if cert, ok := s.certs.Load(domain); ok {
        c := cert.(*Certificate)
        if time.Now().Before(c.ExpiresAt) {
            return c, nil
        }
    }

    // Load from database
    key := certKey(domain)
    data, err := s.kv.Get(key)
    if err != nil {
        return nil, err
    }

    var cert Certificate
    json.Unmarshal(data, &cert)

    // Check if renewal needed
    if time.Now().Add(24*time.Hour).After(cert.ExpiresAt) && cert.AutoRenew {
        err := s.renewCertificate(domain)
        if err != nil {
            logger.Warn("Auto-renewal failed for", domain, ":", err)
        }
    }

    s.certs.Store(domain, &cert)
    return &cert, nil
}

func (s *ACMEStore) ObtainCertificate(domain string) (*Certificate, error) {
    // Obtain certificate from ACME server
    cert, err := s.resolver.ObtainCert(domain, acme.ObtainRequest{
        Domains: []string{domain},
        Bundle:  true,
    })
    if err != nil {
        return nil, err
    }

    // Store certificate
    stored := &Certificate{
        Domain:    domain,
        CertPEM:   cert.Certificate,
        KeyPEM:    cert.PrivateKey,
        IssuedAt:  time.Now(),
        ExpiresAt: cert.Leaf.NotAfter,
        AutoRenew: true,
    }

    s.saveCertificate(stored)
    return stored, nil
}

func (s *ACMEStore) saveCertificate(cert *Certificate) error {
    key := certKey(cert.Domain)
    data, _ := json.Marshal(cert)

    if err := s.kv.Put(key, data); err != nil {
        return err
    }

    s.certs.Store(cert.Domain, cert)
    return nil
}
```

### ACME HTTP Challenge

```go
// tau/services/auth/api_acme.go
func (srv *Service) handleACMEChallenge(w http.ResponseWriter, r *http.Request) {
    token := chi.URLParam(r, "token")

    // Get challenge response from storage
    response, err := srv.getChallengeResponse(token)
    if err != nil {
        http.Error(w, "Challenge not found", http.StatusNotFound)
        return
    }

    w.Header().Set("Content-Type", "text/plain")
    w.Write([]byte(response))
}

func (srv *Service) registerACMEChallenge(domain, token, response string) error {
    key := challengeKey(domain, token)
    return srv.kv.Put(key, []byte(response))
}
```

### Domain Validation

```go
// tau/services/auth/domains_http_endpoints.go
type DomainValidation struct {
    Domain     string    `json:"domain"`
    Status     string    `json:"status"`  // pending, validated, failed
    Challenge  string    `json:"challenge"`
    Token      string    `json:"token"`
    ValidatedAt time.Time `json:"validated_at,omitempty"`
}

func (srv *Service) handleDomainValidation(w http.ResponseWriter, r *http.Request) {
    var req struct {
        Domain string `json:"domain"`
    }
    json.NewDecoder(r.Body).Decode(&req)

    // Start validation
    validation := &DomainValidation{
        Domain:    req.Domain,
        Status:    "pending",
        Challenge: "http-01",
    }

    // Obtain certificate (triggers ACME challenge)
    cert, err := srv.acmeStore.ObtainCertificate(req.Domain)
    if err != nil {
        validation.Status = "failed"
        json.NewEncoder(w).Encode(validation)
        return
    }

    validation.Status = "validated"
    validation.ValidatedAt = time.Now()

    json.NewEncoder(w).Encode(validation)
}
```

---

## Project Management

### Project Structure

```go
// tau/services/auth/projects/type.go
type Project struct {
    ID          string            `json:"id"`
    Name        string            `json:"name"`
    Description string            `json:"description"`
    Owner       string            `json:"owner"`
    Repositories []string         `json:"repositories"`
    Databases   []string         `json:"databases"`
    Storage     []string         `json:"storage"`
    Functions   []string         `json:"functions"`
    Domains     []string         `json:"domains"`
    CreatedAt   time.Time         `json:"created_at"`
    UpdatedAt   time.Time         `json:"updated_at"`
    Settings    ProjectSettings   `json:"settings"`
}

type ProjectSettings struct {
    AutoDeploy      bool              `json:"auto_deploy"`
    Branch          string            `json:"branch"`
    Environment     map[string]string `json:"environment"`
    BuildCommand    string            `json:"build_command"`
    OutputDirectory string            `json:"output_directory"`
}
```

### Project Methods

```go
// tau/services/auth/projects/methods.go
type ProjectManager struct {
    kv kvdb.KVDB
}

func (pm *ProjectManager) Create(project *Project) error {
    project.ID = generateProjectID()
    project.CreatedAt = time.Now()
    project.UpdatedAt = time.Now()

    key := projectKey(project.ID)
    data, _ := json.Marshal(project)

    return pm.kv.Put(key, data)
}

func (pm *ProjectManager) Get(id string) (*Project, error) {
    key := projectKey(id)
    data, err := pm.kv.Get(key)
    if err != nil {
        return nil, err
    }

    var project Project
    json.Unmarshal(data, &project)
    return &project, nil
}

func (pm *ProjectManager) List(owner string) ([]*Project, error) {
    prefix := []byte("project:")
    results, err := pm.kv.List(prefix)

    var projects []*Project
    for _, data := range results {
        var project Project
        json.Unmarshal(data, &project)
        if owner == "" || project.Owner == owner {
            projects = append(projects, &project)
        }
    }

    return projects, nil
}

func (pm *ProjectManager) AddRepository(projectID, repoID string) error {
    project, err := pm.Get(projectID)
    if err != nil {
        return err
    }

    project.Repositories = append(project.Repositories, repoID)
    project.UpdatedAt = time.Now()

    key := projectKey(project.ID)
    data, _ := json.Marshal(project)

    return pm.kv.Put(key, data)
}
```

---

## Repository Management

### Repository Structure

```go
// tau/services/auth/repositories/type.go
type Repository struct {
    ID           string            `json:"id"`
    ProjectID    string            `json:"project_id"`
    Name         string            `json:"name"`
    Provider     string            `json:"provider"`  // github, gitlab, etc.
    URL          string            `json:"url"`
    Branch       string            `json:"branch"`
    DeployKey    string            `json:"deploy_key"`
    WebhookID    string            `json:"webhook_id"`
    LastSync     time.Time         `json:"last_sync"`
    SyncStatus   string            `json:"sync_status"`
    Settings     RepoSettings      `json:"settings"`
}

type RepoSettings struct {
    AutoSync    bool     `json:"auto_sync"`
    BuildOnPush bool     `json:"build_on_push"`
    Branches    []string `json:"branches"`
}
```

### Repository Methods

```go
// tau/services/auth/repositories/repositories.go
type RepositoryManager struct {
    kv kvdb.KVDB
}

func (rm *RepositoryManager) Create(repo *Repository) error {
    repo.ID = generateRepoID()

    // Generate deploy key
    deployKey, err := generateDeployKey()
    if err != nil {
        return err
    }
    repo.DeployKey = deployKey.PublicKey

    key := repoKey(repo.ID)
    data, _ := json.Marshal(repo)

    return rm.kv.Put(key, data)
}

func (rm *RepositoryManager) SetupWebhook(repo *Repository) error {
    // Register webhook with GitHub
    hook := &GitHubHook{
        Name:   "web",
        Events: []string{"push", "pull_request"},
        Config: map[string]interface{}{
            "url":          rm.getWebhookURL(repo.ID),
            "content_type": "json",
            "secret":       generateWebhookSecret(),
        },
    }

    // Call GitHub API to create webhook
    // ...

    repo.WebhookID = hook.ID
    return rm.Update(repo)
}

func (rm *RepositoryManager) HandleWebhook(repoID string, payload []byte) error {
    var event GitHubEvent
    json.Unmarshal(payload, &event)

    switch event.Action {
    case "push":
        return rm.handlePush(repoID, event)
    case "pull_request":
        return rm.handlePullRequest(repoID, event)
    }

    return nil
}

func (rm *RepositoryManager) handlePush(repoID string, event GitHubEvent) error {
    // Trigger build via Patrick
    job := &patrick.BuildJob{
        RepositoryID: repoID,
        Branch:       event.Ref,
        Commit:       event.HeadCommit.ID,
    }

    return patrickClient.CreateJob(job)
}
```

---

## Webhook Handling

### Hook Manager

```go
// tau/services/auth/hooks/hooks.go
type HookManager struct {
    kv kvdb.KVDB
}

type Hook struct {
    ID        string            `json:"id"`
    ProjectID string            `json:"project_id"`
    URL       string            `json:"url"`
    Events    []string          `json:"events"`
    Secret    string            `json:"secret"`
    Active    bool              `json:"active"`
    CreatedAt time.Time         `json:"created_at"`
}

func (hm *HookManager) Create(hook *Hook) error {
    hook.ID = generateHookID()
    hook.Secret = generateSecret()
    hook.CreatedAt = time.Now()

    key := hookKey(hook.ID)
    data, _ := json.Marshal(hook)

    return hm.kv.Put(key, data)
}

func (hm *HookManager) Trigger(hook *Hook, event interface{}) error {
    payload, _ := json.Marshal(event)

    // Sign payload
    signature := hmacSHA256(hook.Secret, payload)

    // Send webhook
    client := &http.Client{Timeout: 10 * time.Second}
    req, _ := http.NewRequest("POST", hook.URL, bytes.NewReader(payload))
    req.Header.Set("Content-Type", "application/json")
    req.Header.Set("X-Taubyte-Signature", signature)

    resp, err := client.Do(req)
    if err != nil {
        return err
    }
    defer resp.Body.Close()

    if resp.StatusCode < 200 || resp.StatusCode >= 300 {
        return fmt.Errorf("webhook returned status %d", resp.StatusCode)
    }

    return nil
}
```

---

## Secrets Management

### Secrets Storage

```go
// tau/services/auth/secrets.go
type SecretsManager struct {
    kv        kvdb.KVDB
    encryptor *Encryptor
}

func (sm *SecretsManager) Store(projectID, name, value string) error {
    // Encrypt value
    encrypted, err := sm.encryptor.Encrypt([]byte(value))
    if err != nil {
        return err
    }

    secret := &Secret{
        ProjectID: projectID,
        Name:      name,
        Value:     base64.StdEncoding.EncodeToString(encrypted),
        CreatedAt: time.Now(),
    }

    key := secretKey(projectID, name)
    data, _ := json.Marshal(secret)

    return sm.kv.Put(key, data)
}

func (sm *SecretsManager) Get(projectID, name string) (string, error) {
    key := secretKey(projectID, name)
    data, err := sm.kv.Get(key)
    if err != nil {
        return "", err
    }

    var secret Secret
    json.Unmarshal(data, &secret)

    // Decrypt value
    encrypted, _ := base64.StdEncoding.DecodeString(secret.Value)
    decrypted, err := sm.encryptor.Decrypt(encrypted)
    if err != nil {
        return "", err
    }

    return string(decrypted), nil
}
```

---

## P2P Protocol

### Stream Handlers

```go
// tau/services/auth/stream.go
func (srv *Service) setupStreamRoutes() {
    srv.stream.HandleFunc("auth.github", srv.handleGitHubAuth)
    srv.stream.HandleFunc("auth.validate", srv.handleValidateToken)
    srv.stream.HandleFunc("project.get", srv.handleGetProject)
    srv.stream.HandleFunc("project.list", srv.handleListProjects)
    srv.stream.HandleFunc("secrets.get", srv.handleGetSecret)
}

func (srv *Service) handleGitHubAuth(stream network.Stream) {
    var req GitHubAuthRequest
    json.NewDecoder(stream).Decode(&req)

    // Validate GitHub token
    user, err := srv.github.GetUser(req.Token)
    if err != nil {
        json.NewEncoder(stream).Encode(AuthResponse{
            Success: false,
            Error:   err.Error(),
        })
        return
    }

    json.NewEncoder(stream).Encode(AuthResponse{
        Success: true,
        User:    user,
    })
}
```

---

## Testing

### E2E Tests

```go
// tau/services/auth/e2e_test.go
func TestGitHubOAuth(t *testing.T) {
    config := createTestConfig()
    srv, err := New(ctx, config)

    // Simulate OAuth flow
    authURL := srv.github.AuthURL("test-state")

    // Verify auth URL format
    if !strings.Contains(authURL, "github.com/login/oauth/authorize") {
        t.Error("Invalid auth URL")
    }
}

func TestACMECertificate(t *testing.T) {
    srv := createTestAuth()

    // Obtain test certificate
    cert, err := srv.acmeStore.ObtainCertificate("test.example.com")
    if err != nil {
        t.Fatal(err)
    }

    // Verify certificate
    if cert.Domain != "test.example.com" {
        t.Error("Certificate domain mismatch")
    }
}
```

---

## Related Documents

- `../exploration.md` - Main exploration
- `gateway.md` - Gateway service (uses auth)
- `../production-grade.md` - Production considerations
- `../security-signing-exploration.md` - Security patterns
