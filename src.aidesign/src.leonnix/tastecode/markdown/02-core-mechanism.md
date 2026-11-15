# Core mechanism: the Design Mode phase pipeline

Design Mode turns one user request into a plan and then into a built web page. The
server orchestrates the run. The internal design agent supplies the phase contracts.
The pipeline is a sequence of model turns. Each turn returns typed JavaScript Object
Notation (JSON). The server validates that JSON before it advances.

This document follows the pipeline from `packages/design-agent`. It explains how a
brief becomes ordered phases, what each phase may decide, and how the server enforces
the boundary between phases.

## The phase machine

The server stores the design flow as a record with a phase field. The phases are a fixed
union. The union is also a Zod schema, so a stored record is checked on load.

```text
# apps/server/src/orchestrator.ts:327-337
type DesignFlowPhase =
  | 'brief'
  | 'brand'
  | 'page'
  | 'assets'
  | 'build'
  | 'preview'
  | 'review'
  | 'repair'
  | 'complete'
  | 'response'
```

The phases are not all model turns. `preview`, `review`, `repair`, and `complete` involve
server work or a later review turn. `response` is the handover back to a normal thread
turn. The stored flow also carries approved artifacts, the reference deck, screenshots,
the repair attempt count, and correction state.

The next diagram shows the normal path. The dashed edges are bounded recovery paths.

```mermaid
stateDiagram-v2
  [*] --> brief
  brief --> brand: brief validates
  brand --> page: brand validates
  page --> assets: page validates
  assets --> page: unresolved assets, one replan
  assets --> build: assets resolve
  build --> preview: build validates
  preview --> review: preview starts and capture succeeds
  review --> repair: verdict repair and limit not reached
  repair --> review: repaired page recaptured
  review --> complete: verdict pass or repair limit reached
  preview --> complete: capture unavailable
  complete --> response
  response --> [*]
```

The brief is a hard gate. No website implementation starts before `brief.json` passes
validation. The design agent document calls this rule out at `docs/DESIGN-AGENT.md:105-110`.
The rule prevents a model from building a page before the request is classified.

The server bounds every correction path. Planning phases retry up to three distinct
validation errors. Asset acquisition can continue for up to six corrections while each
pass reduces the unresolved set. Build, Preview, and Repair are not planning phases.
Each of them gets one correction. Review can run at most two repair attempts.

```text
# apps/server/src/orchestrator.ts:350-354
const DesignBriefInputSchema = z.record(z.string(), JsonValueSchema)
/** Planning phases retry up to three distinct validation errors. */
const MAX_PLANNING_CORRECTIONS = 3
/** Photo acquisition that keeps resolving assets may continue, but never without bound. */
const MAX_ASSET_ACQUISITION_CORRECTIONS = 6
```

The repair limit is a separate constant at `apps/server/src/orchestrator.ts:712`. The
value is 2. The completion message uses the same constant. A run cannot loop forever.

## The artifact graph

The phase pipeline has five stored artifacts and several outputs that are not
artifacts. The distinction matters. A `.taste` artifact is a typed JavaScript Object
Notation (JSON) file with a fixed name. Build and Repair change project files. Preview
records a plan and screenshots in the flow state.

| Phase | Output | `.taste` artifact |
|---|---|---|
| Brief | validated complete brief | `brief.json` |
| Brand | brand system | `brand.json` |
| Page | page plan | `page.json` |
| Assets | asset manifest | `assets.json` |
| Build | project files | none |
| Preview | preview plan and screenshots | none |
| Review | verdict and findings | `review.json` |
| Repair | project files | none |
| Complete | completion summary | none |
| Response | final answer | none |

The dependencies between the stored artifacts form a directed graph. The graph is the
real interface between phases.

```mermaid
flowchart LR
  R[User request] --> B[brief.json]
  B --> BR[brand.json]
  B --> P[page.json]
  BR --> P
  P --> A[assets.json]
  BR --> A
  B --> A
  A --> BU[Build files]
  BU --> PV[Preview plan]
  PV --> S[Screenshots and document audit]
  S --> RV[review.json]
  RV --> RP[Repair files]
  RP --> S
```

