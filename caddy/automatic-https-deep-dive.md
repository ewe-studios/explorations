# Automatic HTTPS - Deep Dive

## Overview

Caddy's automatic HTTPS is its flagship feature. It obtains and renews TLS certificates automatically, configures HTTP->HTTPS redirects, and staples OCSP responses - all by default.

## Architecture Components

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Automatic HTTPS Pipeline                         │
│                                                                      │
│  ┌─────────────┐    ┌──────────────┐    ┌─────────────────────┐    │
│  │  Caddyfile  │───►│  Caddyfile   │───►│  HTTPApp Provision  │    │
│  │  (config)   │    │  Parser      │    │  (apply defaults)   │    │
│  └─────────────┘    └──────────────┘    └─────────────────────┘    │
│                                                    │                 │
│                                                    ▼                 │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │              TLS App Provision                              │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │    │
│  │  │ Auto HTTPS   │  │  CertMagic   │  │  ACME        │      │    │
│  │  │ Detection    │  │  Config      │  │  Issuer      │      │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘      │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                    │                 │
│                                                    ▼                 │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │              Certificate Operations                         │    │
│  │  - Load from storage                                        │    │
│  │  - Obtain from CA if needed                                 │    │
│  │  - Cache in memory                                          │    │
│  │  - OCSP stapling                                            │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

## Caddyfile Parsing for HTTPS

### Automatic HTTPS Detection

```go
// From httpcaddyfile/builtins.go
func parseSiteDirective(h Helper) ([]ConfigValue, error) {
    // Extract domain names from site block
    // e.g., "example.com" or "www.example.com"

    // Enable automatic HTTPS for these domains
    h.httpOnly(false)  // Will serve HTTPS too

    return []ConfigValue{
        {
            Class: "route",
            Value: caddyhttp.Route{
                MatchersRaw: caddy.ModuleMap{
                    "host": caddy.JSON([]string{domain}),
                },
            },
        },
    }, nil
}
```

### HTTPS Defaults Applied

```go
// From httpcaddyfile/httptype.go
func (st *ServerType) serversFromPairings(...) {
    // For each server block:
    for i, sblock := range sblockKeys {
        // 1. Enable HTTPS by default
        if !hasTLSListener {
            addresses = append(addresses, NetworkAddress{
                Network:   "tcp",
                Host:      host,
                StartPort: 443,
                EndPort:   443,
            })
        }

        // 2. Enable HTTP->HTTPS redirect
        if !hasHTTPListener {
            addresses = append(addresses, NetworkAddress{
                Network:   "tcp",
                Host:      host,
                StartPort: 80,
                EndPort:   80,
            })
        }

        // 3. Configure auto HTTPS
        if autoHTTPS {
            domainSet[domain] = struct{}{}
        }
    }
}
```

## TLS App Integration

### Certificate Automation Configuration

```go
// From modules/caddytls/tls.go
func (t *TLS) Provision(ctx caddy.Context) error {
    // 1. Set up certificate cache
    cacheOpts := certmagic.CacheOptions{
        GetConfigForCert: func(cert certmagic.Certificate) (*certmagic.Config, error) {
            return t.getConfigForName(cert.Names[0]), nil
        },
        Logger: t.logger.Named("cache"),
    }
    certCache = certmagic.NewCache(cacheOpts)

    // 2. Create CertMagic config
    magic := certmagic.New(certCache, certmagic.Config{
        Storage: ctx.Storage(),
        Logger:  t.logger,
        OnEvent: t.onEvent,
    })

    // 3. Configure automation policies
    t.Automation.defaultPublicAutomationPolicy = new(AutomationPolicy)
    t.Automation.defaultPublicAutomationPolicy.Provision(t)

    // 4. Process "automate" certificate loader
    for _, sub := range automateNames {
        t.automateNames[sub] = struct{}{}
    }

    return nil
}
```

### Automation Policy Structure

```go
type AutomationPolicy struct {
    // Subjects this policy applies to
    SubjectsRaw []string `json:"subjects,omitempty"`

    // Certificate issuers
    IssuersRaw []json.RawMessage `json:"issuers,omitempty"`

    // Storage backend (overrides default)
    StorageRaw json.RawMessage `json:"storage,omitempty"`

    // On-demand TLS settings
    OnDemand bool `json:"on_demand,omitempty"`

    // Internal/processed fields
    subjects []string
    issuers  []certmagic.Issuer
    storage  certmagic.Storage
    magic    *certmagic.Config
}
```

