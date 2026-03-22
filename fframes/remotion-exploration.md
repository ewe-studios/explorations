---
name: Remotion
description: Programmatic video creation framework using React for programmable video generation
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/remotion/
---

# Remotion - Programmatic Video Creation with React

## Overview

Remotion is a **React-based framework for creating videos programmatically**. It enables developers to leverage web technologies (CSS, Canvas, SVG, WebGL) and React's component model for video generation, providing an alternative to traditional video editing software.

Key features:
- **React composition** - Build videos using reusable React components
- **Web technologies** - Full CSS, SVG, Canvas, WebGL support
- **Programmatic control** - Use variables, functions, math for effects
- **Real-time preview** - Fast refresh and instant feedback
- **CLI rendering** - Render videos via command line
- **Lambda rendering** - Cloud rendering on AWS Lambda
- **TypeScript support** - Full type safety

## Directory Structure

```
remotion/
├── packages/
│   ├── remotion/                # Core framework
│   │   ├── src/
│   │   │   ├── composition/     # Composition primitives
│   │   │   ├── timeline/        # Timeline navigation
│   │   │   ├── animatable/      # Animation helpers
│   │   │   ├── use-current-frame/ # Frame hook
│   │   │   ├── use-video-config/ # Video config hook
│   │   │   └── index.ts
│   │   └── package.json
│   ├── @remotion/cli/           # Command-line interface
│   ├── @remotion/renderer/      # Video rendering engine
│   ├── @remotion/media-utils/   # Media utilities
│   ├── @remotion/bundler/       # Webpack bundler
│   ├── @remotion/server/        # Server for rendering
│   ├── @remotion/lambda/        # AWS Lambda rendering
│   ├── @remotion/media-player/  # Custom video player
│   └── @remotion/zod-types/     # Zod type utilities
├── packages/compositions/       # Pre-built compositions
├── docs/                        # Documentation
├── examples/                    # Example videos
├── bun.lock                     # Bun lockfile
├── package.json
├── turbo.json                   # Turborepo config
└── README.md
```

## Core Concepts

### Composition

```tsx
import { Composition } from 'remotion';
import { HelloVideo } from './HelloVideo';

export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Composition
        id="HelloVideo"
        component={HelloVideo}
        durationInFrames={300}  // 10 seconds at 30fps
        fps={30}
        width={1920}
        height={1080}
        defaultProps={{
          titleText: 'Hello World',
        }}
      />
    </>
  );
};
```

### Video Component

```tsx
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { AbsoluteFill, Sequence } from 'remotion';

interface HelloVideoProps {
  titleText: string;
}

export const HelloVideo: React.FC<HelloVideoProps> = ({ titleText }) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();

  // Calculate progress (0 to 1)
  const progress = frame / durationInFrames;

  // Animate opacity
  const opacity = frame < 30 ? frame / 30 : 1;

  return (
    <AbsoluteFill style={{ backgroundColor: 'white' }}>
      <Sequence from={0} durationInFrames={150}>
        <div style={{
          opacity,
          fontSize: 120,
          textAlign: 'center',
          marginTop: 400,
        }}>
          {titleText}
        </div>
      </Sequence>
    </AbsoluteFill>
  );
};
```

### Animation Helpers

```tsx
import { interpolate, useCurrentFrame } from 'remotion';

export const AnimatedText: React.FC<{ text: string }> = ({ text }) => {
  const frame = useCurrentFrame();

  // Interpolate value
  const scale = interpolate(frame, [0, 60], [0.5, 1], {
    extrapolateRight: 'clamp',
  });

  // Easing functions
  const easedScale = interpolate(frame, [0, 60], [0.5, 1], {
    extrapolateRight: 'clamp',
    easing: (t) => t * t, // Ease in
  });

  // Spring animation
  const spring = useSpring({
    config: { mass: 1, stiffness: 100, damping: 15 },
    to: frame > 30 ? 1 : 0,
  });

  return (
    <div style={{
      transform: `scale(${spring})`,
    }}>
      {text}
    </div>
  );
};
```

## Rendering Pipeline

