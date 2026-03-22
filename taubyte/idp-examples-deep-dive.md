# IDP Examples - Deep Dive Exploration

**Paths:**
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/digitalocean-idp/`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/cato-idp/`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/spore-drive-idp-example/`

---

## Executive Summary

These three projects demonstrate **Infrastructure/Developer Platform (IDP)** implementations using **Spore Drive**, a powerful infrastructure-as-code tool from Taubyte. They show how to deploy Tau (Taubyte's open-source PaaS/IDP) across different cloud providers and custom infrastructure.

**Key Value Proposition:** Build your own PaaS/IDP with significant cost savings compared to AWS Lambda, Vercel, Cloudflare, or similar providers.

---

## Architecture Overview

### Common Pattern

All three IDP examples follow the same architectural pattern:

```
┌─────────────────────────────────────────────────────────────┐
│                    Spore Drive                              │
│  (Infrastructure Orchestration Layer)                       │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
     ┌────────────┐  ┌────────────┐  ┌────────────┐
     │DigitalOcean│  │   Cato     │  │  Custom    │
     │  Droplets  │  │  Bare Metal│  │  Servers   │
     └────────────┘  └────────────┘  └────────────┘
              │               │               │
              └───────────────┼───────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │  Tau Platform   │
                    │  (PaaS/IDP)     │
                    └─────────────────┘
```

---

## Spore Drive Foundation

### What is Spore Drive?

**Spore Drive** (`@taubyte/spore-drive`) is the core orchestration engine that:
- Manages cloud infrastructure configuration
- Deploys Tau services across nodes
- Handles service lifecycle
- Provides progress tracking

### Core Concepts

**Config:** Central configuration object
**Drive:** Deployment engine
**Course:** Deployment plan
**Shape:** Node group/service template
**Host:** Individual server/node
**Displacement:** Deployment execution

---

## DigitalOcean IDP

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/digitalocean-idp/`

**Purpose:** Deploy Tau PaaS on DigitalOcean Droplets

### Architecture

```
digitalocean-idp/
├── src/
│   ├── do.ts          # DigitalOcean API integration
│   ├── index.ts       # Main orchestration
│   └── namecheap.ts   # DNS management
├── package.json
├── tsconfig.json
├── LICENSE
└── README.md
```

### Dependencies

```json
{
  "dependencies": {
    "@opentf/cli-pbar": "^0.7.2",
    "@taubyte/spore-drive": "^0.1.10",
    "@types/xml2js": "^0.4.14",
    "colors": "^1.4.0",
    "dotenv": "^16.4.7",
    "dots-wrapper": "^3.11.12",
    "ts-node": "^10.9.2",
    "xml2js": "^0.6.2"
  },
  "devDependencies": {
    "@types/colors": "^1.2.4",
    "tsx": "^4.19.1",
    "typescript": "^5.6.3"
  }
}
```

### Configuration

**Environment Variables:**
```bash
# DigitalOcean
export DIGITALOCEAN_API_TOKEN="<your DigitalOcean token>"
export DIGITALOCEAN_PROJECT_NAME="<your project name>"
export DROPLET_ROOT_PASSWORD="<your droplet root pass>"

# Optional: Namecheap DNS
export NAMECHEAP_API_KEY="<your Namecheap API key>"
export NAMECHEAP_IP="<your IP address>"
export NAMECHEAP_USERNAME="<your Namecheap username>"
```

### Key Implementation (src/index.ts)

**Config Creation:**
```typescript
export const createConfig = async (config: Config) => {
  // Set domains
  await config.cloud.domain.root.set(DOMAIN);
  await config.cloud.domain.generated.set(DOMAIN_GENERATED);

  // Generate validation keys if not exist
  try {
    await config.cloud.domain.validation.keys.data.privateKey.get();
  } catch {
    await config.cloud.domain.validation.generate();
  }

  // Generate P2P swarm key
  try {
    await config.cloud.p2p.swarm.key.data.get();
  } catch {
    await config.cloud.p2p.swarm.generate();
  }

  // Configure authentication
  const mainAuth = config.auth.signer["main"];
  await mainAuth.username.set("root");
  await mainAuth.password.set(DROPLET_ROOT_PASSWORD);

  // Define service shape
  const all = config.shapes.get("all");
  await all.services.set([
    "auth", "tns", "hoarder", "seer",
    "substrate", "patrick", "monkey"
  ]);
  await all.ports.port["main"].set(4242);
  await all.ports.port["lite"].set(4262);

  // Configure hosts from DigitalOcean droplets
  const hosts = await config.hosts.list();
  const bootstrapers = [];

  for (const droplet of await Droplets()) {
    const { hostname, publicIp, tags } = DropletInfo(droplet);
    if (!hosts.includes(hostname)) {
      const host = config.hosts.get(hostname);
      bootstrapers.push(hostname);

      await host.addresses.add([`${publicIp}/32`]);
      await host.ssh.address.set(`${publicIp}:22`);
      await host.ssh.auth.add(["main"]);
      await host.location.set("40.730610, -73.935242");
      if (!(await host.shapes.list()).includes("all"))
        await host.shapes.get("all").generate();
    }
  }

  await config.cloud.p2p.bootstrap.shape["all"].nodes.add(bootstrapers);
  await config.commit();
};
```

