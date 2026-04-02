# Production-Grade OpenClaude Implementation Guide

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude`

**Repository:** https://gitlawb.com/z6MkqDnb7Siv3Cwj7pGJq4T5EsUisECqR8KpnDLwcaZq5TPr/openclaude

---

## Table of Contents

1. [Production Readiness Checklist](#production-readiness-checklist)
2. [Deployment Strategies](#deployment-strategies)
3. [Environment Hardening](#environment-hardening)
4. [Multi-Provider Setup](#multi-provider-setup)
5. [Monitoring and Observability](#monitoring-and-observability)
6. [Security Best Practices](#security-best-practices)
7. [Performance Optimization](#performance-optimization)
8. [Disaster Recovery](#disaster-recovery)
9. [Cost Management](#cost-management)
10. [Team Collaboration](#team-collaboration)

---

## Production Readiness Checklist

### Pre-Deployment

- [ ] Environment variables configured securely
- [ ] API keys stored in secrets manager (not in code/repos)
- [ ] Provider reachability validated
- [ ] Rate limits understood and configured
- [ ] Fallback providers configured
- [ ] Monitoring/alerting set up
- [ ] Log aggregation configured
- [ ] Backup authentication method available

### Post-Deployment

- [ ] Health checks passing
- [ ] Latency within SLA
- [ ] Error rate below threshold
- [ ] Cost tracking enabled
- [ ] Team trained on troubleshooting
- [ ] Runbook documented

---

## Deployment Strategies

### Strategy 1: Single Provider with Fallback

Best for: Simple setups, cost-conscious teams

```bash
# Primary: OpenAI
export CLAUDE_CODE_USE_OPENAI=1
export OPENAI_API_KEY=$OPENAI_PRIMARY_KEY
export OPENAI_BASE_URL=https://api.openai.com/v1
export OPENAI_MODEL=gpt-4o

# Fallback: DeepSeek (cheaper)
export DEEPSEEK_API_KEY=$DEEPSEEK_KEY
export DEEPSEEK_BASE_URL=https://api.deepseek.com/v1
export DEEPSEEK_MODEL=deepseek-chat
```

**Failover Script:**
```bash
#!/bin/bash
# failover-to-deepseek.sh

# Test primary
if ! curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models | grep -q "200"; then
  
  echo "OpenAI unavailable, switching to DeepSeek"
  export OPENAI_BASE_URL=https://api.deepseek.com/v1
  export OPENAI_API_KEY=$DEEPSEEK_API_KEY
  export OPENAI_MODEL=deepseek-chat
fi

exec bun run dev
```

### Strategy 2: Smart Router (Automatic Selection)

Best for: Teams using multiple providers, dynamic workloads

```bash
# Install Python dependencies
pip install httpx

# Configure smart router
export ROUTER_MODE=smart
export ROUTER_STRATEGY=balanced  # latency | cost | balanced
export ROUTER_FALLBACK=true

# Provider configurations
export OPENAI_API_KEY=$OPENAI_KEY
export BIG_MODEL=gpt-4o
export SMALL_MODEL=gpt-4o-mini

export GEMINI_API_KEY=$GEMINI_KEY

export OLLAMA_BASE_URL=http://localhost:11434
```

**Smart Router Benefits:**
- Automatic provider selection based on latency/cost
- Real-time health monitoring
- Rolling latency averages
- Automatic failover

### Strategy 3: Hybrid Cloud + Local

Best for: Privacy-sensitive workloads, cost optimization

```bash
# Default to local for development
export OPENAI_BASE_URL=http://localhost:11434/v1
export OPENAI_MODEL=llama3.3:70b

# Cloud for production builds
if [ "$CI" = "true" ]; then
  export OPENAI_BASE_URL=https://api.openai.com/v1
  export OPENAI_API_KEY=$CI_OPENAI_KEY
  export OPENAI_MODEL=gpt-4o
fi
```

### Strategy 4: Environment-Specific Providers

Best for: Large teams, separate dev/prod environments

```bash
# Development (local, free)
if [ "$ENVIRONMENT" = "development" ]; then
  export OPENAI_BASE_URL=http://localhost:11434/v1
  export OPENAI_MODEL=llama3.1:8b
fi

