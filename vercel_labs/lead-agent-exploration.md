# Lead Agent - Deep Dive Exploration

## Overview

**Lead Agent** is an inbound lead qualification and research agent that demonstrates durable workflows, AI-powered qualification, and human-in-the-loop approvals via Slack.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs/lead-agent`

---

## Architecture

```
┌─────────────────┐
│  User submits   │
│  contact form   │
└────────┬────────┘
         │
         ↓
┌─────────────────────────────────────────────────────────────┐
│                    Lead Agent Workflow                       │
│  (Workflow DevKit - Durable Execution)                       │
│                                                              │
│  ┌──────────────────┐    ┌──────────────────┐               │
│  │  Step: Research  │ →  │  Step: Qualify   │               │
│  │  (AI Agent)      │    │  (generateObject)│               │
│  └──────────────────┘    └────────┬─────────┘               │
│                                   │                          │
│                    ┌──────────────┴──────────────┐          │
│                    ↓                              ↓          │
│         ┌─────────────────┐           ┌─────────────────┐   │
│         │  QUALIFIED/     │           │  NOT QUALIFIED  │   │
│         │  FOLLOW_UP      │           │  (other action) │   │
│         └────────┬────────┘           └─────────────────┘   │
│                  │                                          │
│         ┌────────┴────────┐                                │
│         ↓                 ↓                                │
│  ┌─────────────┐   ┌──────────────┐                       │
│  │ Step: Write │   │ Step: Human  │                       │
│  │ Email       │   │ Feedback     │                       │
│  │ (generateText)  │ (Slack w/ buttons)                   │
│  └─────────────┘   └──────────────┘                       │
│                           │                                │
│                           ↓                                │
│                  ┌─────────────────┐                       │
│                  │  Slack Approve  │                       │
│                  │  or Reject      │                       │
│                  └────────┬────────┘                       │
│                           │                                │
│                           ↓                                │
│                  ┌─────────────────┐                       │
│                  │  Send Email     │                       │
│                  │  (if approved)  │                       │
│                  └─────────────────┘                       │
└─────────────────────────────────────────────────────────────┘
```

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Framework | Next.js 16 (App Router) |
| Workflows | Workflow DevKit (`use workflow`) |
| AI | Vercel AI SDK + AI Gateway |
| Human-in-Loop | Slack Bolt + Vercel Slack Adapter |
| Web Search | Exa.ai |
| Deployment | Vercel AI Cloud |

---

## Key Implementation Details

### 1. Workflow Definition (`workflows/inbound/index.ts`)

```typescript
import { FormSchema } from '@/lib/types';
import {
  stepHumanFeedback,
  stepQualify,
  stepResearch,
  stepWriteEmail
} from './steps';

/**
 * Workflow to handle inbound lead
 */
export const workflowInbound = async (data: FormSchema) => {
  'use workflow';  // Workflow DevKit directive

  // Step 1: Research the lead
  const research = await stepResearch(data);

  // Step 2: Qualify the lead
  const qualification = await stepQualify(data, research);

  // Step 3: Conditional branching
  if (
    qualification.category === 'QUALIFIED' ||
    qualification.category === 'FOLLOW_UP'
  ) {
    const email = await stepWriteEmail(research, qualification);
    await stepHumanFeedback(research, email, qualification);
  }

  // Handle other qualification categories
  // (SUPPORT, PARTNERSHIP, etc.)
};
```

**Workflow DevKit Features:**
- `'use workflow'` - Marks function as durable workflow
- `'use step'` - Marks individual steps (in step files)
- Automatic state checkpointing
- Webhook correlation for async steps

### 2. Research Agent (`lib/services.ts`)

```typescript
import { Experimental_Agent as Agent, stepCountIs, tool } from 'ai';
import { z } from 'zod';

// Search tool
const search = tool({
  description: 'Search the web for information',
  inputSchema: z.object({
    keywords: z.string().describe('Entity to search for'),
    resultCategory: z.enum([
      'company', 'research paper', 'news', 'pdf',
      'github', 'tweet', 'personal site', 'linkedin profile', 'financial report'
    ]),
  }),
  execute: async ({ keywords, resultCategory }) => {
    const result = await exa.searchAndContents(keywords, {
      numResults: 2,
      type: 'keyword',
      category: resultCategory,
      summary: true,
    });
    return result;
  },
});

