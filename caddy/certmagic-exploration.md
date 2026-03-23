# CertMagic - Deep Dive Exploration

## Overview

CertMagic is the TLS automation library that powers Caddy's HTTPS features. It can be used standalone in any Go application to add automatic HTTPS with a single line of code.

**Key Capabilities:**
- Automated certificate issuance via ACME protocol
- Automatic renewal with configurable windows
- OCSP stapling and response caching
- Multiple challenge types (HTTP-01, TLS-ALPN-01, DNS-01)
- Distributed challenge solving across clusters

## Package Structure

```
certmagic/
├── certmagic.go        # High-level convenience functions
├── config.go           # Configuration structure and management
├── acmeissuer.go       # ACME protocol implementation
├── acmeclient.go       # ACME client wrapper
├── handshake.go        # TLS handshake handling
├── maintain.go         # Background certificate maintenance
├── cache.go            # In-memory certificate cache
├── storage.go          # Storage abstraction
├── filestorage.go      # Filesystem storage implementation
├── solvers.go          # ACME challenge solvers
├── ocsp.go             # OCSP stapling handling
├── crypto.go           # Cryptographic utilities
├── dnsutil.go          # DNS challenge utilities
└── internal/           # Internal utilities
    ├── atomicfile/     # Atomic file writes
    └── testutil/       # Test utilities
```

## High-Level API

### One-Line HTTPS

```go
// Serve HTTP handler over HTTPS with redirects
err := certmagic.HTTPS([]string{"example.com"}, mux)
```

This single call:
1. Obtains certificate from Let's Encrypt
2. Sets up HTTP listener on port 80
3. Sets up HTTPS listener on port 443
4. Redirects HTTP to HTTPS
5. Solves ACME challenges automatically
6. Renews certificate before expiration

### TLS Configuration

```go
// Get tls.Config for custom server
tlsConfig, err := certmagic.TLS([]string{"example.com"})

// Use with custom http.Server
server := &http.Server{
    Addr:      ":443",
    TLSConfig: tlsConfig,
    Handler:   mux,
}
```

### Custom Listener

```go
// Get TLS listener directly
ln, err := certmagic.Listen([]string{"example.com"})
```

## Configuration System

### Config Structure

```go
type Config struct {
    // Renewal window: fraction of cert lifetime when renewal starts
    RenewalWindowRatio float64

    // Event callback for certificate operations
    OnEvent func(ctx context.Context, event string, data map[string]any) error

    // Fallback server names
    DefaultServerName  string
    FallbackServerName string

    // On-demand TLS configuration
    OnDemand *OnDemandConfig

    // Certificate options
    MustStaple   bool
    Issuers      []Issuer
    IssuerPolicy IssuerPolicy
    ReusePrivateKeys bool
    KeySource    KeyGenerator
    CertSelection CertificateSelector

    // OCSP configuration
    OCSP OCSPConfig

    // Storage backend
    Storage Storage

    // Subject transformation (e.g., for wildcards)
    SubjectTransformer func(ctx context.Context, domain string) string

    // Logging
    Logger *zap.Logger

    // Internal pointer to certificate cache
    certCache *Cache
}
```

### Creating Configurations

**Default Configuration:**
```go
// Customize defaults FIRST
certmagic.DefaultACME.Email = "admin@example.com"
certmagic.DefaultACME.Agreed = true

// Then create config from defaults
cfg := certmagic.NewDefault()
```

**Custom Configuration:**
```go
// Create cache first
cache := certmagic.NewCache(certmagic.CacheOptions{
    GetConfigForCert: func(cert certmagic.Certificate) (*certmagic.Config, error) {
        return certmagic.New(cache, certmagic.Config{/* custom */}), nil
    },
    Logger: logger,
})

// Create config with cache
cfg := certmagic.New(cache, certmagic.Config{
    Storage: customStorage,
    Issuers: []certmagic.Issuer{customIssuer},
})
```

### ACME Configuration

