# optional-skills/ Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/optional-skills/`

**Status:** complete

---

## Module Overview

The `optional-skills/` module contains official skills maintained by Nous Research that are **not activated by default**. This ~7,723 line module houses specialized, experimental, or heavyweight skills that users can optionally install.

These skills ship with the hermes-agent repository but are not copied to `~/.hermes/skills/` during setup. They are discoverable via the Skills Hub and can be installed on-demand.

**Why optional?**
- **Niche integrations** - Specific paid services, specialized tools
- **Experimental features** - Promising but not yet proven
- **Heavyweight dependencies** - Require significant setup (API keys, installs)

By keeping them optional, the default skill set stays lean while still providing curated, tested, official skills for users who want them.

---

## Directory Structure

### Skill Categories

| Directory | Purpose | Example Skills |
|-----------|---------|----------------|
| `autonomous-ai-agents/` | Meta-agent skills | Agent orchestration |
| `blockchain/` | Blockchain integrations | Solana, Base |
| `communication/` | Messaging tools | Email, chat |
| `creative/` | Creative tools | Meme generation |
| `devops/` | DevOps tools | Deployment, monitoring |
| `email/` | Email automation | SMTP, IMAP |
| `health/` | Health tracking | Fitness, wellness |
| `mcp/` | MCP protocol | FastMCP scaffolding |
| `mlops/` | ML operations | Model deployment |
| `migration/` | Migration tools | OpenClaw migration |
| `productivity/` | Productivity tools | Canvas, telephony |
| `research/` | Research tools | Domain intelligence |
| `security/` | Security tools | OSS forensics |

### Skill Files by Category

| File | Lines | Purpose |
|------|-------|---------|
| `DESCRIPTION.md` | 25 | Module overview |
| `blockchain/solana/scripts/solana_client.py` | 698 | Solana blockchain client |
| `blockchain/base/scripts/base_client.py` | 1,008 | Base L2 blockchain client |
| `creative/meme-generation/scripts/generate_meme.py` | 471 | Meme creation |
| `mcp/fastmcp/scripts/scaffold_fastmcp.py` | 56 | FastMCP scaffolding |
| `mcp/fastmcp/templates/api_wrapper.py` | 54 | FastMCP API wrapper template |
| `mcp/fastmcp/templates/file_processor.py` | 55 | FastMCP file processor template |
| `mcp/fastmcp/templates/database_server.py` | 77 | FastMCP database server template |
| `migration/openclaw-migration/scripts/openclaw_to_hermes.py` | 2,653 | OpenClaw migration script |
| `productivity/canvas/scripts/canvas_api.py` | 157 | Canvas LMS integration |
| `productivity/memento-flashcards/scripts/youtube_quiz.py` | 88 | YouTube quiz generator |
| `productivity/memento-flashcards/scripts/memento_cards.py` | 353 | Flashcard creation |
| `productivity/telephony/scripts/telephony.py` | 1,343 | Phone/SMS integration |
| `research/domain-intel/scripts/domain_intel.py` | 397 | Domain intelligence |
| `security/oss-forensics/scripts/evidence-store.py` | 313 | OSS evidence storage |

**Total:** ~7,723 lines across 15+ skill directories

---

## Key Components

### 1. Blockchain Skills

#### Solana Client (`blockchain/solana/`)
Solana blockchain interaction.

**Script:**
```python
# solana_client.py (698 lines)
"""Solana blockchain client for Hermes Agent.

Provides tools for:
- SOL balance checking
- Token transfers
- Program (smart contract) interaction
- Transaction history
"""

from solders.keypair import Keypair
from solders.client import Client

class SolanaClient:
    def __init__(self, rpc_url: str, keypair: Keypair = None):
        self.client = Client(rpc_url)
        self.keypair = keypair
    
    def get_balance(self, address: str) -> int:
        """Get SOL balance in lamports."""
        return self.client.get_balance(address).value
    
    def transfer(self, to: str, amount: int) -> str:
        """Transfer SOL to another address."""
        # Build and sign transaction
        # Send to network
        return tx_signature
    
    def get_token_accounts(self, address: str) -> list:
        """Get all token accounts for an address."""
        return self.client.get_token_accounts_by_owner(address).value
```

#### Base Client (`blockchain/base/`)
Base L2 blockchain integration.

