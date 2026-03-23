# AI Facts - Deep Dive Exploration

## Overview

**AI Facts** is a real-time fact-checking application that transcribes spoken audio and verifies statements using multiple AI providers (OpenAI + Perplexity) for cross-referencing.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs/ai-facts`

---

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Next.js App    │ →── │  Deepgram API    │ →── │  Audio Stream   │
│  (Microphone)   │     │  (Transcription) │     │  (WebSocket)    │
└────────┬────────┘     └──────────────────┘     └─────────────────┘
         │
         ↓ (transcribed text)
┌─────────────────────────────────────────────────────────────────┐
│                        Statement Parser                          │
│  "The Earth is round. Water boils at 100C. Paris is in France." │
│                          ↓                                       │
│  Split on sentence boundaries (?!. )                            │
└─────────────────────────────────────────────────────────────────┘
         │
         ↓ (individual statements)
┌─────────────────┐     ┌──────────────────┐
│  OpenAI API     │     │  Perplexity API  │
│  (Fact Check)   │     │  (Fact Check)    │
└────────┬────────┘     └────────┬─────────┘
         │                       │
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │  Cross-Reference      │
         │  Agreement = Verified │
         └───────────────────────┘
                     ↓
         ┌───────────────────────┐
         │  Live Results UI      │
         │  ✓/✗ + Explanation    │
         └───────────────────────┘
```

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Framework | Next.js App Router |
| Audio | Deepgram (real-time transcription) |
| AI/LLM | OpenAI + Perplexity |
| AI SDK | `ai` package |
| UI | shadcn/ui + Tailwind CSS |

---

## Key Implementation Details

### 1. Audio Transcription with Deepgram

**Deepgram Streaming API:**

```typescript
// Deepgram WebSocket connection
const deepgram = createClient(DEEPGRAM_API_KEY);
const connection = deepgram.listen('live', {
  model: 'nova-2',
  language: 'en',
  interim_results: true,
  punctuate: true,
  smart_format: true,
});

// Connect microphone stream
const mediaStream = await navigator.mediaDevices.getUserMedia({ audio: true });
const mediaRecorder = new MediaRecorder(mediaStream);

mediaRecorder.ondataavailable = (event) => {
  if (event.data.size > 0) {
    connection.send(event.data);
  }
};

mediaRecorder.start(250); // Send 250ms chunks
```

**Transcript Events:**

```typescript
connection.on('results', (data) => {
  const transcript = data.channel.alternatives[0].transcript;

  // Check for sentence endings
  if (/[.!?]$/.test(transcript)) {
    // Complete statement - send for fact checking
    onStatementComplete(transcript.trim());
  }
});
```

### 2. Statement Parser

```typescript
// Split transcribed text into individual statements
const parseStatements = (text: string): string[] => {
  return text
    .trim()
    .split(/(?<=[.!?])\s+/)  // Split on sentence boundaries
    .filter(s => s.length > 0);
};

// Example:
// Input: "The Earth is round. Water boils at 100C."
// Output: ["The Earth is round.", "Water boils at 100C."]
```

### 3. Fact Checking with Multiple Providers

**OpenAI Fact Check:**

```typescript
import { generateObject } from 'ai';
import { z } from 'zod';

const factCheckSchema = z.object({
  isTrue: z.boolean().describe('Whether the statement is factually accurate'),
  confidence: z.number().min(0).max(1).describe('Confidence level 0-1'),
  explanation: z.string().describe('Explanation of why true or false'),
  sources: z.array(z.string()).optional().describe('Optional source URLs'),
});

async function factCheckWithOpenAI(statement: string) {
  const { object } = await generateObject({
    model: 'openai/gpt-4o',
    schema: factCheckSchema,
    system: `You are a fact-checking assistant. Verify the accuracy of statements.
    Be skeptical and require high confidence before marking something as true.
    If you're uncertain, mark it as unverified.`,
    prompt: `Fact check this statement: "${statement}"`,
  });

  return object;
}
```

**Perplexity Fact Check:**

```typescript
async function factCheckWithPerplexity(statement: string) {
  const response = await fetch('https://api.perplexity.ai/chat/completions', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${PERPLEXITY_API_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      model: 'sonar-medium-online',
      messages: [
        {
          role: 'system',
          content: 'You are a fact-checking assistant. Verify claims and provide citations.',
        },
        {
          role: 'user',
          content: `Verify this claim and provide sources: "${statement}"`,
        },
      ],
    }),
  });

  const data = await response.json();
  return {
    isTrue: !data.choices[0].message.content.includes('false'),
    explanation: data.choices[0].message.content,
    sources: data.citations || [],
  };
}
```

### 4. Cross-Referencing Results

```typescript
interface FactCheckResult {
  statement: string;
  openAI: { isTrue: boolean; confidence: number; explanation: string };
  perplexity: { isTrue: boolean; explanation: string; sources: string[] };
  finalVerdict: 'TRUE' | 'FALSE' | 'UNVERIFIED';
  agreement: boolean;
}