```go
type ACMEIssuer struct {
    CA              string  // ACME directory URL
    TestCA          string  // Staging CA for testing
    Email           string  // Account email
    AccountKeyPEM   string  // Existing account key
    Agreed          bool    // Terms agreement
    ExternalAccount *acme.EAB  // External account binding

    // Challenge configuration
    DisableHTTPChallenge      bool
    DisableTLSALPNChallenge   bool
    DisableDistributedSolvers bool
    DNS01Solver               acmez.Solver

    // Network configuration
    ListenHost     string
    AltHTTPPort    int
    AltTLSALPNPort int

    // Timeouts and resolvers
    CertObtainTimeout time.Duration
    Resolver          string

    // Logging
    Logger *zap.Logger

    // Internal state
    config     *Config
    httpClient *http.Client
}
```

## Certificate Lifecycle Management

### Certificate Obtainment Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    ManageSync/ManageAsync                    │
│                     (cfg.manageAll)                          │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   manageOne()                                │
│  1. Check if already managed                                │
│  2. Try loading from storage                                │
│  3. If not found, obtain from issuer                        │
└─────────────────────────────────────────────────────────────┘
                            │
            ┌───────────────┴───────────────┐
            │                               │
            ▼                               ▼
┌─────────────────────────┐       ┌─────────────────────────┐
│  CacheManagedCertificate│       │    ObtainCertSync       │
│  (load from storage)    │       │   (or ObtainCertAsync)  │
└─────────────────────────┘       └─────────────────────────┘
            │                               │
            │                               ▼
            │                   ┌─────────────────────────┐
            │                   │  PreCheck()             │
            │                   │  - Validate domains     │
            │                   │  - Check rate limits    │
            │                   └─────────────────────────┘
            │                               │
            │                               ▼
            │                   ┌─────────────────────────┐
            │                   │  Issue()                │
            │                   │  - Create order         │
            │                   │  - Solve challenges     │
            │                   │  - Download cert        │
            │                   └─────────────────────────┘
            │                               │
            └───────────────┬───────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              certCache.CacheCertificate()                    │
│  - Add to in-memory cache                                    │
│  - Start maintenance goroutine                               │
└─────────────────────────────────────────────────────────────┘
```

### Renewal Logic

```go
// From maintain.go - RenewManagedCertificates
func (certCache *Cache) RenewManagedCertificates(ctx context.Context) error {
    // 1. Scan cache for certificates needing renewal
    // 2. Check storage first (might be renewed by another instance)
    // 3. Queue renewal if needed
    // 4. Process queue outside read lock

    certCache.mu.RLock()
    for certKey, cert := range certCache.cache {
        if cert.NeedsRenewal(cfg) {
            // Check if storage has newer cert
            storedCertNeedsRenew, _ := cfg.managedCertInStorageNeedsRenewal(ctx, cert)
            if !storedCertNeedsRenew {
                // Just reload from storage
                reloadQueue = append(reloadQueue, cert)
                continue
            }
            // Queue for renewal
            renewQueue.insert(cert)
        }
    }
    certCache.mu.RUnlock()

    // Process queues outside read lock...
}
```

### Renewal Window Calculation

```go
// From config.go
func (cert Certificate) NeedsRenewal(cfg *Config) bool {
    lifetime := expiresAt(cert.Leaf).Sub(cert.Leaf.NotBefore)
    renewalWindow := time.Duration(float64(lifetime) * cfg.RenewalWindowRatio)
    return time.Until(expiresAt(cert.Leaf)) < renewalWindow
}
```

Default ratio: 1/3 of certificate lifetime (about 30 days for 90-day certs)

## ACME Protocol Implementation

### ACME Client Architecture

```go
type acmeClient struct {
    iss        *ACMEIssuer   // Parent issuer configuration
    acmeClient *acmez.Client // Underlying ACME client
    account    acme.Account  // ACME account
}
```

### Account Management

```go
func (iss *ACMEIssuer) newACMEClientWithAccount(ctx context.Context, useTestCA, interactive bool) (*acmeClient, error) {
    // 1. Create basic ACME client
    client, err := iss.newACMEClient(useTestCA)
    if err != nil {
        return nil, err
    }

    // 2. Try loading account from storage
    account, err := iss.getAccountToUse(ctx, client.Directory)

    // 3. Register new account if needed
    if account.Status == "" {
        // Lock to prevent duplicate registration
        acctLockKey := accountRegLockKey(account)
        err = acquireLock(ctx, iss.config.Storage, acctLockKey)

        // Double-check after lock
        account, err = iss.getAccountToUse(ctx, client.Directory)

        // Register if still needed
        if account.Status == "" {
            // Call NewAccountFunc if configured
            if iss.NewAccountFunc != nil {
                iss.mu.Lock()
                account, err = iss.NewAccountFunc(ctx, iss, account)
                iss.mu.Unlock()
            }

            // Agree to terms
            account.TermsOfServiceAgreed = iss.isAgreed()

            // External account binding
            if iss.ExternalAccount != nil {
                account.SetExternalAccountBinding(ctx, client.Client, *iss.ExternalAccount)
            }

            // Create account
            account, err = client.NewAccount(ctx, account)

            // Persist to storage
            iss.saveAccount(ctx, client.Directory, account)
        }
    }

    return &acmeClient{iss, client, account}, nil
}
```

### Challenge Solving

**HTTP-01 Solver:**
```go
type httpSolver struct {
    handler http.Handler  // Challenge handler
    address string        // Listen address
}

