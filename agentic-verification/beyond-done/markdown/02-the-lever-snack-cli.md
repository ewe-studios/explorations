---
title: The Lever — pnpm snack
---

# The lever: `pnpm snack`, a remote control built for agents

`pnpm snack` is the whole lever. It is one small command-line interface (CLI) that launches, drives, inspects, and proves the waitlist app over the Chrome DevTools Protocol (CDP). It prints JavaScript Object Notation (JSON), and every error says what to run next (`lab/snack.ts:1-3`).

## Words on this page

| Word | Meaning |
|------|---------|
| Chrome DevTools Protocol (CDP) | The protocol that lets a program attach to a running Chrome and control it |
| Session | A saved record of the running app and browser, written by `up` and read by the other commands |
| Event log | A JSON-lines file where `up` records console, page errors, loads, and responses |
| Reveal | The name of the hidden bug, printed when `up` stops |
| Revision | The SHA-256 hash of the served HTML, first 12 hex characters |

## The command table

The commands live in one object. Each entry is a group, a usage string, and a summary (`lab/snack.ts:24-39`).

```ts
// lab/snack.ts:24-39
const COMMANDS = {
  up:       ["Health", "up [--bug NAME|random] [--solution] [--headless]", "Start the app and a Chrome you can drive. Leave it running in its own terminal."],
  down:     ["Health", "down", "Stop what up started."],
  doctor:   ["Health", "doctor", "Is this instance worth driving? Tools, session, loaded build, last proof."],
  join:     ["Drive", "join <email>", "Type an email into the form and submit it, like a user."],
  reload:   ["Drive", "reload", "Reload the page, like a user."],
  reset:    ["Drive", "reset [--dry-run]", "Clear the app's storage and reload: a fresh visitor."],
  snapshot: ["Look", "snapshot", "What a user sees: status, email, confirmation, accessibility tree."],
  storage:  ["Look", "storage", "The app's localStorage, read through the DevTools protocol."],
  console:  ["Look", "console [--errors]", "Console messages and uncaught errors since up."],
  network:  ["Look", "network", "Responses and failed requests since up."],
  screenshot:["Look", "screenshot [path]", "Save a full-page screenshot."],
  eval:     ["Look", "eval <expression>", "Read page state after the user path ran. Not for calling app internals."],
  prove:    ["Prove", "prove [--solution]", "Fresh visitor, your proof, a receipt. Exit 0 PASS, 1 FAIL, 2 can't tell."],
  judge:    ["Prove", "judge", "Ask Jev whether the confirmation copy is honest. Needs TYPESAFE_API_KEY."],
} as const;
```

The four groups are Health, Drive, Look, and Prove. Health manages the running instance. Drive acts like a user. Look reads state. Prove makes evidence.

## The session: who owns the running app

`up` writes a session file with the process identifier (pid), the app address, and the CDP address (`lab/snack.ts:47`). Every other command reads that session and attaches to the running Chrome.

The first non-obvious decision is the pid-ownership check. A saved pid alone could be reused by a different process, so `readSession` asks the app server to confirm its own pid before trusting the session (`lab/snack.ts:55-66`):

```ts
// lab/snack.ts:55-57
// A live pid alone could be a reused one: the session is ours only if its app server says it runs in that pid.
const owner = !alive(session.pid) ? "gone" : await fetch(new URL("/__snack/pid", session.appUrl), { signal: AbortSignal.timeout(1000) })
  .then(async response => ((await response.json()) as { pid?: number }).pid === session.pid ? "ours" : "gone")
  .catch((error: Error) => error.name === "TimeoutError" ? "silent" : "gone");
```

The `/__snack/pid` endpoint answers with the server's own pid (`lab/server.ts:71-74`). Three outcomes follow:

| Outcome | Meaning | Action |
|---------|---------|--------|
| `ours` | The app is the one `up` started | Trust it |
| `gone` | The app is dead or not ours | Delete the session, tell the user to run `pnpm snack up` |
| `silent` | The app did not answer (for example, suspended with Control-Z) | Keep the record, signal nothing |

**Aha: silent is not proof.** A suspended app is not "gone", so the tool does not delete the session and does not claim the app is dead. The comment states it: "Silent is not proof of anything: keep the record and signal nothing" (`lab/snack.ts:59`). This is the class's own theme — a missing answer is "can't tell", not "broken" — applied to the tool itself.

## Attach, do one thing, detach

Every read command uses the same wrapper (`lab/snack.ts:70-85`). `withPage` reads the session, connects to Chrome over CDP, finds or opens the app page, runs one callback, and closes the browser connection. Closing over CDP only disconnects; `up` keeps the browser alive (`lab/snack.ts:83`).