## ACME/HTTPS Automation Flow

### Full Certificate Obtainment Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                    ManageAsync() Entry Point                        │
│                 (cfg.manageAll -> cfg.manageOne)                    │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Step 1: Check If Already Managed                                   │
│  - Search cache for existing managed certificate                    │
│  - If found and not expired, return                                 │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Step 2: Try Loading From Storage                                   │
│  - Load certificate from storage by domain name                     │
│  - Load associated private key                                      │
│  - Load metadata (issuer info, renewal window)                      │
│  - If successful, cache and return                                  │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Step 3: Obtain New Certificate (if not in storage)                 │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  3a. PreCheck() - Validation                                │   │
│  │  - Verify domain qualifies for public cert                  │   │
│  │  - Check if IP certificate (RFC 8738)                       │   │
│  │  - Verify email available                                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                 │                                   │
│                                 ▼                                   │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  3b. Create ACME Order                                      │   │
│  │  - Create CSR with domain names                             │   │
│  │  - Call ACME server NewOrder endpoint                       │
│  │  - Receive order with authorizations                        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                 │                                   │
│                                 ▼                                   │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  3c. Solve Challenges                                       │   │
│  │  - Select challenge type (HTTP-01, TLS-ALPN-01, DNS-01)     │   │
│  │  - Present challenge (create resource/record)               │   │
│  │  - Wait for ACME server validation                          │   │
│  │  - Clean up challenge resources                             │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                 │                                   │
│                                 ▼                                   │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  3d. Download Certificate                                   │   │
│  │  - Poll order status until "valid"                          │   │
│  │  - Download certificate chain                               │   │
│  │  - Verify chain validity                                    │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Step 4: Cache Certificate                                          │
│  - Add to in-memory certificate cache                               │
│  - Start maintenance goroutine                                      │
│  - Certificate ready for TLS handshakes                             │
└─────────────────────────────────────────────────────────────────────┘
```

### Challenge Selection Logic

```go
// From acmeclient.go
func (iss *ACMEIssuer) newACMEClient(useTestCA bool) (*acmez.Client, error) {
    client.ChallengeSolvers = make(map[string]acmez.Solver)

    // DNS challenge takes precedence (exclusive)
    if iss.DNS01Solver != nil {
        client.ChallengeSolvers[acme.ChallengeTypeDNS01] = iss.DNS01Solver
    } else {
        // Enable HTTP-01 challenge
        if !iss.DisableHTTPChallenge {
            var solver acmez.Solver = &httpSolver{
                handler: iss.HTTPChallengeHandler(http.NewServeMux()),
                address: net.JoinHostPort(iss.ListenHost, strconv.Itoa(iss.getHTTPPort())),
            }
            if !iss.DisableDistributedSolvers {
                solver = distributedSolver{
                    storage:                iss.config.Storage,
                    storageKeyIssuerPrefix: iss.storageKeyCAPrefix(client.Directory),
                    solver:                 solver,
                }
            }
            client.ChallengeSolvers[acme.ChallengeTypeHTTP01] = solver
        }

        // Enable TLS-ALPN-01 challenge
        if !iss.DisableTLSALPNChallenge {
            var solver acmez.Solver = &tlsALPNSolver{
                config:  iss.config,
                address: net.JoinHostPort(iss.ListenHost, strconv.Itoa(iss.getTLSALPNPort())),
            }
            if !iss.DisableDistributedSolvers {
                solver = distributedSolver{
                    storage:                iss.config.Storage,
                    storageKeyIssuerPrefix: iss.storageKeyCAPrefix(client.Directory),
                    solver:                 solver,
                }
            }
            client.ChallengeSolvers[acme.ChallengeTypeTLSALPN01] = solver
        }
    }

    // Wrap solvers for global challenge info tracking
    for name, solver := range client.ChallengeSolvers {
        client.ChallengeSolvers[name] = solverWrapper{solver}
    }

    return client, nil
}
```

## HTTP Challenge Handler

### Handler Implementation

```go
// From acmeissuer.go
func (am *ACMEIssuer) HTTPChallengeHandler(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        // Check if request is ACME challenge
        if !LooksLikeHTTPChallenge(r) {
            next.ServeHTTP(w, r)
            return
        }

        // Try to solve challenge
        if am.HandleHTTPChallenge(w, r) {
            return // Challenge handled
        }

        // Not our challenge, pass to next handler
        next.ServeHTTP(w, r)
    })
}