func (s *httpSolver) Solve(ctx context.Context, challenge acmez.Challenge) error {
    // Start HTTP server
    // Serve challenge response at /.well-known/acme-challenge/TOKEN
    // Wait for validation
    // Stop server
}
```

**TLS-ALPN-01 Solver:**
```go
type tlsALPNSolver struct {
    config  *Config
    address string
}

func (s *tlsALPNSolver) Solve(ctx context.Context, challenge acmez.Challenge) error {
    // Start TLS listener on port 443 (or alt port)
    // Register challenge certificate in certCache
    // Certificate has special SAN: acme-tls/1 protocol
    // Wait for ACME server to connect
    // Validate and complete
}
```

**DNS-01 Solver:**
```go
type DNS01Solver struct {
    DNSManager
}

type DNSManager struct {
    DNSProvider libdns.RecordSetter  // Any libdns provider
}

func (s *DNS01Solver) Solve(ctx context.Context, challenge acmez.Challenge) error {
    // Create TXT record at _acme-challenge.domain
    // Wait for propagation
    // ACME server validates
    // Clean up record
}
```

### Distributed Challenge Solving

```go
type distributedSolver struct {
    storage                Storage
    storageKeyIssuerPrefix string
    solver                 acmez.Solver
}

func (ds distributedSolver) Solve(ctx context.Context, challenge acmez.Challenge) error {
    // Store challenge info in shared storage
    key := ds.storageKeyIssuerPrefix + "/challenges/" + challenge.Token
    ds.storage.Store(ctx, key, challengeData)

    // Any instance can solve the challenge
    // First to complete wins
    // Others detect completion and proceed
}
```

## TLS Handshake Integration

### GetCertificate Callback

```go
func (cfg *Config) GetCertificate(clientHello *tls.ClientHelloInfo) (*tls.Certificate, error) {
    return cfg.GetCertificateWithContext(clientHello.Context(), clientHello)
}

func (cfg *Config) GetCertificateWithContext(ctx context.Context, clientHello *tls.ClientHelloInfo) (*tls.Certificate, error) {
    // 1. Check for TLS-ALPN challenge (acme-tls/1 protocol)
    if clientHello.SupportedProtos == []string{"acme-tls/1"} {
        return cfg.getTLSALPNChallengeCert(clientHello)
    }

    // 2. Get certificate from cache
    cert, err := cfg.getCertDuringHandshake(ctx, clientHello, true)
    return &cert.Certificate, err
}
```

### Certificate Selection Logic

```go
func (cfg *Config) getCertDuringHandshake(ctx context.Context, hello *tls.ClientHelloInfo, loadIfNecessary bool) (Certificate, error) {
    // 1. Check in-memory cache (exact match)
    cert, matched, defaulted := cfg.getCertificateFromCache(hello)
    if matched {
        return cert, nil
    }

    // 2. Try external Managers
    externalCert, err := cfg.getCertFromAnyCertManager(ctx, hello, logger)
    if err != nil || !externalCert.Empty() {
        return externalCert, err
    }

    // 3. Check if cert should be obtained (OnDemand)
    if err := cfg.checkIfCertShouldBeObtained(ctx, name, false); err != nil {
        return Certificate{}, err
    }

    // 4. Load from storage if OnDemand enabled
    if cfg.OnDemand != nil {
        loadedCert, err := cfg.loadCertFromStorage(ctx, logger, hello)
        if err == nil {
            return loadedCert, nil
        }

        // 5. Obtain from CA on-demand
        return cfg.obtainOnDemandCertificate(ctx, hello)
    }

    // 6. Return default/fallback cert
    if defaulted {
        return cert, nil
    }

    return Certificate{}, fmt.Errorf("no certificate available for '%s'", name)
}
```

### Synchronization for Concurrent Requests

```go
// Prevents thundering herd on certificate load
var certLoadWaitChans = make(map[string]*certLoadWaiter)