// Fetch URL tool
const fetchUrl = tool({
  description: 'Return visible text from a public URL as Markdown.',
  inputSchema: z.object({
    url: z.string().describe('Absolute URL, including http:// or https://'),
  }),
  execute: async ({ url }) => {
    const result = await exa.getContents(url, { text: true });
    return result;
  },
});

// CRM Search tool
const crmSearch = tool({
  description: 'Search existing CRM for opportunities by company name',
  inputSchema: z.object({
    name: z.string().describe('Company name (e.g. "Vercel")'),
  }),
  execute: async ({ name }) => {
    // Fetch from Salesforce, HubSpot, etc.
    return [];
  },
});

// Tech stack analysis tool
const techStackAnalysis = tool({
  description: 'Return tech stack analysis for a domain.',
  inputSchema: z.object({
    domain: z.string().describe('Domain, e.g. "vercel.com"'),
  }),
  execute: async ({ domain }) => {
    // BuiltWith, Wappalyzer, etc.
    return [];
  },
});

// Create the research agent
export const researchAgent = new Agent({
  model: 'openai/gpt-5',
  system: `
You are a researcher to find information about a lead.

Available tools:
- search: Searches the web for information
- fetchUrl: Fetches contents of public URLs
- crmSearch: Searches the CRM for company info
- techStackAnalysis: Analyzes tech stack of domains

Synthesize findings into a comprehensive report.
`,
  tools: {
    search,
    fetchUrl,
    crmSearch,
    techStackAnalysis,
  },
  stopWhen: [stepCountIs(20)],  // Max 20 tool calls
});
```

### 3. Lead Qualification (`lib/services.ts`)

```typescript
import { generateObject } from 'ai';
import { FormSchema, QualificationSchema, qualificationSchema } from '@/lib/types';

export async function qualify(
  lead: FormSchema,
  research: string
): Promise<QualificationSchema> {
  const { object } = await generateObject({
    model: 'openai/gpt-5',
    schema: qualificationSchema,
    prompt: `Qualify the lead based on:

    LEAD DATA: ${JSON.stringify(lead)}

    RESEARCH: ${research}`,
  });

  return object;
}
```

**Qualification Schema (`lib/types.ts`):**

```typescript
import { z } from 'zod';

export const qualificationCategorySchema = z.enum([
  'QUALIFIED',      // Ready to buy
  'FOLLOW_UP',      // Potential, needs nurturing
  'SUPPORT',        // Existing customer needs help
  'PARTNERSHIP',    // Partnership inquiry
  'INVESTOR',       // Investor inquiry
  'PRESS',          // Media inquiry
  'NOT_QUALIFIED',  // Not a fit
]);

export const qualificationSchema = z.object({
  category: qualificationCategorySchema,
  reason: z.string().describe('Explanation for the qualification decision'),
  priority: z.enum(['HIGH', 'MEDIUM', 'LOW']).optional(),
  nextSteps: z.array(z.string()).optional(),
});

export type QualificationSchema = z.infer<typeof qualificationSchema>;
```

### 4. Email Generation (`lib/services.ts`)

```typescript
import { generateText } from 'ai';

export async function writeEmail(
  research: string,
  qualification: QualificationSchema
) {
  const { text } = await generateText({
    model: 'openai/gpt-5',
    prompt: `Write an email for a ${qualification.category} lead:

    Research: ${JSON.stringify(research)}
    Category: ${qualification.category}
    Reason: ${qualification.reason}`,
  });

  return text;
}
```

### 5. Human-in-the-Loop via Slack (`lib/services.ts`)

```typescript
import { sendSlackMessageWithButtons } from '@/lib/slack';

export async function humanFeedback(
  research: string,
  email: string,
  qualification: QualificationSchema
) {
  const message = `*New Lead Qualification*

*Email:* ${email}
*Category:* ${qualification.category}
*Reason:* ${qualification.reason}

*Research:*
${research.slice(0, 500)}...

*Please review and approve or reject this email*`;

  const slackChannel = process.env.SLACK_CHANNEL_ID || '';

  return await sendSlackMessageWithButtons(slackChannel, message);
}
```

**Slack Integration (`lib/slack.ts`):**

```typescript
import { WebClient } from '@slack/web-api';
import { SlackMessageResponse } from '@/lib/types';