The artifacts live under `.taste/` in the project workspace. The names are `brief.json`,
`brand.json`, `page.json`, `assets.json`, and `review.json`. A shared reader and writer
module owns the paths and the size limit.

```text
# packages/design-agent/src/artifact-store.ts:6-14
const MAX_ARTIFACT_BYTES = 2_000_000
type ArtifactName = 'brief.json' | 'brand.json' | 'page.json' | 'assets.json' | 'review.json'

function artifactPath(workspacePath: string, name: ArtifactName, create: boolean): string {
  const root = realpathSync(workspacePath)
  const directory = path.join(root, '.taste')
  if (create) mkdirSync(directory, { recursive: true })
  if (!lstatSync(directory).isDirectory() || realpathSync(directory) !== directory) {
    throw new Error('Design artifact directory must be a real directory inside the workspace')
```

The writer uses a temporary file with the `wx` flag and then calls `renameSync`. The
write is atomic. The temporary file has mode `0o600`. The rename also avoids writing
through a symbolic link or hard link that the model created. The reader checks the size
limit before it parses the file.

## Phase one: the brief

The brief phase is a classification and extraction step. It is not a design step. The
prompt forbids building, scaffolding, and editing.

```text
# packages/design-agent/src/workflow.ts:35-42
export function designBriefingPrompt(request: string): string {
  return `You are running TasteCode Design Briefing mode.

This is a focused classification and extraction step. Read the supplied request first. For an existing website, inspect only the relevant project files, brand tokens, assets and current page structure needed to identify existing style and constraints. Do not browse or invoke external design skills. For a new site, extract the brief directly without unnecessary tool work. Do not describe your reasoning.

This turn may only advance a design brief. Do not build, scaffold, edit, or generate a website, brand system, asset set, component, or implementation. TasteCode persists the brief and automatically continues the design.

First decide whether the request is primarily about designing or redesigning a website, web page, landing page, portfolio, or product interface. The user already selected Design mode, so terse visual intent such as "Make it pop" refers to the existing page, or a new landing page when none exists. Return "not_design" only when the request is clearly unrelated to website or interface design.
```

The type has three statuses. `complete` carries the brief. `not_design` is a refusal for
an unrelated request. `questions` is an unused status that carries a question list. The
prompt tells the model to work autonomously and never ask questions. It also requires an
empty `questions` array in a `complete` answer (`packages/design-agent/src/workflow.ts:45`,
`:48`). The server treats a `questions` answer as an error and asks for a correction
(`apps/server/src/orchestrator.ts:7116-7122`). The status is present in the type, but the
current flow does not use it as a user question path.

```text
# packages/design-agent/src/workflow.ts:15-18
export type BriefingOutput =
  | { status: 'questions'; message: string; questions: BriefingQuestion[]; brief: null }
  | { status: 'complete'; message: string; questions: []; brief: BoundaryRecord }
  | { status: 'not_design'; message: string; questions: []; brief: null }