func (cfg *Config) getCertDuringHandshake(...) {
    certLoadWaitChansMu.Lock()
    waiter, ok := certLoadWaitChans[name]
    if ok {
        // Another goroutine loading - wait
        certLoadWaitChansMu.Unlock()
        <-waiter.done
        // Return cached result
        return cfg.getCertDuringHandshake(ctx, hello, false)
    }

    // We're the leader
    waiter = &certLoadWaiter{done: make(chan struct{})}
    certLoadWaitChans[name] = waiter
    certLoadWaitChansMu.Unlock()

    defer func() {
        close(waiter.done)
        delete(certLoadWaitChans, name)
    }()

    // ... load certificate ...
}
```

## OCSP Stapling

### OCSP Configuration

```go
type OCSPConfig struct {
    // Disable stapling entirely (NOT recommended)
    DisableStapling bool

    // Override default responder URL
    OverrideURLs []string

    // Disable must-staple requirement
    DisableMustStaple bool
}
```

### Stapling Process

```go
func stapleOCSP(ctx context.Context, ocspConfig OCSPConfig, storage Storage, cert *Certificate, issuer *x509.Certificate) error {
    // 1. Check if certificate has OCSP responder URL
    if len(cert.Leaf.OCSPServer) == 0 {
        return nil
    }

    // 2. Fetch OCSP response
    ocspResponse, err := ocsp.CreateRequest(cert.Leaf, issuer, nil)
    resp, err := http.Post(ocspServer, "application/ocsp-request", bytes.NewReader(ocspResponse))

    // 3. Parse response
    parsed, err := ocsp.ParseResponse(resp, issuer)

    // 4. Cache to storage
    storage.Store(ctx, ocspKey, parsed.Raw)

    // 5. Update certificate
    cert.ocsp = parsed
    cert.Certificate.OCSPStaple = parsed.Raw
}
```

### OCSP Maintenance

```go
// From maintain.go - updateOCSPStaples
func (certCache *Cache) updateOCSPStaples(ctx context.Context) {
    certCache.mu.RLock()
    for _, cert := range certCache.cache {
        // Skip expired certs
        if cert.Expired() {
            continue
        }

        // Check if status is fresh
        if cert.ocsp != nil && freshOCSP(cert.ocsp) {
            continue
        }

        // Queue for update
        updateQueue = append(updateQueue, cert)
    }
    certCache.mu.RUnlock()

    // Update outside lock
    for _, cert := range updateQueue {
        stapleOCSP(ctx, cfg.OCSP, cfg.Storage, &cert, nil)

        // Check for revocation
        if cert.ocsp.Status == ocsp.Revoked {
            // Queue immediate renewal
            renewQueue = append(renewQueue, cert)
        }
    }
}
```

## Storage System

### Storage Interface

```go
type Storage interface {
    Store(ctx context.Context, key string, value []byte) error
    Load(ctx context.Context, key string) ([]byte, error)
    Delete(ctx context.Context, key string) error
    Exists(ctx context.Context, key string) bool
    List(ctx context.Context, prefix string) ([]string, error)
    Stat(ctx context.Context, key string) (KeyInfo, error)
    Lock(ctx context.Context, key string) error
    Unlock(ctx context.Context, key string) error
}
```

### Storage Key Hierarchy

```
certmagic/
├── acme/
│   └── {ca-host}/
│       ├── accounts/
│       │   └── {account-hash}/
│       │       ├── meta.json
│       │       └── private.key
│       └── orders/
│           └── {order-hash}/
├── certificates/
│   └── {issuer-key}/
│       └── {domain}/
│           ├── {domain}.crt
│           ├── {domain}.key
│           └── {domain}.meta.json
├── ocsp/
│   └── {ocsp-hash}.resp
└── locks/
    └── {lock-name}