# Staging (cost-effective cloud)
if [ "$ENVIRONMENT" = "staging" ]; then
  export OPENAI_BASE_URL=https://api.deepseek.com/v1
  export OPENAI_API_KEY=$DEEPSEEK_KEY
  export OPENAI_MODEL=deepseek-chat
fi

# Production (best quality)
if [ "$ENVIRONMENT" = "production" ]; then
  export OPENAI_BASE_URL=https://api.openai.com/v1
  export OPENAI_API_KEY=$OPENAI_PROD_KEY
  export OPENAI_MODEL=gpt-4o
fi
```

---

## Environment Hardening

### Secure Secret Management

#### Using Environment Files (Development)

```bash
# .env.local (gitignored)
CLAUDE_CODE_USE_OPENAI=1
OPENAI_API_KEY=sk-proj-abc123...
OPENAI_BASE_URL=https://api.openai.com/v1
OPENAI_MODEL=gpt-4o
API_TIMEOUT_MS=120000
```

```bash
# .envrc (for direnv)
export CLAUDE_CODE_USE_OPENAI=1
export OPENAI_API_KEY=$(op read "op://Personal/OpenAI/credential")
export OPENAI_MODEL=gpt-4o
```

#### Using Secret Managers (Production)

**AWS Secrets Manager:**
```bash
# Retrieve at runtime
export OPENAI_API_KEY=$(aws secretsmanager get-secret-value \
  --secret-id openclaude/openai/key \
  --query SecretString \
  --output text)
```

**1Password:**
```bash
export OPENAI_API_KEY=$(op read "op://Vault/OpenAI/credential")
```

**HashiCorp Vault:**
```bash
export OPENAI_API_KEY=$(vault kv get -field=key secret/openclaude/openai)
```

### Runtime Validation

```bash
#!/bin/bash
# validate-env.sh

set -e

echo "Validating OpenClaude environment..."

# Check required variables
required_vars=(
  "CLAUDE_CODE_USE_OPENAI"
  "OPENAI_MODEL"
)

for var in "${required_vars[@]}"; do
  if [ -z "${!var}" ]; then
    echo "FAIL: Required variable $var is not set"
    exit 1
  fi
done

# Check API key (if not local)
if [[ ! "$OPENAI_BASE_URL" =~ localhost ]]; then
  if [ -z "$OPENAI_API_KEY" ] || [ "$OPENAI_API_KEY" = "SUA_CHAVE" ]; then
    echo "FAIL: Valid OPENAI_API_KEY required for non-local provider"
    exit 1
  fi
fi

# Test provider reachability
echo "Testing provider reachability..."
response=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  "${OPENAI_BASE_URL}/models" 2>/dev/null || echo "000")

if [[ "$response" =~ ^(200|401|403)$ ]]; then
  echo "PASS: Provider reachable (status: $response)"
else
  echo "FAIL: Provider unreachable (status: $response)"
  exit 1
fi

echo "Environment validation successful!"
```

### Configuration Validation

```typescript
// scripts/validate-config.ts
interface Config {
  provider: 'openai' | 'ollama' | 'deepseek' | 'gemini'
  baseUrl: string
  model: string
  apiKey?: string
  timeout: number
}

function validateConfig(): Config {
  const config: Config = {
    provider: 'openai',
    baseUrl: process.env.OPENAI_BASE_URL || 'https://api.openai.com/v1',
    model: process.env.OPENAI_MODEL || 'gpt-4o',
    timeout: parseInt(process.env.API_TIMEOUT_MS || '120000'),
  }
  
  // Validate base URL
  try {
    new URL(config.baseUrl)
  } catch {
    throw new Error(`Invalid OPENAI_BASE_URL: ${config.baseUrl}`)
  }
  
  // Validate API key for remote providers
  const isLocal = config.baseUrl.includes('localhost') || 
                  config.baseUrl.includes('127.0.0.1')
  if (!isLocal && !process.env.OPENAI_API_KEY) {
    throw new Error('OPENAI_API_KEY required for remote providers')
  }
  
  // Detect provider type
  if (config.baseUrl.includes('deepseek')) config.provider = 'deepseek'
  if (config.baseUrl.includes('openrouter')) config.provider = 'openrouter'
  if (config.baseUrl.includes('groq')) config.provider = 'groq'
  
  return config
}
```

---

## Multi-Provider Setup

### Provider Configuration Matrix

```yaml
# providers.yaml (example configuration)
providers:
  primary:
    name: openai
    base_url: https://api.openai.com/v1
    api_key_env: OPENAI_API_KEY
    models:
      opus: gpt-4o
      sonnet: gpt-4o-mini
      haiku: gpt-4o-mini
    priority: 1
    weight: 100
  
  fallback:
    name: deepseek
    base_url: https://api.deepseek.com/v1
    api_key_env: DEEPSEEK_API_KEY
    models:
      opus: deepseek-chat
      sonnet: deepseek-chat
      haiku: deepseek-chat
    priority: 2
    weight: 50
  
  local:
    name: ollama
    base_url: http://localhost:11434/v1
    models:
      opus: llama3.3:70b
      sonnet: llama3.1:8b
      haiku: llama3.1:8b
    priority: 3
    weight: 10  # Use sparingly, slower