### CLI Rendering

```bash
# Render video
npx remotion render src/index.tsx HelloVideo output.mp4

# Render with options
npx remotion render \
  src/index.tsx \
  HelloVideo \
  output.mp4 \
  --fps=30 \
  --codec=h264 \
  --crf=25 \
  --frames=1-300

# Render image sequence
npx remotion render src/index.tsx HelloVideo ./frames/frame-.png

# Render GIF
npx remotion render src/index.tsx HelloVideo output.gif
```

### Programmatic Rendering

```tsx
import { renderVideo } from '@remotion/renderer';

await renderVideo({
  id: 'HelloVideo',
  componentName: 'HelloVideo',
  inputProps: {
    titleText: 'Hello World',
  },
  outputLocation: 'output.mp4',
  fps: 30,
  width: 1920,
  height: 1080,
  durationInFrames: 300,
  codec: 'h264',
  crf: 25,
  onProgress: (progress) => {
    console.log(`${Math.round(progress * 100)}% complete`);
  },
});
```

### Server-Based Rendering

```tsx
import { renderMedia } from '@remotion/renderer';
import { createRemotionServer } from '@remotion/server';

const server = createRemotionServer({
  port: 3000,
  maxConcurrency: 4,
});

server.use('/render', async (req, res) => {
  const result = await renderMedia({
    composition: 'HelloVideo',
    inputProps: req.body.props,
    outputLocation: `outputs/${Date.now()}.mp4`,
  });

  res.json({ output: result.outputLocation });
});
```

## Lambda Rendering

### Configuration

```tsx
// remotion-lambda.config.ts
import { Region } from '@remotion/lambda';

export const REGION: Region = 'us-east-1';
export const FUNCTION_NAME = 'remotion-renderer';
export const MEMORY_SIZE = 2048;
export const TIMEOUT = 900; // 15 minutes
```

### Render on Lambda

```tsx
import { renderMediaOnLambda } from '@remotion/lambda';

const result = await renderMediaOnLambda({
  functionName: FUNCTION_NAME,
  region: REGION,
  id: 'HelloVideo',
  inputProps: { titleText: 'Hello' },
  codec: 'h264',
  outputBucket: 'my-video-bucket',
  outputKey: 'videos/output.mp4',
  fps: 30,
  width: 1920,
  height: 1080,
  durationInFrames: 300,
});

console.log(`Video rendered to s3://${result.bucket}/${result.key}`);
```

## Advanced Features

### Sequence and Layers

```tsx
import { Sequence, AbsoluteFill, Continue } from 'remotion';

export const MultiSceneVideo: React.FC = () => {
  return (
    <AbsoluteFill>
      {/* Scene 1: 0-120 frames */}
      <Sequence from={0} durationInFrames={120}>
        <IntroScene />
      </Sequence>

      {/* Scene 2: 60-180 frames (overlaps with Scene 1) */}
      <Sequence from={60} durationInFrames={120}>
        <MainContent />
      </Sequence>

      {/* Scene 3: continues until end */}
      <Sequence from={180}>
        <Continue durationInFrames={60}>
          <OutroScene />
        </Continue>
      </Sequence>

      {/* Overlay on top of all scenes */}
      <Sequence from={0}>
        <Watermark />
      </Sequence>
    </AbsoluteFill>
  );
};
```

### Audio Handling

```tsx
import { AbsoluteFill, Audio, useAudioData, visualizeAudio } from 'remotion';

export const VideoWithAudio: React.FC = () => {
  const audioData = useAudioData('/music.mp3');

  if (!audioData) {
    return null;
  }

  const frequency = visualizeAudio({
    fftSamples: 64,
    audioData,
    channelData: 'left',
  });

  return (
    <AbsoluteFill>
      <Audio src="/music.mp3" />

      {/* Audio visualization */}
      <div style={{ display: 'flex', gap: 4 }}>
        {frequency.map((value, i) => (
          <div
            key={i}
            style={{
              height: value * 200,
              width: 20,
              background: 'blue',
            }}
          />
        ))}
      </div>
    </AbsoluteFill>
  );
};
```

### Media Management

```tsx
import { staticFile, useCurrentFrame } from 'remotion';
import { Img, Video } from 'remotion';