const slack = new WebClient(process.env.SLACK_BOT_TOKEN);

export async function sendSlackMessageWithButtons(
  channel: string,
  message: string
): Promise<SlackMessageResponse> {
  const result = await slack.chat.postMessage({
    channel,
    text: message,
    blocks: [
      {
        type: 'section',
        text: {
          type: 'mrkdwn',
          text: message,
        },
      },
      {
        type: 'actions',
        elements: [
          {
            type: 'button',
            text: { type: 'plain_text', text: '✓ Approve' },
            value: 'approve',
            style: 'primary',
            action_id: 'approve_email',
          },
          {
            type: 'button',
            text: { type: 'plain_text', text: '✗ Reject' },
            value: 'reject',
            style: 'danger',
            action_id: 'reject_email',
          },
        ],
      },
    ],
  });

  return { ts: result.ts, channel: result.channel };
}
```

### 6. Workflow Steps (`workflows/inbound/steps.ts`)

```typescript
import {
  humanFeedback,
  qualify,
  researchAgent,
  writeEmail
} from '@/lib/services';

/**
 * Step: Research the lead using AI agent
 */
export const stepResearch = async (data: FormSchema) => {
  'use step';  // Workflow DevKit step directive

  const { text: research } = await researchAgent.generate({
    prompt: `Research this lead: ${JSON.stringify(data)}`,
  });

  return research;
};

/**
 * Step: Qualify the lead
 */
export const stepQualify = async (data: FormSchema, research: string) => {
  'use step';

  const qualification = await qualify(data, research);
  return qualification;
};

/**
 * Step: Write personalized email
 */
export const stepWriteEmail = async (
  research: string,
  qualification: QualificationSchema
) => {
  'use step';

  const email = await writeEmail(research, qualification);
  return email;
};

/**
 * Step: Human feedback via Slack
 */
export const stepHumanFeedback = async (
  research: string,
  email: string,
  qualification: QualificationSchema
) => {
  'use step';

  if (!process.env.SLACK_BOT_TOKEN || !process.env.SLACK_SIGNING_SECRET) {
    console.warn('Slack not configured, skipping human feedback');
    return;
  }

  const slackMessage = await humanFeedback(research, email, qualification);
  return slackMessage;
};
```

---

## Slack Webhook Handler

### API Route (`app/api/slack/route.ts`)

```typescript
import { handleSlackInteraction } from '@/lib/slack';
import { NextRequest } from 'next/server';

export async function POST(req: NextRequest) {
  const body = await req.text();
  const signature = req.headers.get('x-slack-signature') || '';
  const timestamp = req.headers.get('x-slack-request-timestamp') || '';

  // Verify Slack signature
  const isValid = await verifySlackSignature(body, signature, timestamp);
  if (!isValid) {
    return new Response('Invalid signature', { status: 401 });
  }

  const payload = JSON.parse(body);

  // Handle button clicks (interactions)
  if (payload.type === 'block_actions') {
    const action = payload.actions[0];
    const responseUrl = payload.response_url;

    if (action.action_id === 'approve_email') {
      // Resume workflow - approved
      await resumeWorkflow(payload, 'approved');
      return new Response(null, { status: 200 });
    }

    if (action.action_id === 'reject_email') {
      // Resume workflow - rejected
      await resumeWorkflow(payload, 'rejected');
      return new Response(null, { status: 200 });
    }
  }

  return new Response('OK', { status: 200 });
}
```

### Workflow Resumption

```typescript
import { WorkflowClient } from 'workflow-devkit';

const workflowClient = new WorkflowClient();