func (am *ACMEIssuer) HandleHTTPChallenge(w http.ResponseWriter, r *http.Request) bool {
    // Check if this is our challenge
    challengePath := "/.well-known/acme-challenge/" + token
    if r.URL.Path != challengePath {
        return false
    }

    // Look up challenge info from global memory
    if challenge, ok := GetACMEChallenge(r.Host); ok {
        return SolveHTTPChallenge(am.Logger, w, r, challenge.Challenge)
    }

    return false
}
```

### Distributed HTTP Challenge Solving

```go
// From solvers.go - distributedSolver
type distributedSolver struct {
    storage                certmagic.Storage
    storageKeyIssuerPrefix string
    solver                 acmez.Solver
}

func (ds distributedSolver) Present(ctx context.Context, chal acme.Challenge) error {
    // Store challenge info in shared storage
    key := ds.challengeStorageKey(chal)
    challengeData, _ := json.Marshal(chal)
    ds.storage.Store(ctx, key, challengeData)

    // Any instance can solve - first to complete wins
    return ds.solver.Present(ctx, chal)
}

func (ds distributedSolver) Wait(ctx context.Context, chal acme.Challenge) error {
    // Check if challenge was solved (by us or another instance)
    key := ds.challengeStorageKey(chal)
    for {
        data, err := ds.storage.Load(ctx, key)
        if err == nil {
            var solvedChallenge acme.Challenge
            json.Unmarshal(data, &solvedChallenge)
            if solvedChallenge.Status == acme.StatusValid {
                return nil // Solved!
            }
        }
        time.Sleep(1 * time.Second)
    }
}
```

## TLS-ALPN Challenge

### Challenge Certificate Generation

```go
// From acmez library (called by CertMagic)
func TLSALPN01ChallengeCert(chal acme.Challenge) (tls.Certificate, error) {
    // Create special certificate for challenge
    // SAN contains: acme-tls/1 protocol identifier
    // Certificate used only for this handshake

    h := sha256.Sum256([]byte(chal.KeyAuthorization))
    keyAuthDigest := base64.RawStdEncoding.EncodeToString(h[:])

    cert := &x509.Certificate{
        SerialNumber: big.NewInt(1),
        Subject: pkix.Name{
            CommonName: acmeChallengeTLSALPNCoreName, // "acme.invalid"
        },
        DNSNames:       []string{acmeChallengeTLSALPNCoreName},
        NotBefore:      time.Now(),
        NotAfter:       time.Now().Add(24 * time.Hour),
        ExtraExtensions: []pkix.Extension{
            {
                // acmeIdentifier extension (OID: 1.3.6.1.5.5.7.1.31)
                Id:    oidExtensionAcmeIdentifier,
                Value: []byte(keyAuthDigest),
            },
        },
    }

    return createCertificate(cert)
}
```

### TLS-ALPN Connection Handler

```go
// From solvers.go - tlsALPNSolver.handleConn
func (s *tlsALPNSolver) handleConn(conn net.Conn) {
    defer conn.Close()

    tlsConn, ok := conn.(*tls.Conn)
    if !ok {
        log.Printf("expected tls.Conn but got %T", conn)
        return
    }

    // Perform handshake - this triggers GetCertificate
    err := tlsConn.Handshake()
    if err != nil {
        log.Printf("handshake error: %v", err)
        return
    }

    // Handshake successful - ACME server validated
    // Connection closed, challenge complete
}
```

## Certificate Management

### Storage Structure

```
~/.local/share/certmagic/
├── acme/
│   └── acme-v02.api.letsencrypt.org-directory/
│       ├── accounts/
│       │   └── A1234567890/
│       │       ├── meta.json       # Account metadata
│       │       └── private.key     # Account private key
│       └── orders/
│           └── O1234567890/
│               └── order.json      # Order state
├── certificates/
│   └── letsencrypt.org/
│       └── example.com/
│           ├── example.com.crt     # Certificate chain
│           ├── example.com.key     # Private key
│           └── example.com.meta.json
├── ocsp/
│   └── abc123...def.resp           # OCSP response
└── locks/
    └── ...                         # Distributed locks
