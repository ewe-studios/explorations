# earshot: sound as evidence for coding agents

An exploration of earshot 0.4.1 at local pin `4ac3d2a`. Six documents explain how a command line tool on a central
processing unit (CPU) turns sound into evidence that a coding agent can read and use.

## The problem: an agent cannot hear

A coding agent can write the code for a film, a product demonstration or a game. The agent cannot hear the result. It
cannot tell a good key press from a poor one. It cannot notice that a hit lands 80 milliseconds after the cut. It
cannot hear that the music swells in the wrong place.

The usual feedback is a person with ears. That person listens, forms an opinion and writes a note. The loop is slow. The
note is not repeatable. The agent cannot test the note.

earshot gives the agent a different feedback path. It measures the sound. It compares the sound to a description. It
places the sound against the picture. Each result is a number, a time, a label or an image. The agent can read the
result, change the code and measure again.

## The thesis: sound becomes evidence

The thesis of this exploration is simple. Earshot does not give an agent taste. It gives the agent evidence. The
evidence has four kinds:

| Evidence | Question it answers | Example |
|---|---|---|
| Calibrated match | Is this the kind of sound I asked for? | A key press scores z +5.4 for "a single mechanical keyboard key press" |
| Timing | Where is the sound, and does it land on the picture? | A hit starts at 0:04.12 and matches the cut within 4 milliseconds |
| Objective measurement | What are the physical facts? | Integrated loudness -14.2 loudness units relative to full scale (LUFS), true peak -1.1 decibels true peak (dBTP) |
| Learned classification | Which broad class is it? | AudioSet label "Typing" at probability 0.71, or a realness probability |

AudioSet is a labelled sound taxonomy. Its tagger returns 527 class probabilities. Typicality is how close a sound sits to its nearest real references.

A model can be wrong. A measurement can also be wrong, but its error is visible. Earshot keeps the two apart. A "strong" match comes with a z score and a precision measured on held-out recordings at a stated base rate. A held-out recording is one that was not used to tune the model or the vocabulary. A base rate is the share of candidate pairs in a search that are true matches. A search over a large library has a low base rate. A loudness number comes from a standard formula. The
documents show where each number comes from and where it stops being trustworthy.

**Aha:** the important design choice is not the use of a model. It is the calibration of the model against the
vocabulary itself. A raw cosine between a sound and a sentence is hard to read. Earshot removes the bias of each term, divides by the spread of the terms on that sound, and reports a z score. The z score turns "0.42" into "strong, z +5.4". For one sound, suppose four corrected term scores are 0.40, 0.20, 0.10 and 0.00. Their mean is 0.175 and their population spread is 0.148. The z scores are +1.5, +0.2, -0.5 and -1.2. A real sound has 322 terms, so the mean and the spread use all 322. This example is arithmetic, not a recorded result. The same idea appears in realness, in forge typicality and in timeline event tags: a raw similarity is standardised against a reference spread before it is read.

```text
# earshot/vocab.py:271-290
    def calibrate(self, A, Q=None, method='both'):
        """A (n, 512) audio, Q (m, 512) query embeddings -> dict of (n, T) term arrays and (n, m) query arrays:
        cos, u (bias removed when method is 'center' / 'both'), z (per-sound z when 'sound' / 'both', else u),
        pct (share of vocab terms the query beats)"""
        A = np.atleast_2d(A)
        C = A @ self.E.T
        U = C - self.mu_E if method in ('center', 'both') else C
        r = {'cos_t': C, 'u_t': U}
        if method in ('sound', 'both'):
            mu, sd = U.mean(1, keepdims=True), U.std(1, keepdims=True) + 1e-9
        else:
            mu, sd = 0.0, 1.0
        r['z_t'] = (U - mu) / sd
        if Q is not None:
            Q = np.atleast_2d(Q)
            Cq = A @ Q.T
            Uq = Cq - self.bias(Q) if method in ('center', 'both') else Cq
            r['cos_q'], r['u_q'], r['z_q'] = Cq, Uq, (Uq - mu) / sd
            r['pct_q'] = (U[:, None, :] < Uq[:, :, None]).mean(-1)
        return r
```