```

### Load Balancing Strategies

#### Round-Robin (Equal Distribution)

```python
class RoundRobinRouter:
    def __init__(self, providers):
        self.providers = providers
        self.index = 0
    
    def select(self):
        provider = self.providers[self.index]
        self.index = (self.index + 1) % len(self.providers)
        return provider
```

#### Weighted Distribution

```python
class WeightedRouter:
    def __init__(self, providers):
        self.providers = []
        for p in providers:
            self.providers.extend([p] * p.weight)
        self.index = 0
    
    def select(self):
        if not self.providers:
            return None
        provider = self.providers[self.index]
        self.index = (self.index + 1) % len(self.providers)
        return provider
```

#### Latency-Based Selection

```python
class LatencyRouter:
    def __init__(self, providers):
        self.providers = providers
        self.latencies = {p: float('inf') for p in providers}
    
    async def measure_latency(self, provider):
        start = time.time()
        try:
            await self.ping(provider)
            self.latencies[provider] = time.time() - start
        except:
            self.latencies[provider] = float('inf')
    
    def select(self):
        return min(self.providers, key=lambda p: self.latencies[p])
```

---

## Monitoring and Observability

### Health Check Endpoints

```bash
#!/bin/bash
# health-check.sh

HEALTH_ENDPOINT="${HEALTH_CHECK_PORT:-8080}/health"

case "$1" in
  start)
    # Start health check server
    python3 -c "
import http.server
import json
import os

class HealthHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/health':
            status = 'healthy'
            # Check provider connectivity
            try:
                import requests
                resp = requests.get(
                    os.environ.get('OPENAI_BASE_URL', '') + '/models',
                    headers={'Authorization': f\"Bearer {os.environ.get('OPENAI_API_KEY', '')}\"},
                    timeout=5
                )
                if resp.status_code not in [200, 401, 403]:
                    status = 'degraded'
            except:
                status = 'unhealthy'
            
            self.send_response(200 if status == 'healthy' else 503)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'status': status}).encode())
        else:
            self.send_response(404)

http.server.HTTPServer(('', $HEALTH_CHECK_PORT), HealthHandler).serve_forever()
" &
    echo $! > /var/run/openclaude-health.pid
    ;;
  
  check)
    curl -s "$HEALTH_ENDPOINT" | jq .
    ;;
esac
```

### Metrics Collection

```typescript
// metrics.ts
interface ProviderMetrics {
  requestCount: number
  errorCount: number
  totalLatency: number
  totalCost: number
  tokenUsage: {
    input: number
    output: number
  }
}

class MetricsCollector {
  private metrics: Map<string, ProviderMetrics> = new Map()
  
  recordRequest(provider: string, latency: number, tokens: { input: number, output: number }, cost: number) {
    const m = this.metrics.get(provider) || this.defaultMetrics()
    m.requestCount++
    m.totalLatency += latency
    m.tokenUsage.input += tokens.input
    m.tokenUsage.output += tokens.output
    m.totalCost += cost
    this.metrics.set(provider, m)
  }
  
  recordError(provider: string) {
    const m = this.metrics.get(provider) || this.defaultMetrics()
    m.errorCount++
    this.metrics.set(provider, m)
  }
  
