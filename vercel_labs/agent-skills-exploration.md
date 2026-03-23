# Agent-Skills - Deep Dive Exploration

## Overview

**Agent-Skills** provides Claude skills for instant Vercel deployments without authentication. It demonstrates how to build AI agent skills that interact with external services.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs/agent-skills`

---

## Available Skills

### vercel-deploy

Deploy applications to Vercel instantly with no authentication required.

**Use when:**
- "Deploy my app"
- "Push this to production"
- "Deploy and give me the link"

**Features:**
- No authentication required
- Auto-detects 40+ frameworks from `package.json`
- Returns preview URL and claim URL
- Handles static HTML projects automatically
- Excludes `node_modules` and `.git`

**Output:**
```
✓ Deployment successful!

Preview URL: https://skill-deploy-abc123.vercel.app
Claim URL:   https://vercel.com/claim-deployment?code=...
```

---

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Claude Code    │ →── │  Skill Script   │ →── │  Vercel API     │
│  or claude.ai   │     │  (bash/Node)    │     │  (Deployments)  │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

---

## Skill Structure

```
skills/vercel-deploy/
├── SKILL.md              # Instructions for Claude
└── scripts/
    └── deploy.sh         # Deployment script
```

### SKILL.md Format

```markdown
# Vercel Deploy Skill

## Purpose
Deploy websites and applications to Vercel instantly.

## Usage
- "Deploy my app"
- "Push this live"
- "Deploy and give me the preview link"

## How it Works
1. Packages project into tarball
2. Detects framework from package.json
3. Uploads to Vercel deployment API
4. Returns preview and claim URLs

## Output Format
✓ Deployment successful!

Preview URL: https://...
Claim URL: https://...
```

---

## Deployment Flow

```typescript
// Conceptual flow (actual implementation uses deployment API)

async function deployToVercel(projectPath: string) {
  // 1. Package project
  const tarball = await createTarball(projectPath, {
    exclude: ['node_modules', '.git', '.vercel'],
  });

  // 2. Detect framework
  const packageJson = await readPackageJson(projectPath);
  const framework = detectFramework(packageJson);

  // 3. Create deployment
  const deployment = await fetch('https://api.vercel.com/v13/deployments', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${process.env.VERCEL_TOKEN}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      name: generateDeploymentName(),
      project: framework,
      target: 'production',
    }),
  });

  // 4. Upload files
  await uploadFiles(deployment.id, tarball);

  // 5. Return URLs
  return {
    previewUrl: `https://${deployment.url}`,
    claimUrl: `https://vercel.com/claim-deployment?code=${deployment.claimCode}`,
  };
}
```

---

## Framework Detection

```typescript
function detectFramework(packageJson: Record<string, any>): string {
  const deps = {
    ...packageJson.dependencies,
    ...packageJson.devDependencies,
  };

  // Next.js
  if (deps.next) return 'nextjs';

  // React + Vite
  if (deps.vite && deps.react) return 'vite';

  // Astro
  if (deps.astro) return 'astro';

  // Nuxt
  if (deps.nuxt) return 'nuxt';

  // SvelteKit
  if (deps['@sveltejs/kit']) return 'sveltekit';

  // Gatsby
  if (deps.gatsby) return 'gatsby';

  // Static HTML
  return 'static';
}
```

---

## Installation

### Claude Code

```bash
cp -r skills/vercel-deploy ~/.claude/skills/
```

### claude.ai

Add skill to project knowledge or paste `SKILL.md` into conversation.

---

## Usage Examples

```
User: Deploy my app

Claude: I'll deploy your project to Vercel now.

[executes: npx vercel-deploy]

✓ Deployment successful!

Preview URL: https://skill-deploy-abc123.vercel.app
Claim URL:   https://vercel.com/claim-deployment?code=xyz789