**Script:**
```python
# base_client.py (1,008 lines)
"""Base L2 blockchain client.

Base is Coinbase's Ethereum L2. This client provides:
- ETH balance checking
- ERC-20 token operations
- Smart contract calls
- Bridge operations (ETH <-> Base)
"""

from web3 import Web3

class BaseClient:
    def __init__(self, rpc_url: str, private_key: str = None):
        self.w3 = Web3(Web3.HTTPProvider(rpc_url))
        if private_key:
            self.account = self.w3.eth.account.from_key(private_key)
    
    def get_balance(self, address: str) -> int:
        """Get ETH balance in wei."""
        return self.w3.eth.get_balance(address)
    
    def transfer_eth(self, to: str, amount_wei: int) -> str:
        """Transfer ETH to another address."""
        # Build, sign, send transaction
        return tx_hash
    
    def call_contract(self, contract_address: str, abi: list, function: str, args: tuple):
        """Call a smart contract function."""
        contract = self.w3.eth.contract(address=contract_address, abi=abi)
        func = getattr(contract.functions, function)
        return func(*args).call()
```

### 2. Creative Skills

#### Meme Generation (`creative/meme-generation/`)
Generate memes using popular templates.

**Script:**
```python
# generate_meme.py (471 lines)
"""Meme generation skill for Hermes Agent.

Uses imgflip API or local template rendering.
"""

import requests
from PIL import Image, ImageDraw, ImageFont

def generate_meme(template: str, top_text: str, bottom_text: str) -> str:
    """Generate a meme with given text.
    
    Args:
        template: Meme template name (e.g., "drake", "distracted_boyfriend")
        top_text: Top text overlay
        bottom_text: Bottom text overlay
    
    Returns:
        Path to generated image
    """
    # Load template
    img = Image.open(f"templates/{template}.jpg")
    draw = ImageDraw.Draw(img)
    
    # Add text overlays
    font = ImageFont.truetype("impact.ttf", 48)
    
    # Top text (centered, white with black outline)
    draw_text(draw, top_text, img.width, 50, font)
    
    # Bottom text
    draw_text(draw, bottom_text, img.width, img.height - 100, font)
    
    # Save result
    output = f"output/meme_{template}_{uuid4()}.jpg"
    img.save(output)
    return output

def search_templates(query: str) -> list:
    """Search available meme templates."""
    # Query imgflip API or local index
    return matching_templates
```

### 3. MCP Skills

#### FastMCP Scaffolding (`mcp/fastmcp/`)
Templates for creating FastMCP servers.

**Scripts:**
```python
# scaffold_fastmcp.py (56 lines)
"""Scaffold a new FastMCP server project."""

def scaffold_fastmcp(name: str, template: str = "basic"):
    """Create a new FastMCP server project.
    
    Args:
        name: Project name
        template: Template to use (basic, api_wrapper, file_processor, database)
    """
    import shutil
    
    template_dir = Path(__file__).parent / "templates" / template
    output_dir = Path(name)
    
    # Copy template files
    shutil.copytree(template_dir, output_dir)
    
    # Replace placeholders
    for file in output_dir.glob("**/*.py"):
        content = file.read_text()
        content = content.replace("{{name}}", name)
        file.write_text(content)
    
    print(f"Created FastMCP server '{name}' in {output_dir}")
```

**Templates:**
- `api_wrapper.py` - Wrap an API as MCP tools
- `file_processor.py` - File processing pipeline
- `database_server.py` - Database query tools

### 4. Migration Skills

#### OpenClaw Migration (`migration/openclaw-migration/`)
Migrate from OpenClaw to Hermes.

**Script:**
```python
# openclaw_to_hermes.py (2,653 lines)
"""Migrate OpenClaw configuration and data to Hermes.

Handles:
- Config file conversion
- Skill migration
- Memory export/import
- Auth credential transfer
- Custom tool adaptation
"""

import json
import yaml
from pathlib import Path

class OpenClawToHermesMigrator:
    def __init__(self, openclaw_home: Path):
        self.openclaw_home = openclaw_home
        self.hermes_home = Path.home() / ".hermes"
    
    def migrate(self) -> bool:
        """Run full migration."""
        self.migrate_config()
        self.migrate_skills()
        self.migrate_memory()
        self.migrate_auth()
        self.migrate_tools()
        self.generate_report()
        return True
    
    def migrate_config(self):
        """Convert OpenClaw config to Hermes format."""
        oc_config = yaml.safe_load(
            (self.openclaw_home / "config.yaml").read_text()
        )
        
        # Map config keys
        hermes_config = {
            "provider": self.map_provider(oc_config.get("provider")),
            "model": {"name": oc_config.get("model")},
            "toolsets": {"enabled": oc_config.get("toolsets", [])},
        }
        
        # Save Hermes config
        (self.hermes_home / "config.yaml").write_text(
            yaml.safe_dump(hermes_config)
        )
    
    def migrate_skills(self):
        """Migrate OpenClaw skills to Hermes."""
        # Copy skill files
        # Adapt skill.yaml format
        pass
    
    def migrate_memory(self):
        """Export OpenClaw memory, import to Hermes."""
        # Export from OpenClaw
        # Import to Hermes memory provider
        pass
    
    def map_provider(self, oc_provider: str) -> str:
        """Map OpenClaw provider name to Hermes."""
        mapping = {
            "anthropic": "anthropic",
            "openai": "openai",
            "nous": "nous",
            "openrouter": "openrouter",
        }
        return mapping.get(oc_provider, oc_provider)
```