export const MediaComposition: React.FC = () => {
  const frame = useCurrentFrame();

  return (
    <div>
      {/* Static image */}
      <Img src={staticFile('/logo.png')} style={{ width: 200 }} />

      {/* Video with controls */}
      <Video
        src={staticFile('/background.mp4')}
        startFrom={30}
        endAt={150}
        style={{ opacity: 0.5 }}
      />

      {/* Conditional media */}
      {frame > 60 && (
        <Img
          src={staticFile('/overlay.png')}
          style={{
            opacity: interpolate(frame, [60, 90], [0, 1]),
          }}
        />
      )}
    </div>
  );
};
```

### Custom Hooks

```tsx
import { useCurrentFrame, useVideoConfig } from 'remotion';

// Hook for spring animation
export const useSpringValue = (config: {
  mass: number;
  stiffness: number;
  damping: number;
  to: number;
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  // Spring physics simulation
  let position = 0;
  let velocity = 0;

  for (let i = 0; i < frame; i++) {
    const dt = 1 / fps;
    const acceleration = (config.to - position) * config.stiffness / config.mass;
    velocity += acceleration * dt;
    velocity *= 1 - config.damping / config.mass;
    position += velocity * dt;
  }

  return position;
};

// Usage
const scale = useSpringValue({
  mass: 1,
  stiffness: 100,
  damping: 15,
  to: 1,
});
```

## Project Structure

```
my-video-project/
├── src/
│   ├── index.tsx              # Root with Composition definitions
│   ├── HelloVideo.tsx         # Main video component
│   ├── components/
│   │   ├── Intro.tsx          # Reusable intro
│   │   ├── Title.tsx          # Title card
│   │   ├── Outro.tsx          # Outro
│   │   └── Watermark.tsx      # Overlay
│   ├── styles/
│   │   └── globals.css        # Global styles
│   └── assets/
│       ├── images/
│       ├── videos/
│       └── audio/
├── package.json
├── tsconfig.json
├── remotion.config.ts         # Remotion configuration
└── output/                    # Rendered videos
```

## Integration with FFrames

The Remotion project in FFrames context represents:
1. **Reference implementation** for programmatic video generation
2. **Comparison point** for React-based vs Rust-based video generation
3. **Design inspiration** for timeline/sequence abstractions

### Key Takeaways for Rust Implementation

```rust
// Rust equivalent concepts for FFrames
trait Composition {
    fn duration_in_frames(&self) -> usize;
    fn render_frame(&self, frame: usize, ctx: &Context) -> Svg;
}

struct Sequence {
    from: usize,
    duration: usize,
    composition: Box<dyn Composition>,
}

impl Composition for Sequence {
    fn render_frame(&self, frame: usize, ctx: &Context) -> Svg {
        if frame < self.from {
            Svg::empty()
        } else {
            self.composition.render_frame(frame - self.from, ctx)
        }
    }
}
```

## Performance Optimization

### Bundle Splitting

```tsx
// Dynamic import for heavy components
const HeavyComponent = dynamic(() => import('./HeavyComponent'), {
  ssr: false,
});

// Only load when needed
{frame > 100 && <HeavyComponent />}
```

### Memoization

```tsx
import { memo, useMemo } from 'react';

const ExpensiveComponent = memo(({ data }) => {
  const processed = useMemo(() => {
    return expensiveProcessing(data);
  }, [data]);

  return <div>{processed}</div>;
});
```

### Parallel Rendering

Remotion automatically parallelizes frame rendering:
- Each frame is rendered independently
- Frames can be rendered in any order
- Lambda rendering uses hundreds of concurrent instances

## Related Documents

- [FFrames Core](./fframes-core-exploration.md) - Rust video framework
- [FFrames Editor](./fframes-editor-exploration.md) - Web-based editor

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/remotion/`
- Remotion Documentation: https://remotion.dev/docs
- Remotion GitHub: https://github.com/remotion-dev/remotion