```

### Certificate Metadata

```go
// Stored in {domain}.meta.json
type CertificateMetadata struct {
    // List of subject names on certificate
    SANs []string `json:"sans"`

    // Issuer identification
    IssuerKey string `json:"issuer_key"`

    // Renewal information
    Renewed     bool      `json:"renewed"`
    RenewedAt   time.Time `json:"renewed_at"`
    StoragePath string    `json:"storage_path"`

    // ACME-specific data
    ACMEData struct {
        OrderURL    string `json:"order_url"`
        Certificate URL    `json:"cert_url"`
    } `json:"acme,omitempty"`
}
```

## OCSP Stapling Deep Dive

### What is OCSP Stapling?

OCSP (Online Certificate Status Protocol) stapling allows the server to provide certificate revocation status during the TLS handshake, rather than requiring clients to query the OCSP responder directly.

**Benefits:**
- Improved privacy (no third-party reveals browsing)
- Better performance (no extra round trip)
- Higher reliability (works even if OCSP responder is down)

### OCSP Stapling Process

```go
// From ocsp.go - stapleOCSP
func stapleOCSP(ctx context.Context, ocspConfig OCSPConfig, storage Storage, cert *Certificate, issuer *x509.Certificate) error {
    // 1. Check if certificate has OCSP responder
    if len(cert.Leaf.OCSPServer) == 0 {
        return nil // No OCSP support
    }

    // 2. Check if we have a cached response
    cached, err := loadOCSPResponse(storage, cert)
    if err == nil && freshOCSP(cached) {
        cert.ocsp = cached
        return nil // Use cached response
    }

    // 3. Create OCSP request
    ocspReq, err := ocsp.CreateRequest(cert.Leaf, issuer, nil)

    // 4. Send to OCSP responder
    resp, err := http.Post(cert.Leaf.OCSPServer[0], "application/ocsp-request",
        bytes.NewReader(ocspReq))

    // 5. Parse response
    parsed, err := ocsp.ParseResponse(resp, issuer)

    // 6. Verify response
    if parsed.Status != ocsp.Good {
        return fmt.Errorf("OCSP status: %v", parsed.Status)
    }

    // 7. Cache response
    storeOCSPResponse(storage, cert, parsed.Raw)

    // 8. Update certificate
    cert.ocsp = parsed
    cert.Certificate.OCSPStaple = parsed.Raw

    return nil
}
```

### OCSP Maintenance Schedule

```go
// From maintain.go
func (certCache *Cache) updateOCSPStaples(ctx context.Context) {
    certCache.mu.RLock()
    for _, cert := range certCache.cache {
        // Skip if expired
        if cert.Expired() {
            continue
        }

        // Check if response needs refreshing
        if cert.ocsp != nil {
            // Status is fresh, skip
            if cert.ocsp.Status != ocsp.Unknown && freshOCSP(cert.ocsp) {
                continue
            }

            // Certificate revoked!
            if cert.ocsp.Status == ocsp.Revoked {
                renewQueue = append(renewQueue, cert)
                continue
            }
        }

        // Queue for update
        updateQueue = append(updateQueue, cert)
    }
    certCache.mu.RUnlock()

    // Update outside lock
    for _, cert := range updateQueue {
        stapleOCSP(ctx, cfg.OCSP, cfg.Storage, &cert, nil)
    }
}
```

### Fresh OCSP Check

```go
func freshOCSP(resp *ocsp.Response) bool {
    // Consider response fresh if:
    // - Not past NextUpdate
    // - Still has some buffer time remaining

    nextUpdate := resp.NextUpdate
    now := time.Now()

    // Add buffer (20% of validity period, min 1 hour)
    validityPeriod := nextUpdate.Sub(resp.ThisUpdate)
    buffer := time.Duration(float64(validityPeriod) * 0.2)
    if buffer < time.Hour {
        buffer = time.Hour
    }

    return now.Add(buffer).Before(nextUpdate)
}
```

## Renewal Logic

### When Certificates Are Renewed

```go
// From certificates.go
func (cert Certificate) NeedsRenewal(cfg *Config) bool {
    // Calculate renewal window
    lifetime := expiresAt(cert.Leaf).Sub(cert.Leaf.NotBefore)
    renewalWindow := time.Duration(float64(lifetime) * cfg.RenewalWindowRatio)

    // Default ratio: 1/3 of lifetime
    // For 90-day cert: renew when 30 days remaining
    if cfg.RenewalWindowRatio == 0 {
        renewalWindow = lifetime / 3
    }

    return time.Until(expiresAt(cert.Leaf)) < renewalWindow
}
```

### Renewal Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                    RenewCertAsync()                                  │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│  1. Check If Already Renewed In Storage                             │
│  - Load cert from storage                                           │
│  - Check if newer than cached version                               │
│  - If yes, just reload (skip renewal)                               │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│  2. Force Renewal                                                   │
│  - Call Issue() with same CSR                                       │
│  - Reuse private key (unless ReusePrivateKeys=false)                │
│  - ACME flow: order -> challenges -> certificate                    │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│  3. Update Storage And Cache                                        │
│  - Write new cert to storage                                        │
│  - Update in-memory cache                                           │
│  - Old cert replaced                                                │
└─────────────────────────────────────────────────────────────────────┘
```