async function resumeWorkflow(
  slackPayload: any,
  decision: 'approved' | 'rejected'
) {
  // Extract workflow ID from slack payload metadata
  const workflowId = extractWorkflowId(slackPayload);

  // Resume workflow with decision
  await workflowClient.resume(workflowId, {
    action: decision,
    slackUser: slackPayload.user.name,
    timestamp: new Date().toISOString(),
  });

  // Update Slack message with result
  await updateSlackMessage(slackPayload, decision);
}
```

---

## Project Structure

```
lead-agent/
├── app/
│   ├── api/
│   │   ├── submit/
│   │   │   └── route.ts       # Form submission endpoint
│   │   └── slack/
│   │       └── route.ts       # Slack webhook handler
│   ├── page.tsx               # Home page with form
│   └── layout.tsx
├── components/
│   └── lead-form.tsx          # Contact form component
├── lib/
│   ├── services.ts            # Core business logic
│   ├── slack.ts               # Slack integration
│   ├── types.ts               # TypeScript schemas
│   └── exa.ts                 # Exa.ai client
├── workflows/
│   └── inbound/
│       ├── index.ts           # Main workflow
│       └── steps.ts           # Workflow steps
├── manifest.json              # Slack app manifest
├── package.json
└── README.md
```

---

## Environment Variables

```bash
# .env.local
AI_GATEWAY_API_KEY=...
SLACK_BOT_TOKEN=xoxb-...
SLACK_SIGNING_SECRET=...
SLACK_CHANNEL_ID=C01234567890
EXA_API_KEY=...
```

---

## Slack App Manifest

```json
{
  "display_information": {
    "name": "Lead Agent",
    "description": "Lead qualification and approval bot"
  },
  "features": {
    "bot_user": {
      "display_name": "Lead Agent",
      "always_online": false
    }
  },
  "oauth_config": {
    "scopes": {
      "bot": [
        "chat:write",
        "commands",
        "incoming-webhook"
      ]
    }
  },
  "settings": {
    "interactivity": {
      "is_enabled": true
    },
    "org_deploy_enabled": false,
    "socket_mode_enabled": false
  }
}
```

---

## Rust Implementation Considerations

### 1. Workflow Engine Trait

```rust
#[async_trait]
pub trait WorkflowEngine: Send + Sync {
    async fn start<T: Workflow + Send + Sync>(
        &self,
        workflow: T,
        input: T::Input,
    ) -> Result<WorkflowHandle>;

    async fn resume(
        &self,
        workflow_id: &str,
        event: WorkflowEvent,
    ) -> Result<()>;

    async fn get_state(&self, workflow_id: &str) -> Result<WorkflowState>;
}

pub trait Workflow: Sized {
    type Input: serde::de::DeserializeOwned;
    type Output: serde::Serialize;

    async fn run(&self, ctx: WorkflowContext, input: Self::Input)
        -> Result<Self::Output>;
}
```

### 2. Step Abstraction

```rust
#[async_trait]
pub trait Step: Send + Sync {
    type Input: serde::de::DeserializeOwned;
    type Output: serde::Serialize;

    async fn execute(&self, input: Self::Input) -> Result<Self::Output>;

    fn name(&self) -> &'static str;
}

// Example: Research step
struct ResearchStep {
    agent: Arc<ResearchAgent>,
}

#[async_trait]
impl Step for ResearchStep {
    type Input = FormSchema;
    type Output = String;

    async fn execute(&self, input: Self::Input) -> Result<String> {
        let research = self.agent
            .generate(&format!("Research this lead: {:?}", input))
            .await?;
        Ok(research)
    }

    fn name(&self) -> &'static str {
        "research"
    }
}
```

### 3. Durable Execution State Machine

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WorkflowState {
    Pending,
    Running { current_step: String },
    WaitingForCallback { callback_id: String },
    Completed { output: serde_json::Value },
    Failed { error: String },
}

pub struct WorkflowContext {
    pub workflow_id: String,
    pub state: WorkflowState,
    pub checkpoint_store: Arc<dyn CheckpointStore>,
}

impl WorkflowContext {
    pub async fn checkpoint(&mut self, state: WorkflowState) -> Result<()> {
        self.checkpoint_store
            .save(&self.workflow_id, &state)
            .await?;
        self.state = state;
        Ok(())
    }

    pub async fn wait_for_callback(&mut self, callback_id: String) -> Result<CallbackResult> {
        self.checkpoint(WorkflowState::WaitingForCallback { callback_id }).await?;

        // Return control to engine - will be resumed via webhook
        // This is a "yield" point in the workflow
        Err(Error::YieldedForCallback(callback_id))
    }
}
```

### 4. AI Agent with Tools

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: JsonSchema,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value>;
}

// Search tool implementation
struct SearchTool {
    exa_client: ExaClient,
}