Your app is now live! You can transfer ownership using the claim URL.
```

---

## Troubleshooting

### Network Egress Error

If deployment fails on claude.ai due to network restrictions:

1. Go to claude.ai/admin-settings/capabilities
2. Add `*.vercel.com` to allowed domains

---

## Rust Implementation Considerations

### Skill as CLI Tool

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tar::Builder;
use std::fs::File;

#[derive(Serialize)]
struct DeploymentRequest {
    name: String,
    project_type: String,
    target: String,
    files: Vec<FileEntry>,
}

#[derive(Serialize)]
struct FileEntry {
    file: String,
    data: String,
}

pub struct VercelDeployer {
    client: Client,
    token: Option<String>,  // Optional for skill mode
}

impl VercelDeployer {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            token: std::env::var("VERCEL_TOKEN").ok(),
        }
    }

    pub async fn deploy(&self, project_path: &str) -> Result<DeploymentResult> {
        // 1. Scan project files
        let files = self.scan_project(project_path)?;

        // 2. Detect framework
        let framework = self.detect_framework(&files)?;

        // 3. Create deployment
        let deployment = self.create_deployment(&framework).await?;

        // 4. Upload files
        self.upload_files(&deployment.id, files).await?;

        // 5. Return URLs
        Ok(DeploymentResult {
            preview_url: format!("https://{}", deployment.url),
            claim_url: format!(
                "https://vercel.com/claim-deployment?code={}",
                deployment.claim_code
            ),
        })
    }

    fn scan_project(&self, path: &str) -> Result<Vec<FileEntry>> {
        let mut files = Vec::new();
        let mut tarball = Vec::new();

        {
            let mut builder = Builder::new(&mut tarball);

            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| !self.should_exclude(e))
            {
                let entry = entry?;
                if entry.file_type().is_file() {
                    // Add file to deployment
                    let relative_path = entry.path()
                        .strip_prefix(path)?
                        .to_string_lossy()
                        .to_string();

                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        files.push(FileEntry {
                            file: relative_path,
                            data: content,
                        });
                    }
                }
            }
        }

        Ok(files)
    }

    fn should_exclude(&self, entry: &walkdir::DirEntry) -> bool {
        let name = entry.file_name().to_string_lossy();
        matches!(name.as_ref(), "node_modules" | ".git" | ".vercel" | ".next")
    }

    fn detect_framework(&self, files: &[FileEntry]) -> Result<String> {
        // Look for framework indicators in files
        for file in files {
            if file.file == "package.json" {
                if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&file.data) {
                    if let Some(deps) = pkg.get("dependencies").and_then(|d| d.as_object()) {
                        if deps.contains_key("next") {
                            return Ok("nextjs".to_string());
                        }
                        if deps.contains_key("vite") && deps.contains_key("react") {
                            return Ok("vite".to_string());
                        }
                        // ... more frameworks
                    }
                }
            }
        }
        Ok("static".to_string())
    }

    async fn create_deployment(&self, framework: &str) -> Result<Deployment> {
        let mut req = self.client
            .post("https://api.vercel.com/v13/deployments")
            .json(&serde_json::json!({
                "project_type": framework,
                "target": "production",
            }));

        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req.send().await?.json::<serde_json::Value>().await?;

        Ok(Deployment {
            id: response["id"].as_str().unwrap().to_string(),
            url: response["url"].as_str().unwrap().to_string(),
            claim_code: response["claimCode"].as_str().unwrap().to_string(),
        })
    }

    async fn upload_files(&self, deployment_id: &str, files: Vec<FileEntry>) -> Result<()> {
        // Upload files in batches
        // ...
        Ok(())
    }
}

#[derive(Debug)]
pub struct DeploymentResult {
    pub preview_url: String,
    pub claim_url: String,
}
```

---

## Key Takeaways

1. **No-Auth Deployment** - Uses Vercel's deployment API with claim URLs
2. **Framework Detection** - Auto-detects from package.json dependencies
3. **Skill Format** - Simple SKILL.md + script structure
4. **Claude Integration** - Works with both Claude Code and claude.ai

---

## See Also

- [Main Vercel Labs Exploration](./exploration.md)
- [Claude Code Skills](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