**DNS Management:**
```typescript
export const fixDNS = async (config: Config): Promise<boolean> => {
  // Skip if Namecheap not configured
  if (!apiUser && !apiKey && !clientIp) {
    return false;
  }

  const client = new NamecheapDnsClient(apiUser, apiKey, clientIp, domain, false);
  await client.init();

  // Set seer A records
  client.setAll("seer", "A", seerAddrs);

  // Set tau NS records
  client.setAll("tau", "NS", ["seer."+DOMAIN]);

  // Set wildcard CNAME
  client.setAll("*.g", "CNAME", ["substrate.tau."+DOMAIN]);

  await client.commit();
  return true;
};
```

**Deployment Execution:**
```typescript
const config: Config = new Config(configPath);
await config.init();
await createConfig(config);

const drive: Drive = new Drive(config, TauLatest);
await drive.init();

const course = await drive.plot(new CourseConfig(["all"]));

console.log("Displacement...");
await course.displace();
await displayProgress(course);
console.log("[Done] Displacement");

console.log("Update DNS Records...");
if (await fixDNS(config)) console.log("[Done] DNS Records");
else console.log("[Skip] DNS Records");
```

---

## Cato IDP

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/cato-idp/`

**Purpose:** Deploy Tau PaaS on Cato Digital Bare Metal Servers

### Architecture

```
cato-idp/
├── src/
│   ├── csv.ts         # CSV parsing for server list
│   ├── index.ts       # Main orchestration
│   └── namecheap.ts   # DNS management
├── .env.example
├── cato.csv           # Server list (user-provided)
├── package.json
├── tsconfig.json
├── LICENSE
└── README.md
```

### Configuration

**Environment Variables (.env.example):**
```bash
# Server Configuration
SSH_KEY=cato.pem                    # Path to SSH private key
SERVERS_CSV_PATH=cato.csv           # Path to servers list
CATO_USER=cato-user                 # SSH user for server access

# Domain Configuration
ROOT_DOMAIN=pom.ac                  # Root domain for your platform
GENERATED_DOMAIN=g.pom.ac           # Generated subdomain

# Namecheap DNS (Optional)
NAMECHEAP_API_KEY=your_api_key
NAMECHEAP_IP=your_ip
NAMECHEAP_USERNAME=your_username
```

### CSV File Format

```csv
hostname,public_ip
server1.example.com,192.168.1.1
server2.example.com,192.168.1.2
```

### Server Requirements

- Linux servers with SSH access on port 22
- User account with sudo/root privileges
- SSH key authentication enabled

---

## Spore Drive IDP Example

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/spore-drive-idp-example/`

**Purpose:** Generic example for deploying Tau on any SSH-accessible servers

### Architecture

```
spore-drive-idp-example/
├── src/
│   ├── csv.ts         # CSV parsing
│   ├── index.ts       # Main orchestration
│   └── namecheap.ts   # DNS management
├── .env.example
├── hosts.csv          # Server list
├── package.json
├── tsconfig.json
├── LICENSE
└── README.md
```

### Configuration

**Environment Variables:**
```bash
# Server Configuration
SSH_KEY=ssh-key.pem                    # Path to SSH private key
SERVERS_CSV_PATH=hosts.csv             # Path to servers list
SSH_USER=ssh-user                      # SSH user

# Domain Configuration
ROOT_DOMAIN=pom.ac                     # Root domain
GENERATED_DOMAIN=g.pom.ac              # Generated subdomain

# Namecheap DNS (Optional)
NAMECHEAP_API_KEY=your_api_key
NAMECHEAP_IP=your_ip
NAMECHEAP_USERNAME=your_username
```

### Key Differences from Other IDPs

1. **Generic SSH Access** - Works with any SSH-accessible server
2. **CSV-Based Host List** - Flexible server definition
3. **SSH Key Authentication** - More secure than password
4. **Customizable Domain** - Full domain control

---

## Common Implementation Details

### Progress Display (All IDPs)

```typescript
async function displayProgress(course: Course) {
  const multiPBar = new ProgressBar({ size: "SMALL" });
  multiPBar.start();
  const taskBars: Record<string, any> = {};
  const errors: { host: string; task: string; error: string }[] = [];

  for await (const displacement of await course.progress()) {
    const host = extractHost(displacement.path);
    const task = extractTask(displacement.path);

    if (!taskBars[host]) {
      taskBars[host] = multiPBar.add({
        prefix: host,
        suffix: "...",
        total: 100,
      });
    }

    taskBars[host].update({
      value: displacement.progress,
      suffix: task
    });

    if (displacement.error) {
      errors.push({ host, task, error: displacement.error });
    }
  }

  // Update final state
  for (const host in taskBars) {
    const errorForHost = errors.find((err) => err.host === host);
    if (errorForHost) {
      taskBars[host].update({ value: 100, color: "r", suffix: "failed" });
    } else {
      taskBars[host].update({ value: 100, suffix: "successful" });
    }
  }

  multiPBar.stop();

  if (errors.length > 0) {
    console.log("\nErrors encountered:");
    errors.forEach((err) => {
      console.log(`Host: ${err.host}, Task: ${err.task}, Error: ${err.error}`);
    });
    throw new Error("displacement failed");
  }
}
```