#[async_trait]
impl Tool for SearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search".to_string(),
            description: "Search the web for information".to_string(),
            input_schema: json_schema!({
                "type": "object",
                "properties": {
                    "keywords": {"type": "string"},
                    "category": {"type": "string", "enum": ["company", "news", "paper"]}
                },
                "required": ["keywords"]
            }),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        #[derive(Deserialize)]
        struct SearchInput {
            keywords: String,
            category: Option<String>,
        }

        let input: SearchInput = serde_json::from_value(input)?;
        let result = self.exa_client.search(&input.keywords, input.category.as_deref()).await?;
        Ok(serde_json::to_value(result)?)
    }
}
```

### 5. Slack Integration

```rust
use slack_sdk::WebClient;
use slack_sdk::models::blocks::{Block, SectionBlock, ActionsBlock, Button};

pub struct SlackNotifier {
    client: WebClient,
    channel: String,
}

impl SlackNotifier {
    pub async fn send_approval_request(
        &self,
        email: &str,
        category: &str,
        research: &str,
    ) -> Result<SlackMessage> {
        let blocks = vec![
            Block::Section(SectionBlock::builder()
                .text(format!(
                    "*New Lead Qualification*\n\n\
                     *Category:* {}\n\
                     *Email:* {}\n\n\
                     *Research:*\n{}",
                    category, email, &research[..500.min(research.len())]
                ))
                .build()),
            Block::Actions(ActionsBlock::builder()
                .elements(vec![
                    slack_sdk::models::blocks::Element::Button(
                        Button::builder()
                            .text("✓ Approve")
                            .value("approve")
                            .action_id("approve_email")
                            .style(slack_sdk::models::blocks::ButtonStyle::Primary)
                            .build()
                    ),
                    slack_sdk::models::blocks::Element::Button(
                        Button::builder()
                            .text("✗ Reject")
                            .value("reject")
                            .action_id("reject_email")
                            .style(slack_sdk::models::blocks::ButtonStyle::Danger)
                            .build()
                    ),
                ])
                .build()),
        ];

        let response = self.client
            .chat_post_message()
            .channel(&self.channel)
            .blocks(blocks)
            .send()
            .await?;

        Ok(SlackMessage {
            ts: response.ts,
            channel: response.channel,
        })
    }
}
```

### 6. Webhook Handler for Slack

```rust
use axum::{
    extract::Json,
    http::StatusCode,
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub async fn slack_webhook(
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, AppError> {
    // Verify Slack signature
    let signature = headers.get("x-slack-signature")
        .ok_or(AppError::MissingSignature)?;
    let timestamp = headers.get("x-slack-request-timestamp")
        .ok_or(AppError::MissingTimestamp)?;

    verify_slack_signature(&body, signature, timestamp)?;

    // Parse payload
    let payload: SlackInteractionPayload = serde_json::from_str(&body)?;

    match payload.r#type {
        InteractionType::BlockActions => {
            let action = payload.actions.first().unwrap();

            match action.action_id.as_str() {
                "approve_email" => {
                    resume_workflow(&payload, WorkflowDecision::Approved).await?;
                }
                "reject_email" => {
                    resume_workflow(&payload, WorkflowDecision::Rejected).await?;
                }
                _ => {}
            }
        }
        _ => {}
    }

    Ok(StatusCode::OK)
}

fn verify_slack_signature(
    body: &str,
    signature: &HeaderValue,
    timestamp: &HeaderValue,
) -> Result<(), AppError> {
    let signing_secret = std::env::var("SLACK_SIGNING_SECRET")?;
    let base = format!("v0:{}:{}", timestamp.to_str()?, body);

    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())?;
    mac.update(base.as_bytes());

    let expected = format!("v0={}", hex::encode(mac.finalize().into_bytes()));
    let actual = signature.to_str()?;

    if actual == expected {
        Ok(())
    } else {
        Err(AppError::InvalidSignature)
    }
}
```

---

## Key Takeaways

1. **Durable Workflows** - `'use workflow'` for multi-step processes with state
2. **AI Agent Pattern** - Tool-based autonomous research agent
3. **Structured Output** - `generateObject` with Zod schemas for qualification
4. **Human-in-the-Loop** - Slack approval with workflow resumption
5. **Conditional Branching** - Different paths based on qualification category
6. **Webhook Correlation** - Slack callbacks resume specific workflow instances

---

## See Also

- [Workflow DevKit](https://useworkflow.dev/)
- [Vercel Slack Adapter](https://github.com/vercel-labs/slack-bolt)
- [Main Vercel Labs Exploration](./exploration.md)
