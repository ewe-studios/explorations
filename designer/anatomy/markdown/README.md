# Anatomy — deep documentation

This series documents the `anatomy` project at `/Users/darkvoid/GitSources/src.Designer/anatomy`.

`anatomy` is a Claude Agent Skill (MIT license). It turns a concept into a crafted, interactive isometric figure of a real physical object. A figure is built with a Node script, proved with a geometry audit, checked in headless Chrome and, when asked, printed or published to a gallery.

The docs follow a numbered chain. Start at `00-overview.md`. Each page ends with a link to the next. This page is the index and the end of the chain.

## Documents

| Doc | Title | Summary |
|---|---|---|
| [00-overview.md](00-overview.md) | Overview: an exploded-view skill | What the skill is, when to use it, the size tiers and what ships. |
| [01-pipeline.md](01-pipeline.md) | Pipeline: prompt to published figure | The ten-step workflow and the capture, build, verify and publish toolchain. |
| [02-isometric-core.md](02-isometric-core.md) | Isometric core: iso-kit | Camera math, depth keys and four-tone shading. |
| [03-lathe.md](03-lathe.md) | Lathe: round parts on any axis | Frames, meridians, Hermite splines and smooth shading. |
| [04-tube.md](04-tube.md) | Tube: pipes, hoses and cables | Routes, chunking, gaps, rings and stripes. |
| [05-audit.md](05-audit.md) | Audit: geometry proof with signed distance fields | Recorder, depth keys and every clearance and routing check. |
| [06-webgl.md](06-webgl.md) | WebGL: shaders inside the drawing | The layer stack, camera contract and envelopes. |
| [07-turn.md](07-turn.md) | Turn: three-dimensional turning groups | The 3D builder, runtime and orbit audit (beta). |
| [08-react.md](08-react.md) | React: server drawing and client hooks | The server/client split, components and hooks. |
| [09-references.md](09-references.md) | References: the craft library | What each of the ten reference documents teaches. |
| [10-capture-verify.md](10-capture-verify.md) | Capture and verify: headless Chrome | Capture, drive, line ends and 3D checks. |
| [11-publish-print.md](11-publish-print.md) | Publish and print | Gallery submission and A4 sheets. |
| [12-examples.md](12-examples.md) | Examples: nine worked figures | What each shipped example shows. |
| [13-tests-evals.md](13-tests-evals.md) | Tests and evals | The node test suite and the eval fixtures. |

## Facts to hold onto

- The core projection is `iso([x, y, z], P)`. It maps world points to screen points with azimuth 45 degrees and elevation 30 degrees by default.
- Every figure is one Node build script. The script writes a standalone HTML page next to itself.
- The audit is not advice. `node build.mjs --audit` fails the build when geometry passes through geometry, an end floats or a bend pinches.
- Verification is headless Chrome. The same scripts that capture a figure also drive it, check its lines and measure its performance.