```ts
// lab/snack.ts:70-84
async function withPage<T>(fn: (page: Page, browser: Browser, session: Session) => Promise<T>): Promise<T> {
  const session = await readSession();
  const browser = await chromium.connectOverCDP(session.cdpUrl).catch(() => {
    throw new SnackError(`Can't reach Chrome at ${session.cdpUrl}.`, "Stop up (Ctrl+C or pnpm snack down), then start it again.");
  });
  try {
    const context = browser.contexts()[0];
    let page = context.pages().find(p => p.url().startsWith(session.appUrl)) ?? context.pages()[0];
    if (!page) page = await context.newPage();
    if (!page.url().startsWith(session.appUrl)) await page.goto(session.appUrl);
    page.setDefaultTimeout(3000);
    return await fn(page, browser, session);
  } finally {
    await browser.close(); // Over CDP this disconnects; up keeps the browser alive.
  }
}
```

## `up`: start the app and record everything

`up` is the only long-running command. It chooses the app source (`solution` for a game or `--solution`, otherwise `exercise`), claims a lock, starts the local app server, launches Chromium with a remote-debugging port, and records events (`lab/snack.ts:140-202`).

The event log is the evidence stream. Four page events are recorded as JSON lines (`lab/snack.ts:164-172`):

```ts
// lab/snack.ts:166-171
const watch = (page: Page) => {
  page.on("console", message => log({ type: "console", level: message.type(), text: message.text() }));
  page.on("pageerror", error => log({ type: "pageerror", level: "error", text: error.message }));
  page.on("load", () => log({ type: "load", url: page.url() }));
  page.on("response", response => log({ type: "response", method: response.request().method(), status: response.status(), url: response.url() }));
};
```

`console` and `network` later read this log back (`lab/snack.ts:339-347`). The `pageerror` line is how the `console-error` bug is caught in the game.

When `up` stops, it prints the reveal — the name of the hidden bug — only for game sessions (`lab/snack.ts:189`):

```ts
// lab/snack.ts:188-189
if (game) out({ reveal: bug ?? "none" });
```

The reveal is bound to the exact session that was running, so a player cannot peek by reading a file. See [Break it quietly](05-break-it-quietly.md).

## `up.lock`: two ups cannot share state

`claimUp` holds a lock file before touching the profile and event log, so two `up` sessions cannot share them (`lab/snack.ts:117-129`). It writes the pid with the exclusive flag, and if the lock is left by a dead process, it removes it. If another `up` is genuinely running, it refuses.

## `doctor`: is this instance worth driving?

`doctor` runs five checks and reports each with a `next` action when it fails (`lab/snack.ts:211-238`):

| Check | Passes when |
|-------|-------------|
| `node` | Node major version is 24 or higher |
| `chromium` | The Playwright Chromium is installed |
| `session` | A live, owned app session exists |
| `build` | The page's revision equals the source revision on disk |
| `proof` | The last receipt is for the current source (else STALE) |

The `build` check is where the revision hash becomes visible to the user. If the page shows one revision and the source is another, the detail reads `STALE: the page shows X, the source is now Y`, with `next: pnpm snack reload` (`lab/snack.ts:227-229`). The `proof` check reports a stale receipt the same way (`lab/snack.ts:231-233`).

## `down`, `join`, `reload`, `reset`

- `down` sends the signal, then polls up to 5 seconds for the pid to die (`lab/snack.ts:204-209`).
- `join` fills the email field and clicks "Join waitlist", waiting for the page to react rather than a fixed time (`lab/snack.ts:101-110`).
- `reload` reloads the page and returns the snapshot.
- `reset` clears both localStorage and sessionStorage over CDP, then reloads. `--dry-run` prints what would be cleared without clearing (`lab/snack.ts:327-336`).

## The `next` field

`SnackError` carries a `next` string and a default exit code of 2 (`lab/snack.ts:42-44`):

```ts
// lab/snack.ts:42-44
class SnackError extends Error {
  constructor(message: string, readonly next: string, readonly code = 2) { super(message); }
}
```

Every error path sets `next` to the concrete command that fixes the situation. The top-level handler prints the error and the `next` as JSON, and exits with the known code (`lab/snack.ts:361-366`). This is the vocabulary that lets an agent act on the tool's output without a long prompt.

## Aha moments

**Aha: the pid check separates "ours" from "silent" from "gone".** Most tools check whether a process is alive. `snack` also checks whether the alive process is really the one it started, and distinguishes "not answering" as its own state. All three map to different next actions.

**Aha: no sleeps.** `joinWaitlist` waits for the status text to change, with a timeout, instead of sleeping (`lab/snack.ts:109`). The comment says: "Wait for the page to react, not for a fixed time. No change is also an answer." This keeps the proof deterministic across machine speeds.

**Aha: the event log is the eyes, and hygiene matters.** `console` and `network` read a log `up` recorded. The app server answers `/favicon.ico` with 204 so a missing favicon does not pollute the console and page-error evidence (`lab/server.ts:78-79`). The proof records page errors, so clean evidence is a feature, not a detail.

## Where to go next

Read [Proof and receipt](03-the-proof-and-receipt.md) for how `prove` turns these commands into a receipt. Read [Break it quietly](05-break-it-quietly.md) for the bugs and the reveal.