The verdict is a threshold on that z score. The thresholds are fixed and few: 3.0, 2.5 and 2.0. A reader can therefore compare two verdicts across files and commands.

```text
# earshot/match.py:50-54
def verdict(z):
    for t, name in VERDICTS:
        if z >= t:
            return name
    return 'no match'
```

## The command map

The command line interface (CLI) has 20 commands. They group by the job the agent needs.

### Hear one file

| Command | Evidence |
|---|---|
| `describe FILE` | Calibrated tags with z scores, AudioSet labels, objective measurements |
| `match FILE QUERY` | A verdict for one description, with alternative readings |
| `rank QUERY FILES...` | Candidate files sorted by one description |
| `find FILE QUERY` | Sample-accurate trims of the best moments |
| `compare FILE --refs DIR` | Distance to a set of real reference recordings, and typicality |
| `ab A B` | The measured differences between two sounds or two trims |
| `realness FILES...` | Real or code-synthesized, with per-window evidence |

### Hear a film or a long file

| Command | Evidence |
|---|---|
| `timeline FILE --video FILM` | Events, sections, loudness, beat grid, sync to cuts and cue frames |
| `sheet FILE --video FILM` | One portable network graphics (PNG) image with frames, labels, spectrogram and sync marks |
| `critique FILE --video FILM` | A launch-film scorecard with timestamped fixes |
| `profile FILMS...` | A reference profile built from films the user chooses |
| `measure FILE` | Objective numbers only, with no model |

### Make or ship a sound

| Command | Evidence |
|---|---|
| `forge RECIPE` | A sound made from code, searched and judged by the critic |
| `fit FILE` | A forge recipe module fitted from a recording |
| `sfx list / render / check` | One sound effect (SFX) from the shipped bank of 252, aimed at the picture |
| `kit init / render / check / audition` | A film's sound roles, rendered to takes with measured sync points |
| `gallery FOLDER` | One static HyperText Markup Language (HTML) page with players and measurements |

The bank is a set of rendered sounds. Each entry is a render block, not an audio file.

### Serve an agent

| Command | Evidence |
|---|---|
| `mcp` | The same tools over the Model Context Protocol (MCP) |
| `llm` | An optional second opinion from a hosted audio large language model (LLM) |
| `doctor` | The install, model cache and CPU settings |

The command table is not the contract. Each command is a thin layer over a Python function. The Python application
programming interface (API) exposes the functions. The MCP server exposes 16 of them as tools.

```text
# earshot/cli.py:16-37
COMMANDS = {
    'describe': 'describe',      # what does this sound like? tags + measurements
    'match': 'match',            # how well does a sound match a text description (calibrated)
    'rank': 'search',            # rank many files by a description
    'find': 'search',            # find the best moments inside a long recording (sample trims)
    'compare': 'compare',        # similarity to reference recordings
    'ab': 'compare',             # A/B two sounds
    'realness': 'realness',      # real recording vs code-synthesized sound
    'timeline': 'timeline',      # listen through a whole soundtrack or film: events, tags, sync, loudness
    'sheet': 'sheet',            # annotated picture of a soundtrack (spectrogram + events + frames) for agents with eyes
    'critique': 'critique',      # launch-video sound scorecard against a profile of professional references
    'profile': 'profile',        # build a reference profile from your own reference films
    'forge': 'forge',            # make sounds in code and improve them by listening
    'fit': 'forge.fit',          # translate a recording into code (analysis by synthesis) and vary it
    'sfx': 'forge.sfx',          # one sound of the shipped bank by id, aimed at the picture; audit any sound effect
    'kit': 'forge.kit',          # a film's sound kit: one bank sound per role, rendered to takes with sync points
    'gallery': 'gallery',        # one static HTML page of forge results: players, spectrograms, numbers
    'measure': 'features',       # objective measurements only (no models)
    'llm': 'llm',                # optional audio-LLM critique (only with your own API key + consent)
    'mcp': 'mcp_server',         # run earshot as an MCP server for coding agents
    'doctor': 'doctor',          # check the install, models and CPU settings
}
```