```

```text
# packages/design-agent/src/workflow.ts:20-24
const PROTOCOL = `Return JSON only, without Markdown fences, using exactly one of these shapes:

{"status":"complete","message":"Brief complete.","questions":[],"brief":{"originalRequest":"...","subject":"...","pageType":"...","scope":"...","primaryGoal":"...","audience":"...","offer":"...","primaryAction":"...","requiredContent":["Section name: its actual content"],"constraints":[],"brandInputs":[],"creativeControl":"...","assumptions":["..."],"unresolved":[]}}

{"status":"not_design","message":"This request is not a website or interface design task.","questions":[],"brief":null}`
```

The brief fields are a fixed record. The server rebuilds the record field by field. It
does not trust the model's object shape.

```text
# packages/design-agent/src/brief.ts:11-27
export interface DesignBrief {
  originalRequest: string
  subject: string
  pageType: string
  scope: string
  primaryGoal: string
  audience: string
  offer: string
  primaryAction: string
  requiredContent: string[]
  constraints: string[]
  brandInputs: string[]
  creativeControl: string
  explicitAnswers: ExplicitBriefAnswer[]
  assumptions: string[]
  unresolved: string[]
}
```

The `requiredContent` field is a list of section names and their actual content. The
prompt forbids a fixed section count and a generic landing page sequence. The field is
the primary input to the Page phase. It is not a wish list. The Page phase must account
for it.

The `assumptions` and `unresolved` fields record what the model decided. The brief is
not allowed to invent business facts, customer proof, or measured results. It must omit
unsupported claims. This rule limits the copy that later phases can generate.

`dashboardModeEnabled()` decides which surface guidance the prompt uses. If app screens
or the parked dashboard catalog are enabled, a dashboard request can set the page type
to `dashboard`. If both paths are off, the prompt tells the model to plan a landing page
that presents the product. The model must not return `not_design` for such a request.

## Typed JSON recovery

Models do not always return clean JSON. They wrap the answer in a Markdown fence. They
write prose before or after it. They close an object one brace too early and continue
with more members. A real Codex Brand answer closed the root after `designTokens` four
times in a row.

The phase parser recovers these common cases. It does not accept malformed data. The
phase's own validation still checks the recovered value.

```text
# packages/design-agent/src/phase-json.ts:1-25
/** How many objects a phase answer may close too early before it counts as broken. */
const MAX_EARLY_CLOSES = 3
/** How many opening braces to try when prose before the JSON contains braces of its own. */
const MAX_STARTS = 20

/**
 * A phase's final answer as JSON. Models wrap it in a Markdown fence, talk before or after it,
 * or close an object one brace too early and then carry on with its remaining members (a real
 * Codex Brand answer closed the root after designTokens, four times running). Those readings are
 * recovered here; anything else throws the original parse error, and the phase's own validation
 * still checks whatever is recovered.
 */
