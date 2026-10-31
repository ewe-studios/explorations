---
title: The Proof and the Receipt
---

# The proof: a repeatable check that writes a receipt

The proof is the one coding exercise of the workshop. Attendees write `lab/exercises/prove.ts`, a function that records observed facts. `pnpm snack prove` runs it in a fresh browser, writes a receipt bound to the exact code revision, and returns an exit code.

## Words on this page

| Word | Meaning |
|------|---------|
| Check | One recorded fact: a name, a pass flag, an expected value, and an actual value |
| Proof kit | The object passed to `prove()`: the page, the email, a `check` function, and a `storage` reader |
| Receipt | The JSON file that records the verdict, revision, checks, page errors, and screenshot |
| Fresh visitor | A new browser context with no storage and no cookies |
| Verdict | `PASS`, `FAIL`, or `ERROR` (the last maps to exit code 2, "can't tell") |

## The proof kit

`prove()` receives a small object (`lab/proof.ts:7-15`):

```ts
// lab/proof.ts:7-15
export type ProofKit = {
  /** A fresh visitor (no storage) with the app already loaded. */
  page: Page;
  email: string;
  /** Record one fact. Never throws: a failed check is evidence, not a crash. */
  check(name: string, actual: unknown, expected: unknown, hint?: string): void;
  /** localStorage for the app's origin, read through the DevTools protocol. */
  storage(): Promise<Record<string, unknown>>;
};
```

The `check` function is the whole interface. It records a fact as a `Check` and uses a deep equality to decide pass or fail (`lab/proof.ts:66-67`). It never throws, because a failed check is evidence, not a crash.

The exercise scaffold uses a trick to force the work to be done (`lab/exercises/prove.ts`). The three values are destructured but uninitialized, so TypeScript refuses to compile the exercise until the attendee assigns them:

```ts
// lab/exercises/prove.ts (scaffold)
void [savedEmail, promised, storage];
```

The reference proof fills in the one missing move — a reload — and four checks (`lab/solution/prove.ts`). The four facts are: the status survives a reload, the email survives a reload, the confirmation survives a reload, and the record is in localStorage.

**Aha: the scaffold cannot compile as a pass.** `void [savedEmail, promised, storage]` uses `void` over a destructuring of uninitialized bindings. TypeScript reports "used before assigned" until each is filled, so the exercise cannot accidentally typecheck into a false pass.

## How a proof runs

`runProof` is shared by `snack prove` and the exercise check (`lab/proof.ts:52`). It works in five steps:

1. Open a fresh browser context — no storage, no cookies (`lab/proof.ts:56`).
2. Collect `pageerror` and console errors into a set (`lab/proof.ts:60-61`).
3. Load the app, record the revision from the meta tag (`lab/proof.ts:62-63`).
4. Run `prove()`, which calls `check(...)` for each fact.
5. Compare the revision before and after, set the verdict, and take the screenshot.

```ts
// lab/proof.ts:69-77
const after = await readRevision(page);
const now = await currentRevision(app);
if (after !== receipt.revision || now !== receipt.revision) {
  receipt.error = `The app changed while proving (loaded ${receipt.revision}, now ${now}). Evidence from two builds proves neither.`;
} else if (!checks.length) {
  receipt.error = "prove() recorded no checks. Call check(name, actual, expected) for each fact you observe.";
} else {
  receipt.verdict = checks.every(check => check.pass) ? "PASS" : "FAIL";
}
```

The revision is checked twice: the meta tag in the page, and the `/__snack/revision` endpoint on the server (`lab/proof.ts:69-71`). If the source changed while the proof ran, the receipt is ERROR, not PASS. A receipt can never silently cover a moving target.

## The receipt

The receipt type is the artifact the whole workshop is about (`lab/proof.ts:17-27`):

```ts
// lab/proof.ts:17-27
export type Receipt = {
  verdict: "PASS" | "FAIL" | "ERROR";
  revision: string | null;
  app: string;
  at: string;
  checks: Check[];
  pageErrors: string[];
  screenshot?: string;
  error?: string;
  next: string[];
};
```

`pnpm snack prove` writes three files (`lab/snack.ts:247-253`):

| File | Purpose |
|------|---------|
| `.lab-runs/snack/receipt.json` | The latest receipt, which `doctor` reads |
| `.lab-runs/snack/receipts/<at>-<revision>.json` | The append-only history, one file per run |
| `.lab-runs/snack/proof-<revision>-<time>.png` | The screenshot bound to this run |

The append is deliberate: "Append, never overwrite: failures stay in the history next to the final green" (`lab/snack.ts:250`).

## The screenshot is evidence, not decoration

If the screenshot cannot be saved, the verdict is forced to ERROR (`lab/proof.ts:78-86`):

```ts
// lab/proof.ts:83-86
// The receipt tells the agent to read the screenshot, so a missing one can't pass.
receipt.verdict = "ERROR";
receipt.error = `Couldn't save the screenshot (${...}). A receipt without its screenshot can't be read.`;
```

**Aha: a receipt without its screenshot cannot pass.** The screenshot is part of the evidence bundle. The receipt's `next` field tells the agent to read the screenshot for anything the checks missed, so its absence is "can't tell", never PASS.

## Exit codes and the `next` field

The verdict maps to an exit code (`lab/snack.ts:253`):

| Verdict | Exit code | Meaning |
|---------|-----------|---------|
| `PASS` | 0 | The checks passed on this revision |
| `FAIL` | 1 | A check failed; the `next` list names each fix |
| `ERROR` | 2 | Can't tell; never a pass, never a product defect |

The `next` list is built from the failed checks (`lab/proof.ts:95-103`). For a FAIL, each failed check becomes a line like `Fix "status": expected X, got Y.` followed by `Then re-run: pnpm snack prove`. For a PASS, it warns that the checks cover nothing else and tells the agent to read the screenshot.

## The three real receipts

The repository ships three real receipts as teaching material (`public/materials/snack-receipts/`):

| File | Verdict | Revision | What it shows |
|------|---------|----------|----------------|
| `1-fail-attendee-app.json` | FAIL | `d14af25cd65a` | All four reload checks fail on the broken app |
| `2-pass-after-agent-fix.json` | PASS | `4e8c43d6bc1e` | The agent's fix passes 5 of 5 checks |
| `3-pass-console-error-miss.json` | PASS | `8d0a01bfdc95` | PASS despite `pageErrors: ["track is not defined"]` |

The third receipt is the point of the game: a proof can pass while still missing a real defect. See [Break it quietly](05-break-it-quietly.md).

## Aha moments

**Aha: the proof covers only what it checks.** The PASS receipt's `next` field says so directly: "These checks passed on this revision. They cover nothing else" (`lab/proof.ts:96`). The proof is honest about its own boundary, which is what lets the game teach attendees to find the boundary.

**Aha: a crash is "can't tell", a failed check is "broken".** `check()` never throws; it records a failing fact. A thrown `prove()` is a tool failure and lands in ERROR (`lab/proof.ts:88-90`). The two signals have different next actions, so the agent never confuses "the app is wrong" with "the harness broke".

## Where to go next

Read [Semantic judgment](04-semantic-judgment-jev.md) for the part the checks cannot reach. Read [Break it quietly](05-break-it-quietly.md) for what a PASS receipt still misses.