The dispatch is lazy. `earshot describe` imports only the describe module. This keeps a first call shorter when the agent needs one answer, because the unused command modules never load. The model load still dominates a cold call, as the limits document explains.

## Two making loops

Earshot is not only a listener. It has two loops that make sound from code.

```mermaid
flowchart LR
    subgraph Forge loop
        R[Recipe module] --> P[Render parameters]
        P --> W[Waveform]
        W --> C[Critic: model and model-free]
        C --> S[Search: evolution strategy or separable covariance matrix adaptation evolution strategy (CMA-ES)]
        S --> P
        W --> B[Robust re-renders and seed bank]
        B --> F[Final picks and report]
    end
    subgraph Fit loop
        A[Recording] --> M[Analysis by synthesis]
        M --> K[Model JSON]
        K --> G[Recipe code generator]
        G --> T[Takes, knobs and morphs]
        T --> Q[Quality metrics against the recording]
        Q --> M
    end
```

The forge loop starts with a recipe and a target description. It renders candidates, scores them with a critic and
searches the parameter space. The critic combines a contrastive language-audio pretraining (CLAP) match, a realness
probability, a typicality score and several model-free checks. The output is a set of diverse final sounds, a report
and an audition file.

The fit loop starts with a recording. The fitter chooses a model kind, such as modal, sines, spectral, gesture or hybrid. It fits a synthesis model by analysis by synthesis. It writes the model as a
readable forge recipe module. The module stores structure, not samples. The user can render takes, turn knobs and
morph between two recordings.

The two loops meet at the recipe contract. A recipe is a Python module with a `render(params, rng)` function. A take is one rendered audio file. A knob is a recipe parameter that the user can turn. A morph blends two fitted models. A fitted recipe has the same `render(params, rng)` shape as a hand-written
recipe. The forge critic can search a fitted recipe without a special case.

## Quality numbers

Every number below comes from a recorded command in the project. This exploration did not run any command.

| Claim | Number | Recorded command |
|---|---|---|
| Vocabulary calibration | Pooled area under the receiver operating characteristic curve (ROC-AUC) 0.966 on the tuning library; raw cosine 0.951 | `eval/semantics/run_eval.py analyze` |
| Held-out match, strong band | 98 percent right kind of sound at a 3 percent base rate; 61 percent for the exact variant, where a nearby variant counts as wrong | `eval/semantics/run_eval.py heldout` |
| Realness | ROC-AUC 0.922 with a whole real category or generator family held out; 6.7 percent of real clips called synthesized | `eval/realness/run_eval.py --stage report` |
| Event detection | 93.2 percent recall against 147 placed effects | `eval/timeline/run_eval.py` |
| Sync to picture | 84 percent within 5 milliseconds; 96 percent within one frame, where a frame is 41.7 milliseconds at 24 frames per second | `eval/timeline/run_eval.py` |
| Launch scorecard | 120 of 140 reference-range checks in range, 86 percent, leave-one-out. A check asks whether one metric lies inside the spread of the other six films. | `eval/launch/run_eval.py` |
| Tests at release | 128 passed, 14 skipped in the fast run; the 14 slow tests also pass | `python -m pytest -q` and `--slow` |

The tuning library is the set of 219 recordings used to fit the calibration. The held-out library is a separate set of 125 recordings that nothing was tuned on. These numbers are records, not live results. The evaluation data is licensed and absent from the repository. The
scripts read it from environment variables or a local ignored path file.

## The six documents

| File | What it teaches |
|---|---|
| [README.md](README.md) | The problem, the thesis, the map, the numbers and the non-claims |
| [00-overview.md](00-overview.md) | The listening problem, four evidence kinds, one full pass, vocabulary, loops and interfaces |
| [01-architecture.md](01-architecture.md) | The module graph, the core, every operation, forge, fit, bank, interfaces and data |
| [02-core-mechanism.md](02-core-mechanism.md) | Bytes to a calibrated verdict, a find trim, realness, timeline, forge and fit, traced end to end |
| [03-workflow-and-quality.md](03-workflow-and-quality.md) | One describe request and one forge run, with the model load, tests, doctor and release checks |
| [04-limits-and-comparison.md](04-limits-and-comparison.md) | What is trustworthy, what is a hint, what is impossible, and how earshot compares |