  getStats(provider: string) {
    const m = this.metrics.get(provider)
    return {
      avgLatency: m.totalLatency / m.requestCount,
      errorRate: m.errorCount / m.requestCount,
      avgCost: m.totalCost / m.requestCount,
      ...m
    }
  }
}
```

### Logging Configuration

```javascript
// logging.config.js
const Pino = require('pino')

const logger = Pino({
  level: process.env.LOG_LEVEL || 'info',
  formatters: {
    level: (label) => ({ level: label.toUpperCase() }),
  },
  redact: {
    paths: ['apiKey', 'Authorization', 'OPENAI_API_KEY'],
    censor: '[REDACTED]',
  },
  transport: {
    target: 'pino-pretty',
    options: {
      colorize: true,
      translateTime: 'SYS:standard',
    },
  },
})

// Log provider selection
logger.info({
  event: 'provider_selected',
  provider: 'openai',
  model: 'gpt-4o',
  latency: 234,
}, 'Provider selected for request')

// Log errors with context
logger.error({
  event: 'provider_error',
  provider: 'openai',
  error: 'rate_limit_exceeded',
  retryAfter: 60,
}, 'Provider request failed')
```

### Alerting Rules

```yaml
# alerting.yaml (Prometheus format)
groups:
  - name: openclaude
    rules:
      - alert: HighErrorRate
        expr: rate(openclaude_errors_total[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} errors/sec"
      
      - alert: ProviderUnavailable
        expr: up{job="openclaude-provider"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Provider unavailable"
      
      - alert: HighLatency
        expr: histogram_quantile(0.95, rate(openclaude_latency_bucket[5m])) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High p95 latency"
          description: "p95 latency is {{ $value }}s"
      
      - alert: CostAnomaly
        expr: rate(openclaude_cost_total[1h]) > 10
        for: 30m
        labels:
          severity: warning
        annotations:
          summary: "Unusual cost detected"
          description: "Cost rate is ${{ $value }}/hour"
```

---

## Security Best Practices

### API Key Rotation

```bash
#!/bin/bash
# rotate-api-keys.sh

set -e

# Generate new key (OpenAI example)
NEW_KEY=$(curl -s -X POST https://api.openai.com/v1/api_keys \
  -H "Authorization: Bearer $OPENAI_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"description": "OpenClaude rotation '"$(date +%Y-%m-%d)"'"}' \
  | jq -r '.key')

# Store in secrets manager
aws secretsmanager put-secret-value \
  --secret-id openclaude/openai/key \
  --secret-string "$NEW_KEY"

# Update running services (graceful reload)
for pid in $(pgrep -f openclaude); do
  kill -HUP $pid
done

# Schedule old key deletion (after grace period)
at now + 1 hour <<EOF
aws secretsmanager delete-secret --secret-id openclaude/openai/old-key --force-delete-without-recovery
EOF

echo "Key rotated successfully"
```

### Rate Limiting

```typescript
// rate-limiter.ts
class RateLimiter {
  private tokens: Map<string, { count: number; resetAt: number }> = new Map()
  
  constructor(
    private limits: { requests: number; window: number }
  ) {}
  
  async acquire(key: string): Promise<boolean> {
    const now = Date.now()
    const record = this.tokens.get(key) || { count: 0, resetAt: now }
    
    if (now >= record.resetAt) {
      record.count = 0
      record.resetAt = now + this.limits.window
    }
    
    if (record.count >= this.limits.requests) {
      return false  // Rate limited
    }
    
    record.count++
    this.tokens.set(key, record)
    return true
  }
  
  getRemaining(key: string): number {
    const record = this.tokens.get(key)
    if (!record) return this.limits.requests
    return Math.max(0, this.limits.requests - record.count)
  }
}

// Usage
const limiter = new RateLimiter({ requests: 100, window: 60000 })

async function makeRequest() {
  if (!await limiter.acquire('openai')) {
    throw new RateLimitError('Too many requests')
  }
  // ... make request
}
```

### Input Validation

```typescript
// input-validation.ts
import { z } from 'zod'

const ProviderConfigSchema = z.object({
  baseUrl: z.string().url(),
  apiKey: z.string().min(1).optional(),
  model: z.string().min(1),
  timeout: z.number().positive().max(600000),
})

function validateProviderConfig(input: unknown) {
  const result = ProviderConfigSchema.safeParse(input)
  if (!result.success) {
    throw new ValidationError(result.error.format())
  }
  return result.data
}

// Sanitize model names to prevent injection
function sanitizeModelName(model: string): string {
  return model.replace(/[^a-zA-Z0-9._:-]/g, '').slice(0, 100)
}
```

### Network Security

```bash
# Firewall rules for local Ollama
sudo ufw allow from 127.0.0.1 to any port 11434
sudo ufw deny from any to any port 11434

# Or with iptables
sudo iptables -A INPUT -p tcp -s 127.0.0.1 --dport 11434 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 11434 -j DROP
```

---

## Performance Optimization

### Connection Pooling

```typescript
// connection-pool.ts
import { Agent } from 'undici'

const pool = new Agent({
  connections: 100,
  keepAliveTimeout: 30000,
  keepAliveMaxTimeout: 30000,
  bodyTimeout: 120000,
})

// Use pooled fetch
const response = await fetch(url, {
  dispatcher: pool,
  headers: { 'Authorization': `Bearer ${apiKey}` },
})
```

### Caching Strategies

#### Response Caching

```typescript
// cache.ts
import NodeCache from 'node-cache'

interface CachedResponse {
  body: string
  tokens: { input: number; output: number }
  timestamp: number
}

class ResponseCache {
  private cache = new NodeCache({ 
    stdTTL: 300,  // 5 minutes
    maxKeys: 1000,
  })
  
  private hashKey(messages: any[], model: string): string {
    const content = JSON.stringify({ messages, model })
    return require('crypto')
      .createHash('sha256')
      .update(content)
      .digest('hex')
  }
  
  get(messages: any[], model: string): CachedResponse | null {
    const key = this.hashKey(messages, model)
    return this.cache.get(key) || null
  }
  
  set(messages: any[], model: string, response: CachedResponse) {
    const key = this.hashKey(messages, model)
    this.cache.set(key, response)
  }
}
```

#### Token Budgeting

```typescript
// token-budget.ts
class TokenBudget {
  private used = 0
  private limit: number
  private resetAt: number
  
  constructor(dailyLimit: number) {
    this.limit = dailyLimit
    this.resetAt = Date.now() + 86400000  // 24 hours
  }
  
  async acquire(tokens: number): Promise<boolean> {
    if (Date.now() >= this.resetAt) {
      this.used = 0
      this.resetAt = Date.now() + 86400000
    }
    
    if (this.used + tokens > this.limit) {
      return false
    }
    
    this.used += tokens
    return true
  }
  
  getRemaining(): number {
    return Math.max(0, this.limit - this.used)
  }
}
```

### Batching Requests

```typescript
// batcher.ts
class RequestBatcher {
  private queue: Array<{ messages: any; resolve: Function; reject: Function }> = []
  private timer: NodeJS.Timeout | null = null
  
  constructor(
    private batchSize: number,
    private maxWait: number,
    private processor: (batch: any[]) => Promise<any[]>
  ) {}
  
  async add(messages: any): Promise<any> {
    return new Promise((resolve, reject) => {
      this.queue.push({ messages, resolve, reject })
      
      if (this.queue.length >= this.batchSize) {
        this.flush()
      } else if (!this.timer) {
        this.timer = setTimeout(() => this.flush(), this.maxWait)
      }
    })
  }
  
  private async flush() {
    if (this.timer) {
      clearTimeout(this.timer)
      this.timer = null
    }
    
    const batch = this.queue.splice(0, this.batchSize)
    try {
      const results = await this.processor(batch.map(b => b.messages))
      batch.forEach((b, i) => b.resolve(results[i]))
    } catch (e) {
      batch.forEach(b => b.reject(e))
    }
  }
}
```

---

## Disaster Recovery

### Backup Provider Configuration

```bash
#!/bin/bash
# emergency-failover.sh

# Save current config
cp .openclaude-profile.json .openclaude-profile.json.backup

# Emergency failover to free tier
cat > .openclaude-profile.json <<EOF
{
  "profile": "ollama",
  "env": {
    "OPENAI_BASE_URL": "http://localhost:11434/v1",
    "OPENAI_MODEL": "llama3.1:8b"
  }
}
EOF

# Restart service
systemctl restart openclaude

echo "Emergency failover complete. Restore with:"
echo "cp .openclaude-profile.json.backup .openclaude-profile.json"
```

### Service Recovery

```yaml
# systemd service with auto-recovery
# /etc/systemd/system/openclaude.service

[Unit]
Description=OpenClaude Service
After=network.target ollama.service
Requires=ollama.service

[Service]
Type=simple
User=openclaude
WorkingDirectory=/opt/openclaude
ExecStart=/usr/bin/bun run dev
Restart=always
RestartSec=10
StartLimitBurst=5
StartLimitInterval=60s

# Environment
Environment=CLAUDE_CODE_USE_OPENAI=1
EnvironmentFile=/etc/openclaude/.env

# Health check
ExecStartPost=/opt/openclaude/scripts/health-check.sh start

# Resource limits
MemoryLimit=1G
CPUQuota=50%

[Install]
WantedBy=multi-user.target
```

### Data Recovery

```bash
#!/bin/bash
# recover-session.sh

# Find available backups
BACKUP_DIR="/var/backups/openclaude"
LATEST=$(ls -t "$BACKUP_DIR" | head -1)

if [ -z "$LATEST" ]; then
  echo "No backups found"
  exit 1
fi

echo "Recovering from backup: $LATEST"

# Restore session data
cp "$BACKUP_DIR/$LATEST/.claude/memory/"* ~/.claude/memory/
cp "$BACKUP_DIR/$LATEST/.claude/skills/"* ~/.claude/skills/

# Restore config if requested
if [ "$1" = "--full" ]; then
  cp "$BACKUP_DIR/$LATEST/.openclaude-profile.json" ./
fi

echo "Recovery complete"
```

---

## Cost Management

### Budget Tracking

```typescript
// cost-tracker.ts
interface CostTier {
  model: string
  inputCostPer1k: number
  outputCostPer1k: number
}

const COST_TIERS: Record<string, CostTier> = {
  'gpt-4o': { model: 'gpt-4o', inputCostPer1k: 0.005, outputCostPer1k: 0.015 },
  'gpt-4o-mini': { model: 'gpt-4o-mini', inputCostPer1k: 0.00015, outputCostPer1k: 0.0006 },
  'deepseek-chat': { model: 'deepseek-chat', inputCostPer1k: 0.00027, outputCostPer1k: 0.0011 },
}

class CostTracker {
  private dailyTotal = 0
  private monthlyTotal = 0
  private budget: number
  
  constructor(monthlyBudget: number) {
    this.budget = monthlyBudget
  }
  
  recordUsage(model: string, inputTokens: number, outputTokens: number) {
    const tier = COST_TIERS[model]
    if (!tier) return
    
    const cost = (inputTokens * tier.inputCostPer1k / 1000) + 
                 (outputTokens * tier.outputCostPer1k / 1000)
    
    this.dailyTotal += cost
    this.monthlyTotal += cost
    
    // Alert if approaching budget
    if (this.monthlyTotal > this.budget * 0.8) {
      this.sendBudgetAlert()
    }
  }
  
  getProjectedMonthlyCost(): number {
    const dayOfMonth = new Date().getDate()
    const daysInMonth = new Date().getMonth() + 1 === 2 ? 28 : 30
    return (this.monthlyTotal / dayOfMonth) * daysInMonth
  }
}
```

### Cost Optimization Strategies

1. **Use smaller models for simple tasks:**
```bash
# For quick questions and simple edits
export OPENAI_MODEL=gpt-4o-mini

# For complex reasoning and coding
export OPENAI_MODEL=gpt-4o
```

2. **Implement model cascading:**
```python
async def cascading_request(messages):
    # Try cheap model first
    try:
        response = await call_model(messages, model="gpt-4o-mini")
        if response.confidence > 0.8:
            return response
    except:
        pass
    
    # Fall back to expensive model
    return await call_model(messages, model="gpt-4o")
```

3. **Schedule heavy tasks during off-peak:**
```bash
# Crontab for batch processing at night
0 2 * * * /opt/openclaude/scripts/process-batch.sh
```

---

## Team Collaboration

### Shared Configuration

```yaml
# .openclaude.team.yaml
team:
  name: "Engineering"
  
  providers:
    development:
      type: ollama
      model: llama3.1:8b
      shared: true
    
    staging:
      type: deepseek
      model: deepseek-chat
      shared_key: true
    
    production:
      type: openai
      model: gpt-4o
      individual_keys: true
  
  policies:
    max_daily_cost: 50
    require_approval_for:
      - model: gpt-4o
        above_tokens: 100000
    allowed_models:
      - gpt-4o
      - gpt-4o-mini
      - deepseek-chat
      - llama3.1:8b
```

### Onboarding Script

```bash
#!/bin/bash
# onboard-new-developer.sh

set -e

DEVELOPER_NAME="$1"
TEAM_SECRET_PATH="op://Engineering/OpenClaude"

if [ -z "$DEVELOPER_NAME" ]; then
  echo "Usage: $0 <developer-name>"
  exit 1
fi

echo "Onboarding $DEVELOPER_NAME to OpenClaude..."

# 1. Grant access to shared secrets
op user grant "$DEVELOPER_NAME" --vault "Engineering"

# 2. Create individual API key (if needed)
if op item get "OpenClaude-$DEVELOPER_NAME" &>/dev/null; then
  echo "Individual API key already exists"
else
  NEW_KEY=$(op item create \
    --title="OpenClaude-$DEVELOPER_NAME" \
    --category=API_CREDENTIAL \
    --vault="Engineering" \
    --password=32)
  echo "Created individual API key"
fi

# 3. Clone configuration
cp .openclaude.team.yaml .openclaude-"$DEVELOPER_NAME".yaml

# 4. Set up local environment
cat >> ~/.zshrc <<EOF

# OpenClaude configuration
export OPENCLAUDE_CONFIG=.openclaude-$DEVELOPER_NAME.yaml
EOF

# 5. Provide setup instructions
cat <<EOF

Onboarding complete! Next steps for $DEVELOPER_NAME:

1. Install 1Password CLI: https://developer.1password.com/docs/cli/
2. Run: op signin
3. Start OpenClaude: bun run dev:profile

Documentation: /docs/openclaude.md
EOF
```

---

## Appendices

### A. Environment Variable Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `CLAUDE_CODE_USE_OPENAI` | Yes | - | Enable OpenAI provider (set to `1`) |
| `OPENAI_API_KEY` | Yes* | - | API key (*not needed for local) |
| `OPENAI_BASE_URL` | No | `https://api.openai.com/v1` | Provider endpoint |
| `OPENAI_MODEL` | No | `gpt-4o` | Default model |
| `API_TIMEOUT_MS` | No | `600000` | Request timeout |
| `ROUTER_MODE` | No | `fixed` | Router mode (`smart` or `fixed`) |
| `ROUTER_STRATEGY` | No | `balanced` | Selection strategy |
| `LOG_LEVEL` | No | `info` | Logging verbosity |

### B. Provider Endpoint Reference

| Provider | Base URL | Notes |
|----------|----------|-------|
| OpenAI | `https://api.openai.com/v1` | Default |
| DeepSeek | `https://api.deepseek.com/v1` | Cost-effective |
| Groq | `https://api.groq.com/openai/v1` | Ultra-low latency |
| Together | `https://api.together.xyz/v1` | Wide model selection |
| OpenRouter | `https://openrouter.ai/api/v1` | Multi-model gateway |
| Ollama | `http://localhost:11434/v1` | Local |
| LM Studio | `http://localhost:1234/v1` | Local |

### C. Model Recommendations by Use Case

| Use Case | Recommended Model | Cost Tier |
|----------|------------------|-----------|
| Production coding | GPT-4o | High |
| Daily development | DeepSeek V3 | Low |
| Quick questions | GPT-4o-mini | Very Low |
| Local development | Llama 3.1 8B | Free |
| Complex reasoning | GPT-4o / Claude | High |
| Batch processing | Llama 3.3 70B (local) | Free |

---

## References

- [00-zero-to-openclaude-engineer.md](./00-zero-to-openclaude-engineer.md) — Fundamentals guide
- [01-openclaude-exploration.md](./01-openclaude-exploration.md) — Architecture deep-dive
- [README.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude/README.md) — User documentation
- [PLAYBOOK.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude/PLAYBOOK.md) — Practical usage guide