### Service Configuration (All IDPs)

```typescript
// Common service shape across all IDPs
const all = config.shapes.get("all");
await all.services.set([
  "auth",      // Authentication service
  "tns",       // Tau Naming Service
  "hoarder",   // Storage service
  "seer",      // Observability service
  "substrate", // Core runtime
  "patrick",   // HTTP server
  "monkey"     // Protocol handler
]);
await all.ports.port["main"].set(4242);
await all.ports.port["lite"].set(4262);
```

---

## Deployment Workflow

### Step-by-Step Process

1. **Install Dependencies**
   ```bash
   npm install
   ```

2. **Configure Environment**
   ```bash
   export DIGITALOCEAN_API_TOKEN="..."
   export DIGITALOCEAN_PROJECT_NAME="my-project"
   export DROPLET_ROOT_PASSWORD="..."
   ```

3. **Run Displacement**
   ```bash
   npm run displace
   ```

4. **Monitor Progress**
   ```
   host1.example.com  [=========>        ] 45% service:patrick
   host2.example.com  [==============>   ] 78% service:substrate
   ```

5. **Verify Deployment**
   ```bash
   curl https://substrate.tau.your-domain.com/health
   ```

---

## Service Architecture

### Tau Services Deployed

| Service | Port | Description |
|---------|------|-------------|
| **Auth** | - | Authentication & authorization |
| **TNS** | - | Tau Naming Service (resource naming) |
| **Hoarder** | - | Object storage service |
| **Seer** | - | Observability & telemetry |
| **Substrate** | - | Core runtime engine |
| **Patrick** | 4242 | HTTP server (main) |
| **Monkey** | - | Protocol handler |
| **Lite** | 4262 | Lightweight HTTP server |

### P2P Network

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Node 1    │◀───▶│   Node 2    │◀───▶│   Node 3    │
│ (Bootstrap) │     │  (Worker)   │     │  (Worker)   │
└─────────────┘     └─────────────┘     └─────────────┘
```

---

## DNS Configuration

### Required Records

**A Records:**
```
seer.your-domain.com.  IN  A  <node1-ip>
seer.your-domain.com.  IN  A  <node2-ip>
```

**NS Records:**
```
tau.your-domain.com.  IN  NS  seer.your-domain.com.
```

**CNAME Records:**
```
*.g.your-domain.com.  IN  CNAME  substrate.tau.your-domain.com.
```

---

## Security Considerations

### Authentication

- **SSH Key Authentication** - Recommended over passwords
- **Swarm Key** - P2P network encryption
- **Domain Validation Keys** - SSL/TLS automation

### Network Security

- **Encrypted P2P** - LibP2P with encryption
- **TLS/SSL** - Auto-generated or ACME certificates
- **Firewall Rules** - Configure cloud provider firewalls

### Best Practices

1. Use SSH keys instead of passwords
2. Enable cloud provider firewalls
3. Use private networking when available
4. Rotate swarm keys periodically
5. Monitor access logs via Seer

---

## Cost Comparison

### DigitalOcean Example

** Tau on DigitalOcean:**
- 3x Basic Droplets ($12/month each) = $36/month
- Total: ~$36/month

**Equivalent Managed Services:**
- AWS Lambda + API Gateway: ~$200-500/month
- Vercel Pro: $20/user/month + usage
- Cloudflare Workers: $5/user/month + usage

**Savings:** 70-90% cost reduction

---

## Troubleshooting

### Common Issues

1. **SSH Connection Failed**
   - Verify SSH key permissions: `chmod 600 key.pem`
   - Check security groups/firewall rules
   - Verify SSH user configuration

2. **Displacement Failed**
   - Check progress output for specific errors
   - Verify server accessibility
   - Check disk space on target servers

3. **DNS Not Working**
   - Verify DNS records propagated
   - Check Namecheap API credentials
   - Wait for TTL expiration

---

## Summary

These IDP examples demonstrate powerful patterns:

**DigitalOcean IDP:**
- Automated droplet discovery
- Password-based SSH
- Integrated DNS management

**Cato IDP:**
- Bare metal deployment
- CSV-based server list
- SSH key authentication

**Spore Drive Example:**
- Generic server support
- Maximum flexibility
- Reference implementation

**Key Benefits:**
- 70-90% cost savings vs managed services
- Full control over infrastructure
- Self-hostable and auditable
- Production-ready patterns