## Reading order

1. Read this page for the problem and the map.
2. Read [00-overview.md](00-overview.md) for the complete system in one pass.
3. Read [01-architecture.md](01-architecture.md) for the file layout and the ownership boundaries.
4. Read [02-core-mechanism.md](02-core-mechanism.md) for the formulas and the code paths.
5. Read [03-workflow-and-quality.md](03-workflow-and-quality.md) for the commands, the model load and the test record.
6. Read [04-limits-and-comparison.md](04-limits-and-comparison.md) before trusting a verdict.

A reader who wants one answer should start at the overview and then read the limits. A reader who wants to change the
code should read the architecture and the core mechanism.

## Pin and terms

The source is a local Git checkout at `4ac3d2a`. A pin is the exact Git commit that this exploration read. It has 287 tracked files. The package version is 0.4.1. The code and
the documents use the Massachusetts Institute of Technology (MIT) licence. The models have their own terms. CLAP uses
`laion/larger_clap_general` under the Apache License 2.0. The audio spectrogram transformer (AST) uses
`MIT/ast-finetuned-audioset-10-10-0.4593` under the BSD 3-Clause licence. The evaluation recordings and films are not
redistributed.

The version has one naming detail. The package is 0.4.1. The launch film is a `v0.4.0` release asset. A release tag is not a package version. A reader who sees the release asset can mistake its tag for the package version. The package version and the tag are separate.

## What this exploration does not claim

- It does not run earshot, a test, a model or an evaluation.
- It does not load, analyse or listen to an audio file.
- It does not make a network request.
- It does not verify runtime behaviour. Every runtime number comes from a project document or a results file.
- It does not recompute the evaluation data. The data is absent.
- It does not resolve two project inconsistencies. The held-out library is 125 files in `docs/VALIDATION.md` and 126 in `docs/DESIGN.md` and `eval/README.md`. The validation record says 13 MCP tools, but the code registers 16. This exploration quotes the code count and marks the record as stale.
- It does not verify the multi-agent build process that `docs/DESIGN.md` describes.
- It does not independently judge the quality of the sounds. It records the project's measurements and their limits.

```text
# earshot/audio.py:124-165
def load(path, sr=SR, mono=False, start=None, end=None, cache=True):
    """decode `path` to float32 (n, 2) or (n,) when mono; optional [start, end) in seconds"""
    if not os.path.exists(path):
        raise AudioError(f'no such file: {path}')
    ch = 1 if mono else 2
    x = None
    npy = None
    if cache:
        config.ensure_dirs()
        npy = os.path.join(config.DEC_CACHE, _key(path, sr, ch) + '.npy')
        if os.path.exists(npy):
            try:
                x = np.load(npy)
            except Exception:
                x = None
    if x is None:
        x = _decode(path, sr, ch)
        if npy:
            tmp = f'{npy}.{os.getpid()}.tmp.npy'
            np.save(tmp, x)
            replace_quietly(tmp, npy)
    bad = ~np.isfinite(x)
    if bad.any():   # one NaN / Inf sample breaks every analysis downstream (#12)
        bad = bad.any(axis=-1) if x.ndim == 2 else bad
        raise AudioError(f'{path}: {int(bad.sum())} NaN / Inf sample(s), first at {int(np.argmax(bad)) / sr:.3f} s: '
                         'the file is corrupt or the code that made it divided by zero')
    if start is not None or end is not None:
        a = max(0, int(round((start or 0.0) * sr)))
        b = len(x) if end is None else min(len(x), int(round(end * sr)))
        x = x[a:b]
    return x


def to_mono(x):
    return x.mean(axis=1) if x.ndim == 2 else x


def resample(x, sr_from, sr_to):
    if sr_from == sr_to:
        return x
    import soxr
    return soxr.resample(x, sr_from, sr_to, quality='HQ').astype(np.float32)
```

The loader is the first step of every listening path. It decodes a file to 48 kilohertz, keeps two channels, guards
against not-a-number values and caches the result. The core mechanism document gives the cache key and the gap in it.