### ARI (ACME Renewal Information)

RFC 9773 extension for improved renewal timing:

```go
// From config.go - updateARI
func (cfg *Config) updateARI(ctx context.Context, cert Certificate, logger *zap.Logger) (Certificate, bool, error) {
    // Get ARI window from issuer
    var renewalInfo acme.RenewalInfo
    if issuer, ok := cfg.Issuers[0].(*ACMEIssuer); ok {
        renewalInfo, _ = issuer.GetRenewalInfo(ctx, cert)
    }

    // ARI window tells CA's preferred renewal time
    // Helps spread out renewals to avoid thundering herd
    if renewalInfo.SuggestedWindow.Start.After(time.Now()) {
        // Not yet in renewal window
        return cert, false, nil
    }

    // In renewal window - proceed with renewal
    return cert, true, nil
}
```

## Internal PKI (Private CA)

### When Public Certs Don't Work

For internal services, localhost, or private networks:

```go
// From modules/caddytls/internalissuer.go
type InternalIssuer struct {
    // CA configuration
    CA *PKI `json:"ca,omitempty"`

    // Certificate validity period
    Lifetime caddy.Duration `json:"lifetime,omitempty"`
}

func (iss *InternalIssuer) Issue(ctx context.Context, csr *x509.CertificateRequest) (*IssuedCertificate, error) {
    // 1. Get or create private CA
    ca, err := iss.CA.GetCA()

    // 2. Sign certificate with CA
    template := &x509.Certificate{
        SerialNumber: big.NewInt(1),
        Subject:      csr.Subject,
        DNSNames:     csr.DNSNames,
        IPAddresses:  csr.IPAddresses,
        NotBefore:    time.Now(),
        NotAfter:     time.Now().Add(iss.Lifetime),
    }

    certDER, err := x509.CreateCertificate(rand.Reader, template, ca.cert, csr.PublicKey, ca.key)

    // 3. Return signed certificate
    return &IssuedCertificate{
        Certificate: pemEncode(certDER),
    }, nil
}
```

### Default Internal CA

```
Internal CA Details:
- Root certificate stored in storage
- Valid for 10 years by default
- Intermediate certificates for signing
- Auto-trusted by Caddy (manual trust for clients)
```

## On-Demand TLS

### Deferred Certificate Management

```go
// From handshake.go - obtainOnDemandCertificate
func (cfg *Config) obtainOnDemandCertificate(ctx context.Context, hello *tls.ClientHelloInfo) (Certificate, error) {
    name, _ := cfg.getNameFromClientHello(hello)

    // 1. Check if allowed
    if err := cfg.checkIfCertShouldBeObtained(ctx, name, true); err != nil {
        return Certificate{}, err
    }

    // 2. Check if another goroutine is obtaining
    certLoadWaitChansMu.Lock()
    waiter, ok := certLoadWaitChans[name]
    if ok {
        certLoadWaitChansMu.Unlock()
        <-waiter.done  // Wait for other to complete
        return cfg.getCertDuringHandshake(ctx, hello, false)
    }

    // 3. We're the leader - obtain certificate
    waiter = &certLoadWaiter{done: make(chan struct{})}
    certLoadWaitChans[name] = waiter
    certLoadWaitChansMu.Unlock()

    defer func() {
        close(waiter.done)
        delete(certLoadWaitChans, name)
    }()

    // 4. Obtain from CA
    err := cfg.ObtainCertAsync(ctx, name)
    if err != nil {
        return Certificate{}, err
    }

    // 5. Cache and return
    return cfg.CacheManagedCertificate(ctx, name)
}
```