```

### File Storage Implementation

```go
type FileStorage struct {
    Path string
}

func (fs *FileStorage) Store(ctx context.Context, key string, value []byte) error {
    // Atomic write using rename
    tmpFile := fs.Path + "/." + key + ".tmp"
    finalFile := fs.Path + "/" + key

    os.WriteFile(tmpFile, value, 0600)
    os.Rename(tmpFile, finalFile)
}

func (fs *FileStorage) Lock(ctx context.Context, key string) error {
    // Use flock for process-local locking
    // Use storage key for distributed locking
    lockFile := fs.Path + "/locks/" + key
    fd, err := os.OpenFile(lockFile, os.O_CREATE|os.O_RDWR, 0644)
    syscall.Flock(int(fd.Fd()), syscall.LOCK_EX)
}
```

## Cache System

### Cache Structure

```go
type Cache struct {
    // Certificate storage
    cache map[string]Certificate

    // Name-to-certificate mapping
    nameIndex map[string]map[string]bool

    // Options
    options CertificateCacheOptions

    // Maintenance
    stopChan chan struct{}
    doneChan chan struct{}

    // Synchronization
    mu       sync.RWMutex
    optionsMu sync.RWMutex
}
```

### Certificate Cache Options

```go
type CacheOptions struct {
    // Callback to get config for a certificate
    GetConfigForCert func(Certificate) (*Config, error)

    // Capacity limit (default: 10000)
    Capacity int

    // How often to check for renewals
    RenewCheckInterval time.Duration

    // How often to update OCSP staples
    OCSPCheckInterval time.Duration

    // Logger
    Logger *zap.Logger
}
```

### Maintenance Goroutine

```go
func (certCache *Cache) maintainAssets(panicCount int) {
    defer func() {
        if err := recover(); err != nil {
            if panicCount < 10 {
                certCache.maintainAssets(panicCount + 1)
            }
        }
    }()

    renewalTicker := time.NewTicker(certCache.options.RenewCheckInterval)
    ocspTicker := time.NewTicker(certCache.options.OCSPCheckInterval)

    for {
        select {
        case <-renewalTicker.C:
            certCache.RenewManagedCertificates(ctx)
        case <-ocspTicker.C:
            certCache.updateOCSPStaples(ctx)
        case <-certCache.stopChan:
            renewalTicker.Stop()
            ocspTicker.Stop()
            close(certCache.doneChan)
            return
        }
    }
}
```

## Rate Limiting

### Internal Rate Limiter

```go
// Designed to prevent firehosing CA endpoints
var rateLimiters = make(map[string]*RateLimiter)

type RateLimiter struct {
    limit  int        // Max events
    window time.Duration
    times  []time.Time
    mu     sync.Mutex
}

func (rl *RateLimiter) Wait(ctx context.Context) error {
    rl.mu.Lock()
    defer rl.mu.Unlock()

    // Remove old entries outside window
    cutoff := time.Now().Add(-rl.window)
    for len(rl.times) > 0 && rl.times[0].Before(cutoff) {
        rl.times = rl.times[1:]
    }

    // Check if at limit
    if len(rl.times) >= rl.limit {
        // Wait until oldest entry expires
        waitTime := rl.times[0].Add(rl.window).Sub(time.Now())
        rl.mu.Unlock()
        select {
        case <-time.After(waitTime):
        case <-ctx.Done():
            return ctx.Err()
        }
        rl.mu.Lock()
    }

    rl.times = append(rl.times, time.Now())
    return nil
}
```

## Events System

### Event Types

| Event | Data Fields | Abortable |
|-------|-------------|-----------|
| `cached_unmanaged_cert` | sans | No |
| `cert_obtaining` | renewal, identifier, forced, remaining, issuer | Yes |
| `cert_obtained` | renewal, identifier, remaining, issuer, paths | No |
| `cert_failed` | renewal, identifier, remaining, issuers, error | No |
| `tls_get_certificate` | client_hello | Yes |
| `cert_ocsp_revoked` | subjects, certificate, reason, revoked_at | No |

### Event Emission

```go
func (cfg *Config) emit(ctx context.Context, eventName string, data map[string]any) error {
    if cfg.OnEvent != nil {
        return cfg.OnEvent(ctx, eventName, data)
    }
    return nil
}