### 5. Productivity Skills

#### Canvas LMS Integration (`productivity/canvas/`)
Canvas Learning Management System API.

**Script:**
```python
# canvas_api.py (157 lines)
"""Canvas LMS API integration.

For educators and students using Canvas.
"""

import requests

class CanvasClient:
    def __init__(self, base_url: str, token: str):
        self.base_url = base_url
        self.token = token
        self.headers = {"Authorization": f"Bearer {token}"}
    
    def get_courses(self) -> list:
        """Get all courses for the user."""
        response = requests.get(
            f"{self.base_url}/api/v1/courses",
            headers=self.headers
        )
        return response.json()
    
    def get_assignments(self, course_id: int) -> list:
        """Get assignments for a course."""
        response = requests.get(
            f"{self.base_url}/api/v1/courses/{course_id}/assignments",
            headers=self.headers
        )
        return response.json()
    
    def submit_assignment(self, course_id: int, assignment_id: int, 
                          submission_type: str, url: str = None):
        """Submit an assignment."""
        data = {"submission": {"submission_type": submission_type}}
        if url:
            data["submission"]["url"] = url
        response = requests.post(
            f"{self.base_url}/api/v1/courses/{course_id}/assignments/{assignment_id}/submissions",
            headers=self.headers,
            json=data
        )
        return response.json()
```

#### Telephony (`productivity/telephony/`)
Phone and SMS integration.

**Script:**
```python
# telephony.py (1,343 lines)
"""Telephony integration for Hermes Agent.

Supports:
- Twilio API
- Vonage API
- Local SIP (Asterisk)
- SMS sending/receiving
- Voice calls (TTS)
"""

from twilio.rest import Client as TwilioClient

class TelephonyClient:
    def __init__(self, provider: str, account_sid: str, auth_token: str):
        if provider == "twilio":
            self.client = TwilioClient(account_sid, auth_token)
            self.phone_number = os.getenv("TWILIO_PHONE_NUMBER")
    
    def send_sms(self, to: str, body: str) -> str:
        """Send an SMS message."""
        message = self.client.messages.create(
            body=body,
            from_=self.phone_number,
            to=to
        )
        return message.sid
    
    def make_call(self, to: str, twiml: str) -> str:
        """Make a voice call with TTS."""
        call = self.client.calls.create(
            twiml=twiml,
            from_=self.phone_number,
            to=to
        )
        return call.sid
    
    def get_call_status(self, call_sid: str) -> str:
        """Get call status."""
        call = self.client.calls(call_sid).fetch()
        return call.status
```

#### Memento Flashcards (`productivity/memento-flashcards/`)
Flashcard creation from content.

**Scripts:**
```python
# memento_cards.py (353 lines)
"""Create flashcards from content using spaced repetition."""

def create_flashcards(content: str, num_cards: int = 10) -> list:
    """Generate flashcards from content.
    
    Uses LLM to extract Q&A pairs.
    """
    prompt = f"""
    Generate {num_cards} flashcards from the following content.
    Each flashcard should be a question-answer pair.
    
    Content:
    {content}
    
    Output format:
    Q: <question>
    A: <answer>
    """
    
    # Call LLM
    response = call_llm(prompt)
    
    # Parse Q&A pairs
    cards = parse_flashcards(response)
    return cards

# youtube_quiz.py (88 lines)
"""Generate quiz from YouTube video transcript."""

def youtube_quiz(video_id: str, num_questions: int = 5) -> list:
    """Generate quiz questions from YouTube video."""
    transcript = fetch_transcript(video_id)
    return create_flashcards(transcript, num_questions)
```

### 6. Research Skills

#### Domain Intelligence (`research/domain-intel/`)
Domain and company intelligence gathering.