### Permission Checks

```go
// Decision function example
certmagic.Default.OnDemand = &certmagic.OnDemandConfig{
    DecisionFunc: func(ctx context.Context, name string) error {
        // Check domain allowlist
        allowedDomains := []string{"example.com", "*.example.com"}
        for _, d := range allowedDomains {
            if MatchWildcard(name, d) {
                return nil
            }
        }

        // Check rate limits
        if !rateLimiter.Allow(name) {
            return fmt.Errorf("rate limited")
        }

        // Check external permission service
        resp, err := http.Get("https://permission-service/check?domain=" + name)
        if err != nil || resp.StatusCode != 200 {
            return fmt.Errorf("not permitted")
        }

        return nil
    },
}
```

## Error Handling

### Retry Logic

```go
// From acmeissuer.go
func (am *ACMEIssuer) Issue(ctx context.Context, csr *x509.CertificateRequest) (*IssuedCertificate, error) {
    attempts := ctx.Value(AttemptsCtxKey{}).(*int)

    cert, usedTestCA, err := am.doIssue(ctx, csr, *attempts)

    // If succeeded with test CA but failed with production
    if *attempts > 0 && usedTestCA && am.CA != am.TestCA {
        // Retry with production CA
        cert, _, err = am.doIssue(ctx, csr, 0)
        if err != nil {
            // Check if rate limited
            var problem acme.Problem
            if errors.As(err, &problem) && problem.Status == 429 {
                // Keep retrying - rate limit will expire
                return nil, err
            }
            return nil, ErrNoRetry{err}
        }
    }

    return cert, err
}
```

### Exponential Backoff

```go
// Retry intervals:
// Attempt 1: Immediate
// Attempt 2: 5 minutes
// Attempt 3: 30 minutes
// Attempt 4: 2 hours
// Attempt 5: 6 hours
// ...
// Maximum: 24 hours
// Total duration: ~30 days

func nextRetryInterval(attempt int) time.Duration {
    // Exponential backoff with jitter
    base := 5 * time.Minute
    max := 24 * time.Hour

    interval := base * time.Duration(math.Pow(2, float64(attempt)))

    // Add jitter (+/- 25%)
    jitter := time.Duration(float64(interval) * 0.25 * (rand.Float64() - 0.5))

    interval = interval + jitter

    if interval > max {
        interval = max
    }

    return interval
}
```

## Performance Considerations

### Certificate Cache

- Default capacity: 10,000 certificates
- LRU eviction when full
- Read-heavy access pattern (RWMutex)

### Challenge Server Efficiency

```go
// Challenge servers are shared across concurrent operations
type solverInfo struct {
    listener net.Listener
    count    int32       // Atomic reference count
    closed   int32       // Atomic closed flag
    done     chan struct{}
}

// First solver to Present() starts server
// Last solver to CleanUp() stops server
```

### Storage I/O Optimization

```go
// Atomic file writes
func (fs *FileStorage) Store(ctx, key, value) error {
    tmpFile := path + "/." + key + ".tmp"
    finalFile := path + "/" + key

    // Write to temp file first
    os.WriteFile(tmpFile, value, 0600)

    // Atomic rename
    os.Rename(tmpFile, finalFile)
}
```

## Security Considerations

### Rate Limiting

**Let's Encrypt Limits:**
- 50 new certificates per account per week
- 5 duplicate certificates per week
- 50 failed validations per account per 3 hours
- 300 new orders per account per 3 hours

**CertMagic Internal Limits:**
```go
// From config.go
RateLimitEvents      = 10   // Max transactions
RateLimitEventsWindow = 1 * time.Minute  // Sliding window
```

### Private Key Rotation

By default, CertMagic generates a new private key for each certificate:
- Reduces impact of key compromise
- Prevents key pinning attacks
- Slightly increases CPU usage during issuance

```go
// From crypto.go
func (kg StandardKeyGenerator) GenerateKey() (crypto.PrivateKey, error) {
    return rsa.GenerateKey(rand.Reader, 2048)  // or ECDSA P-256
}
```

### ACME Account Security

- Account keys stored separately per CA
- External Account Binding (EAB) supported
- Account key rotation on re-registration