// Usage in certificate obtainment
err := cfg.emit(ctx, "cert_obtaining", map[string]any{
    "renewal":    isRenewal,
    "identifier": domain,
    "forced":     force,
    "remaining":  timeLeft,
    "issuer":     issuerName,
})
if err != nil {
    return fmt.Errorf("event handler aborted: %w", err)
}
```

## Error Handling

### Error Types

```go
// Indicates operation should not be retried
type ErrNoRetry struct {
    Err error
}

func (e ErrNoRetry) Error() string {
    return e.Err.Error()
}

// Context key for retry attempts
type AttemptsCtxKey struct{}

// Usage:
attempts := 0
ctx := context.WithValue(ctx, AttemptsCtxKey{}, &attempts)
```

### Retry Logic

```go
// From acmeissuer.go
func (am *ACMEIssuer) Issue(ctx context.Context, csr *x509.CertificateRequest) (*IssuedCertificate, error) {
    attempts := 0
    if attemptsPtr, ok := ctx.Value(AttemptsCtxKey{}).(*int); ok {
        attempts = *attemptsPtr
    }

    cert, usedTestCA, err := am.doIssue(ctx, csr, attempts)
    if err != nil {
        return nil, err
    }

    // If succeeded with test CA but failed with production, retry
    if attempts > 0 && usedTestCA && am.CA != am.TestCA {
        cert, _, err = am.doIssue(ctx, csr, 0)
        if err != nil {
            var problem acme.Problem
            if errors.As(err, &problem) && problem.Status == http.StatusTooManyRequests {
                // Rate limited - keep retrying
                return nil, err
            }
            return nil, ErrNoRetry{err}
        }
    }

    return cert, err
}
```

## DNS Challenge Providers

CertMagic supports all [libdns](https://github.com/libdns) providers:

```go
import "github.com/libdns/cloudflare"

certmagic.DefaultACME.DNS01Solver = &certmagic.DNS01Solver{
    DNSManager: certmagic.DNSManager{
        DNSProvider: &cloudflare.Provider{
            APIToken: "your-token",
        },
    },
}
```

Supported providers:
- Cloudflare
- Route53
- Google Cloud DNS
- Azure DNS
- DigitalOcean
- And 20+ more

## On-Demand TLS

### Configuration

```go
type OnDemandConfig struct {
    // Decision function for each request
    DecisionFunc func(ctx context.Context, name string) error

    // External certificate managers
    Managers []Manager

    // Internal allowlist (auto-populated)
    hostAllowlist map[string]struct{}
}
```

### Decision Function

```go
certmagic.Default.OnDemand = &certmagic.OnDemandConfig{
    DecisionFunc: func(ctx context.Context, name string) error {
        // Check if domain is allowed
        if !isAllowedDomain(name) {
            return fmt.Errorf("domain not allowed")
        }

        // Rate limiting
        if !rateLimiter.Allow(name) {
            return fmt.Errorf("rate limited")
        }

        return nil
    },
}
```

## Best Practices

### 1. Use Staging CA for Development

```go
certmagic.DefaultACME.CA = certmagic.LetsEncryptStagingCA
```

### 2. Always Set Email Address

```go
certmagic.DefaultACME.Email = "admin@example.com"
```

### 3. Monitor Logs

```go
// Use structured logging
logger, _ := zap.NewProduction()
certmagic.Default.Logger = logger
```

### 4. Use Persistent Storage

```go
// Don't use ephemeral storage in production
// Certificates will be lost on restart
```

### 5. Enable Distributed Solving in Clusters

```go
// Use shared storage (not local filesystem)
// All instances must use same storage backend
```