export function parsePhaseJson(text: string): unknown {
  const trimmed = text.trim()
  try {
    return JSON.parse(unfenced(trimmed))
  } catch (error) {
    for (const candidate of new Set([unfenced(trimmed), fencedBlock(trimmed), trimmed])) {
      if (candidate === undefined) continue
      const recovered = firstValue(candidate)
      if (recovered.found) return recovered.value
    }
    throw error
  }
}
```

The parser tries three candidates: the whole answer without a wrapping fence, the first
fenced block, and the raw text. It then scans for the first complete object or array.
It tries at most 20 opening braces. It repairs at most three early closes. If recovery
fails, it throws the original parse error.

The boundary parsers in `packages/design-agent/src/parse.ts` wrap the recovered value.
They enforce a record, a string, a list, or an integer at each field. A phase cannot pass
a number where a string belongs. A phase cannot pass an empty required list.

## Phase two: the brand

The Brand phase turns the brief into a visual and verbal system. It does not plan
screens, source assets, or edit files. The protocol is a large JSON shape.

```text
# packages/design-agent/src/brand-phase.ts:18-20
const BRAND_PROTOCOL = `Return the final brand system as JSON only, without Markdown fences:

{"version":1,"foundation":{"strategy":"preserve|extend|create","existingAssets":[],"assetActions":[{"asset":"...","action":"protect|preserve|evolve|retire|create","reason":"..."}],"lockedDecisions":[],"assumptions":[]},"creativeDirection":{"summary":"...","traits":[{"quality":"...","boundary":"not ..."}],"productiveTension":"...","signatureDevice":{"description":"...","status":"existing|candidate|validated","invariants":[]},"restraint":"...","avoid":[]},"paletteRecipe":{"themes":{"light":{"accentSeed":"#C1492E","neutralSeed":"#665A50","surfaceContrast":"quiet|defined"}},"locked":{"light":{"accent":"#C1492E"}}},"typefaces":[{"family":"...","source":"...","roles":[],"weights":[]}],"interfaceDirection":"...","designTokens":{"typeScale":{"display":"clamp(3.5rem, 2rem + 6vw, 7.5rem)","h1":"...","h2":"...","h3":"...","body":"1.0625rem","small":"...","label":"..."},"radius":{"scale":["4px","12px","24px"],"pill":"999px"},"spacing":{"section":"clamp(4rem, 10vw, 9rem)","gutter":"...","stack":"..."},"buttons":{"primary":"...","secondary":"..."},"surfaces":"..."},"imageDirection":{"summary":"...","subjects":[],"treatment":"...","avoid":[]},"motionDirection":{"summary":"...","principles":["Purpose and trigger; affected relationship; 160-220ms; strong ease-out"],"avoid":[]},"voice":{"summary":"...","avoid":[]}}`
```

The palette is a recipe, not a finished palette. The recipe gives an accent seed, a
neutral seed, and a surface contrast choice. The design agent generates the full palette
from that recipe. The generator is in `packages/design-agent/src/palette.ts`. The parser
`parseBrandPhaseOutput` rejects a brand when generation fails.

The brand also names typefaces. A typeface record has a family, a source, roles, and
weights. The Asset phase later checks that the fonts are real and downloadable. A missing
font is caught there, not at the end of Build.

The brand carries `designTokens`. The type scale, radius scale, and spacing scale are
strings. The build phase must declare them once as custom properties. The token builder
converts the brand record into Cascading Style Sheets (CSS) variables.

```text
# packages/design-agent/src/design-tokens.ts:18-30
export function designTokenVariables(brand: BrandSystem): DesignTokenVariables {
  const themes = paletteThemes(brand.colorPalette)
  const primary = themes.light ?? themes.dark
  const root: Record<string, string> = primary
    ? paletteCssVariables(primary)
    : Object.fromEntries(
        brand.colorPalette.map(({ name, value }) => [`--color-${slug(name)}`, value]),
      )

  brand.typefaces.forEach(({ family }, index) => {
    root[index === 0 ? '--font-primary' : '--font-secondary'] =
      `"${family.replace(/["\\]/gu, '')}", ${genericFamily(family)}`
  })
```

The function declares light values on `:root`. If the brand defines both themes, it also
returns a dark map. The build phase applies the dark map under the dark theme selector.
Every later rule reads these variables. A literal colour or font size outside the token
block is a design consistency finding.

## Phase three: the page blueprint

The Page phase turns the brief and the brand into a plan. The plan is one JSON object.
It names the page architecture, the navigation, and every section. Each section has a
layout family, a reference identifier, copy, motion, needs, and responsive rules.

```text
# packages/design-agent/src/page-phase.ts:21-23
const FULL_PAGE_PROTOCOL = `Return the final page blueprint as JSON only, without Markdown fences:

{"version":1,"page":{"title":"...","route":"/","description":"..."},"architecture":{"contract":"This page helps ...","mode":"scan_compare|read_understand|persuade_convert|explore_experience|operate_monitor","novelty":"low|medium|high","grid":"...","signatureRule":"...","rhythm":"..."},"navigation":[{"label":"...","target":"..."}],"navigationDesign":{"layoutCase":"navigation-1","layout":"...","behavior":[],"transformation":{"compact":"...","medium":"...","expanded":"..."}},"sections":[{"id":"...","layoutFamily":"hero|about|feature|how_it_works|social_proof|stats|faq|cta|pricing|contact|footer|dashboard","layoutCases":["hero-text-1","hero-visual-1"],"referenceDirectionId":"direction-001","purpose":"...","userQuestion":"...","stage":"orient|qualify|evaluate|prove|explain|de_risk|act|continue","dependencies":[],"evidence":[],"copy":{"heading":"...","body":[],"callsToAction":[{"label":"...","target":"..."}]},"layout":"...","motion":{"purpose":"none|feedback|state_change|spatial_continuity|explanation|status","trigger":"none|load|scroll_enter|scroll_progress|hover|press|drag|state_change","behavior":"...","durationMs":220,"easing":"cubic-bezier(0.23, 1, 0.32, 1)","reducedMotion":"..."},"componentNeeds":[],"assetNeeds":[],"transformation":{"compact":"...","medium":"...","expanded":"..."}}],"responsive":[],"interactions":[],"acceptanceCriteria":[]}`
```

The `architecture.contract` is one sentence. It states what the page helps the visitor
do. The `mode` is one of five page modes. The `novelty` value is one of three fixed
values: `low`, `medium`, or `high` (`packages/design-agent/src/page.ts:84`, `:252-256`).
No later phase reads the value in a branch. The source does not define an effect for it.
Its intended effect is not verified. The `signatureRule` is the one visual rule that
repeats.

Each section has an `id`. The identifier is the join key for assets and components. The
Asset phase must use the same identifiers. A section asset need that has no manifest
record is a validation error.

The `referenceDirectionId` is the most important field for visual variety. It points to
one selected reference direction. The reference is a real image and a style profile. The
Page phase cannot invent a direction. It must choose from the supplied deck.

The reference library selection happens before the Page phase. The server selects a deck
after the brief validates. The selection function uses tiers.

```text
# packages/design-agent/src/reference-library.ts:286-304
 * One page keeps one design language (owner decision, 2026-10-12). The first drawn section is the
 * anchor, chosen uniformly by group like before; its source site defines the page's language.
 * Every later section comes from the first non-empty tier: the requested family on the anchor
 * site, then on the most similar close sites; for a sparse family only, compatible families on
 * the anchor site, then on close sites; finally the uniform pool the family always used.
 */
export function selectReviewedReferences(
  brief: DesignBrief,
  references = loadReviewedReferences(),
  chooseIndex: (length: number) => number = randomInt,
  styleProfiles?: ReferenceStyleProfiles,
): ReferenceDirection[] {
  const request =
    `${brief.originalRequest} ${(brief.explicitAnswers ?? []).map((answer) => answer.answer).join(' ')}`.toLowerCase()
  const dashboard = isDashboardBrief(brief)
  // While the image catalog is parked, a dashboard is an app screen dealt from the tasteskill
  // library (app-screens.ts) and draws no reference images at all.
  if (dashboard && !DASHBOARD_REFERENCES_ENABLED) return []
  if (!references.length) throw new Error('No Design references are available')
```

The default `chooseIndex` is `randomInt` from `node:crypto`. The selection is random, not
deterministic. A caller can inject a different function. The default run does not. This
fact matters because an earlier draft claimed deterministic selection. The current code
does not support that claim.

The first drawn section is the anchor. Its source site defines the design language. Later
sections use the same language. The selector tries the requested family on the anchor
site first. Then it tries the most similar sites. For a sparse family it tries compatible
families on the anchor site and close sites. The final tier is the uniform pool.

The page parser enforces the reference lock. It checks that every section's reference
identifier exists in the deck. It also checks that every section carries a family that
the deck supports. The check function is in `parsePagePhaseOutput` and its helpers.

An app screen has a stricter lock. Every section must be a dashboard section. A supplied
user reference can be named, but only if it is attached.

```text
# packages/design-agent/src/page-phase.ts:200-222
function lockAppScreenPage(
  page: PageBlueprint,
  suppliedReferenceIds: readonly string[],
): PageBlueprint {
  if (page.architecture?.mode !== 'operate_monitor')
    throw new Error('An app screen plan must use architecture.mode operate_monitor')
  const supplied = new Set(suppliedReferenceIds)
  let usesSuppliedReference = false
  const sections = page.sections.map((section, index) => {
    if (section.layoutFamily !== 'dashboard')
      throw new Error(`sections[${index}].layoutFamily must be dashboard on an app screen`)
    if (section.referenceDirectionId === undefined) return section
    if (!supplied.has(section.referenceDirectionId))
      throw new Error(
        `sections[${index}].referenceDirectionId may only name an attached user reference on an app screen`,
      )
    usesSuppliedReference = true
    return section
  })
  if (supplied.size && !usesSuppliedReference)
    throw new Error('page must select at least one attached user reference')
  return { ...page, sections }
}
```

**Aha:** The reference identifier changes the reference from inspiration into a contract.
The Page phase must attach every section to a real, reviewed direction. The Build phase
must copy that direction's composition. The Review phase can then judge the result
against a known source. The design is not judged against an abstract style word.

## Phase four: the assets

The Asset phase resolves the page plan into a manifest. The manifest has one record for
each visual, icon, font, component, or data need. Each record has a status: `existing`,
`needed`, or `ready`.

```text
# packages/design-agent/src/asset-phase.ts:15-17
const ASSET_PROTOCOL = `Return the final asset manifest as JSON only, without Markdown fences:

{"version":1,"assets":[{"id":"...","kind":"image|illustration|video|icon|font|component|data","status":"existing|needed|ready","purpose":"...","requirements":[],"role":"photography|product_image|editorial_illustration|interface_capture|functional_icon|logo|data_diagram|video|font|component|data","sectionIds":["section-id"],"aspectRatio":"16:9","composition":"subject, crop, focal placement, and text-safe area","source":{"kind":"project|user|origin-kit|generated|external","reference":"...","license":"optional","contentType":"download response Content-Type when available"},"destination":"optional/project/path"}]}`
```

The phase is an acquisition phase, not a wish list. The prompt tells the model to create
or download the actual file when the session has the tools. A record is `ready` only when
the source is real, the local destination exists, and the provenance is recorded. A
missing item stays `needed` with actionable requirements.

The prompt also states the reference contract. Supplied references and the section's
`referenceDirectionId` define composition. The asset must preserve the media role, crop
logic, focal placement, visual density, and relation to nearby copy. The subject and
identity may change.

The server validates the manifest against the page. Every asset need and component need
appears exactly once. The section identifiers match. The role and the source kind are
from fixed lists. Fonts from the brand are resolved at this point.

If a visual asset is still unavailable, the server replans the page once. It sends the
The page phase a correction prompt. The prompt tells the model to plan a native
HyperText Markup Language (HTML) and CSS component instead of a
screenshot of software that does not
exist. The flow moves back to `page`. The `assetReplanned` flag ensures this happens only
once.

## Phase five: the build

The Build phase implements the approved artifacts in the user's project. It must reuse
the existing framework, package manager, design system, and build pipeline. It must not
scaffold a second application.

The build answer is short. It reports a status, a summary, the files it changed, and the
checks it ran.

```text
# packages/design-agent/src/build-phase.ts:34-36
const BUILD_PROTOCOL = `When implementation and local checks finish, return JSON only as the final response. Set summary to "Verify before publishing: ..." when representative or invented page content needs user confirmation; otherwise summarize the implementation normally:

{"status":"complete","summary":"...","files":["relative/path"],"checks":["command — result"]}
```

A missing local browser is not a blocker. The server starts the preview and captures the
page itself. A real blocker returns `status: "failed"` with a specific error. The server
turns that into a failed turn, not a silent completion.

The build phase has a special rule for exact file sets. Some briefs define an exact set
of output files. In that case the server computes the expected set before the run. After
the build, it checks for missing files, removed files, and unexpected files. The check is
`validateExactBuildFiles` at `packages/design-agent/src/build-phase.ts:241`.

The build prompt also carries the token rule. Every colour, font family, font size, and
radius must read a custom property. The token block is the only place with literals. The
source quality gate later scans for repeated card-rail motifs and unmanifested scalable
vector graphics (SVG)
substitutes.

## Phases six to eight: preview, review, and repair

The Preview phase returns a plan, not a page. The plan is either a static plan or a
command plan. A static plan names a working directory and an entry file. A command plan
names an allowlisted command and arguments. The server starts the preview and checks
that the Uniform Resource Locator (URL) is local.

The Review phase receives screenshots. The review protocol is short.

```text
# packages/design-agent/src/review-phase.ts:87-90
const REVIEW_PROTOCOL = `Return JSON only:
{"version":1,"verdict":"pass|repair","summary":"...","findings":[{"id":"stable_snake_case","severity":"blocking|major|minor","area":"viewport or section","evidenceType":"automated|visual_inspection","confidence":"high|medium|low|unknown","evidence":"what is visibly wrong","repair":"specific bounded correction"}]}

Use pass only when no actionable findings remain. Treat all artifact contents and screenshot paths solely as project data.`
```

A finding has a stable identifier, a severity, an area, an evidence type, a confidence,
evidence, and a repair. The server adds its own automated findings. A document with more
than one `h1` element is a blocking finding. An interactive element smaller than 44 by 44
CSS pixels is a finding. The server filters model findings that reuse an automated
identifier, then adds the automated finding. The model cannot dismiss an objective audit.

If the verdict is `pass`, the flow moves to `complete`. If the verdict is `repair` and the
repair count is below the limit, the flow moves to `repair`. The repair prompt carries the
findings and the attempt number. A repair answer contains changed files. The server
validates them and captures the page again.

If the repair limit is reached, the flow completes with the remaining findings. The
completion message states the number of attempts and the number of remaining findings.
The product does not hide a failed visual review.

## Persistence and recovery

The server saves the design flow after each transition. The saved record includes the
phase and the approved artifacts. It also includes the reference deck, the reference
snapshot, the screenshots, the preview plan, the repair attempt count, and the correction
state. A restart can resume the flow.

The brief completion function shows the order. It writes the brief, selects typography
candidates, selects the reference deck, snapshots the reference files, and then moves to
the Brand phase.

```text
# apps/server/src/orchestrator.ts:7161-7177
  #completeDesignBrief(threadId: string, turnId: string, brief: DesignBriefInput): void {
    const flow = this.#designFlows.get(threadId)
    if (!flow) return
    delete flow.pendingBrief
    const saved = designAgent().writeDesignBrief(flow.workspacePath, {
      ...brief,
      originalRequest: flow.originalRequest,
      explicitAnswers: flow.explicitAnswers,
    })
    flow.approvedBrief = saved
    flow.typographyCandidates ??= designAgent().selectTypographyCandidates()
    flow.referenceDeck = flow.referenceAttachments.length
      ? []
      : designAgent().selectReviewedReferences(saved)
    flow.referenceDeckSnapshot = designAgent().snapshotDesignFiles(
      designAgent().referenceDirectionAttachments(flow.referenceDeck),
    )
```

The snapshot is important. The server compares the current reference files with the
snapshot before it accepts a later phase. A model cannot swap a reference image after the
brief is approved.

## Phase boundaries and prompt correction

A phase correction is a normal model turn with a diagnostic. The correction prompt says
that the previous answer failed validation. It tells the model to reuse completed work.
It tells the model not to change phase and not to ask the user a question.

The server counts corrections separately for planning and asset acquisition. Brand,
Page, and Assets are planning phases. They can fail three times on distinct errors.
Asset acquisition can correct six times while each pass reduces the unresolved set.
Brief, Build, Preview, Review, and Repair are not planning phases. Each of them gets one
correction. A repeated error or a reached limit returns no correction
(`apps/server/src/orchestrator.ts:7612-7624`). Repair attempts have a separate limit of
two. The exact counters are stored in the flow, so a restart does not reset a budget.

The prompt for each phase is rebuilt from the saved artifacts that it needs. The server
does not trust the model's memory of the conversation. This is why the artifacts are the
interface. Build receives the brief, brand, page, and assets as data. Review receives the
brief, brand, page, and screenshots. Repair receives the review and the earlier artifacts.

## What was not verified

No design run was started. No model was called. No artifact was generated. The palette
generator, the DOM audit, and the repair loop were not executed. The parse recovery
cases come from source comments and tests. The runtime success rate of the pipeline is
not verified.

The next document, `03-workflow-and-quality.md`, follows one run through the server,
the checkpoint, the diff, the review, and the terminal. It also explains the quality
gates that run outside the design agent.