**Script:**
```python
# domain_intel.py (397 lines)
"""Domain intelligence gathering.

Collects:
- WHOIS information
- DNS records
- Subdomain enumeration
- Technology stack detection
- SSL certificate info
"""

import whois
import dns.resolver
import requests

def domain_intel(domain: str) -> dict:
    """Gather intelligence on a domain."""
    results = {}
    
    # WHOIS
    results['whois'] = whois.whois(domain)
    
    # DNS records
    results['dns'] = {
        'A': get_dns_records(domain, 'A'),
        'MX': get_dns_records(domain, 'MX'),
        'TXT': get_dns_records(domain, 'TXT'),
        'NS': get_dns_records(domain, 'NS'),
    }
    
    # Subdomains
    results['subdomains'] = enumerate_subdomains(domain)
    
    # Technology stack
    results['technologies'] = detect_technologies(domain)
    
    # SSL certificate
    results['ssl'] = get_ssl_info(domain)
    
    return results
```

### 7. Security Skills

#### OSS Forensics (`security/oss-forensics/`)
Open source software forensics.

**Script:**
```python
# evidence-store.py (313 lines)
"""Evidence storage for security investigations.

Provides:
- Evidence collection
- Chain of custody tracking
- Hash verification
- Timeline generation
"""

import hashlib
from datetime import datetime

class EvidenceStore:
    def __init__(self, store_path: Path):
        self.store_path = store_path
        self.store_path.mkdir(parents=True, exist_ok=True)
    
    def add_evidence(self, file_path: Path, description: str, 
                     case_id: str) -> str:
        """Add evidence to the store."""
        evidence_id = generate_evidence_id()
        
        # Calculate hash
        file_hash = hashlib.sha256(file_path.read_bytes()).hexdigest()
        
        # Copy to evidence store
        evidence_path = self.store_path / f"{evidence_id}_{file_path.name}"
        evidence_path.write_bytes(file_path.read_bytes())
        
        # Record metadata
        metadata = {
            'evidence_id': evidence_id,
            'original_path': str(file_path),
            'description': description,
            'case_id': case_id,
            'hash': file_hash,
            'collected_at': datetime.now().isoformat(),
            'chain_of_custody': [{
                'action': 'collected',
                'timestamp': datetime.now().isoformat(),
            }]
        }
        
        # Save metadata
        metadata_path = self.store_path / f"{evidence_id}.json"
        metadata_path.write_text(json.dumps(metadata, indent=2))
        
        return evidence_id
    
    def verify_evidence(self, evidence_id: str) -> bool:
        """Verify evidence integrity."""
        # Load stored hash
        # Recalculate hash
        # Compare
        pass
```

---

## Installation

Optional skills can be installed via:

```bash
# Browse available optional skills
hermes skills browse --source optional

# Install a specific skill
hermes skills install <skill-identifier>

# Example: Install Solana client
hermes skills install blockchain/solana

# Example: Install meme generation
hermes skills install creative/meme-generation
```

**Installation Process:**
1. Copy skill files to `~/.hermes/skills/`
2. Install dependencies (if any)
3. Update skills config to enable
4. Verify installation

---

## Configuration

**Skills Config:**
```yaml
# ~/.hermes/config.yaml

skills:
  enabled:
    - github
    - docker
    # Optional skills after installation:
    - blockchain/solana
    - creative/meme-generation
```

**Environment Variables:**
```bash
# Blockchain
export SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
export SOLANA_KEYPAIR_PATH=~/.solana/id.json

# Base
export BASE_RPC_URL=https://mainnet.base.org
export BASE_PRIVATE_KEY=...

# Twilio
export TWILIO_ACCOUNT_SID=...
export TWILIO_AUTH_TOKEN=...
export TWILIO_PHONE_NUMBER=+1...

# Canvas
export CANVAS_BASE_URL=https://canvas.instructure.com
export CANVAS_TOKEN=...
```

---

## Integration Points

### With Skills Hub (`hermes_cli/skills_hub.py`)
- Discovery and browsing
- Installation and activation
- Updates

### With Agent (`agent/`)
- Skills inject prompts and tools
- Tool execution via agent loop

### With Tools (`tools/`)
- Skills may use core tools
- Some skills add new tools

---

## Related Files

**Module Documentation:**
- [skills/exploration.md](../skills/exploration.md) - Main skills module

**Related Modules:**
- [hermes_cli/skills_hub.md](../hermes_cli/skills_hub.md) - Skills management
- [tools/skills_hub.py](../tools/skills_hub.py) - Skills tool

---

*Deep dive created: 2026-04-07*