function crossReferenceResults(
  openAI: FactCheckResult['openAI'],
  perplexity: FactCheckResult['perplexity']
): Omit<FactCheckResult, 'statement' | 'openAI' | 'perplexity'> {
  const agreement = openAI.isTrue === perplexity.isTrue;

  // Both must agree with high confidence for verified result
  if (agreement && openAI.confidence > 0.8) {
    return {
      finalVerdict: openAI.isTrue ? 'TRUE' : 'FALSE',
      agreement: true,
    };
  }

  // Disagreement or low confidence = unverified
  return {
    finalVerdict: 'UNVERIFIED',
    agreement: false,
  };
}
```

### 5. Real-Time Results UI

```typescript
// Chat page component with live updates
export default function ChatPage() {
  const [statements, setStatements] = useState<FactCheckResult[]>([]);
  const [isListening, setIsListening] = useState(false);

  const handleStatementComplete = async (statement: string) => {
    // Parallel fact checking
    const [openAI, perplexity] = await Promise.all([
      factCheckWithOpenAI(statement),
      factCheckWithPerplexity(statement),
    ]);

    const result = crossReferenceResults(openAI, perplexity);

    setStatements(prev => [...prev, {
      statement,
      openAI,
      perplexity,
      ...result,
    }]);
  };

  return (
    <div>
      <button onClick={toggleListening}>
        {isListening ? 'Stop' : 'Start'} Listening
      </button>

      <div className="results">
        {statements.map((result, i) => (
          <div key={i} className={`result ${result.finalVerdict}`}>
            <div className="statement">{result.statement}</div>
            <div className="verdict">
              {result.finalVerdict === 'TRUE' && '✓ True'}
              {result.finalVerdict === 'FALSE' && '✗ False'}
              {result.finalVerdict === 'UNVERIFIED' && '? Unverified'}
            </div>
            {result.agreement ? (
              <div className="explanation">
                <strong>OpenAI:</strong> {result.openAI.explanation}
                <strong>Perplexity:</strong> {result.perplexity.explanation}
              </div>
            ) : (
              <div className="disagreement">
                Providers disagree - verify manually
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
```

---

## API Routes

### Chat/Transcription Endpoint

```typescript
// app/api/chat/route.ts
import { streamText } from 'ai';
import { openai } from '@ai-sdk/openai';

export async function POST(req: Request) {
  const { messages } = await req.json();

  const result = streamText({
    model: openai('gpt-4o'),
    system: `You are a fact-checking assistant.
    - Analyze statements for factual accuracy
    - Provide clear explanations
    - Cite sources when possible
    - Mark uncertain claims as unverified`,
    messages,
  });

  return result.toTextStreamResponse();
}
```

---

## Deepgram Integration Details

### Live Transcription Options

```typescript
const connection = deepgram.listen('live', {
  // Model selection
  model: 'nova-2',  // Most accurate
  // or 'enhanced' for general use
  // or 'base' for cost savings

  // Language
  language: 'en',

  // Real-time options
  interim_results: true,  // Show partial transcripts
  punctuate: true,        // Add punctuation
  smart_format: true,     // Format numbers, dates, etc.

  // Advanced options
  diarize: false,         // Speaker diarization
  profanity_filter: true, // Filter profanity
});
```

### Event Handling

```typescript
// Partial transcripts (for live display)
connection.on('results', (data) => {
  for (const result of data.channel.alternatives) {
    if (data.is_final) {
      // Final transcript - send for fact checking
      onFinalTranscript(result.transcript);
    } else {
      // Interim transcript - show in UI
      onInterimTranscript(result.transcript);
    }
  }
});

// Connection lifecycle
connection.on('open', () => {
  console.log('Deepgram connected');
  setIsConnected(true);
});

connection.on('close', () => {
  console.log('Deepgram disconnected');
  setIsConnected(false);
});
```

---

## Environment Variables

```bash
# .env.example
OPENAI_API_KEY=sk-...
DEEPGRAM_API_KEY=...
PERPLEXITY_API_KEY=...
```

---

## File Structure

```
ai-facts/
├── app/
│   ├── api/
│   │   └── chat/
│   │       └── route.ts       # Chat API
│   ├── layout.tsx
│   ├── page.tsx               # Main UI with microphone
│   └── globals.css
├── lib/
│   ├── useFactCheck.ts        # Fact checking hooks
│   └── utils.ts
├── components/
│   └── ui/                    # shadcn/ui components
├── package.json
└── README.md
```

---

## Rust Implementation Considerations

### 1. Deepgram WebSocket Client

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{sink::SinkExt, stream::StreamExt};

struct DeepgramClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl DeepgramClient {
    async fn connect(api_key: &str) -> Result<Self> {
        let url = format!(
            "wss://api.deepgram.com/v1/listen?model=nova-2&punctuate=true&smart_format=true"
        );

        let (ws, _) = connect_async(
            format!("{}&api_key={}", url, api_key)
        ).await?;

        Ok(Self { ws })
    }

    async fn send_audio(&mut self, data: &[u8]) -> Result<()> {
        self.ws.send(Message::Binary(data.to_vec())).await?;
        Ok(())
    }

    async fn receive_transcript(&mut self) -> Result<Option<String>> {
        while let Some(msg) = self.ws.next().await {
            if let Message::Text(text) = msg? {
                let result: DeepgramResult = serde_json::from_str(&text)?;
                if result.is_final {
                    return Ok(Some(result.transcript));
                }
            }
        }
        Ok(None)
    }
}
```

### 2. Audio Capture (cpal)

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn capture_microphone() -> Result<impl Stream> {
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();
    let config = device.default_input_config()?;

    let (tx, rx) = mpsc::channel();

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Convert f32 samples to bytes
            let bytes = bytemuck::cast_slice(data).to_vec();
            tx.send(bytes).ok();
        },
        |err| eprintln!("Audio error: {}", err),
        None,
    )?;

    stream.play()?;
    Ok(stream)
}
```

### 3. Sentence Splitting

```rust
fn split_into_statements(text: &str) -> Vec<String> {
    // Regex for sentence boundaries
    let re = Regex::new(r"(?<=[.!?])\s+").unwrap();
    re.split(text)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
```

### 4. Parallel Fact Checking

```rust
use futures::future::join_all;

async fn fact_check_all(statement: &str) -> FactCheckResult {
    let tasks = vec![
        fact_check_openai(statement),
        fact_check_perplexity(statement),
    ];

    let results = join_all(tasks).await;

    let openai = results[0].as_ref().unwrap();
    let perplexity = results[1].as_ref().unwrap();

    cross_reference(openai, perplexity)
}
```

---

## Key Takeaways

1. **Real-Time Pipeline** - Audio → Deepgram → Statements → Fact Check
2. **Multi-Provider Consensus** - OpenAI + Perplexity agreement = higher confidence
3. **Sentence Boundaries** - Split on `.!?` for discrete fact-checkable units
4. **Streaming Architecture** - WebSocket for live transcription
5. **Structured Output** - Zod schemas for consistent fact-check results

---

## See Also

- [Deepgram Documentation](https://deepgram.com/)
- [Perplexity API](https://www.perplexity.ai/hub/technical-docs/api)
- [Main Vercel Labs Exploration](./exploration.md)
