# color-eyre Deep Dive

## Overview

color-eyre is the most feature-rich error handler for eyre, providing:
- Colorful backtrace rendering
- SpanTrace integration from tracing-error
- Custom sections (notes, warnings, suggestions)
- Panic hook integration
- GitHub issue URL generation
- Configurable themes and frame filters

## Architecture

### Handler Structure

```rust
pub struct Handler {
    filters: Arc<[Box<FilterCallback>]>,
    backtrace: Option<Backtrace>,
    suppress_backtrace: bool,
    #[cfg(feature = "capture-spantrace")]
    span_trace: Option<SpanTrace>,
    sections: Vec<HelpInfo>,
    display_env_section: bool,
    #[cfg(feature = "track-caller")]
    display_location_section: bool,
    #[cfg(feature = "issue-url")]
    issue_url: Option<String>,
    #[cfg(feature = "issue-url")]
    issue_metadata: Arc<Vec<(String, Box<dyn Display + Send + Sync>)>>,
    #[cfg(feature = "issue-url")]
    issue_filter: Arc<IssueFilterCallback>,
    theme: Theme,
    #[cfg(feature = "track-caller")]
    location: Option<&'static Location<'static>>,
}
```

### HookBuilder Configuration

```rust
pub struct HookBuilder {
    filters: Vec<Box<FilterCallback>>,
    capture_span_trace_by_default: bool,
    display_env_section: bool,
    display_location_section: bool,
    panic_section: Option<Box<dyn Display + Send + Sync + 'static>>,
    panic_message: Option<Box<dyn PanicMessage>>,
    theme: Theme,
    issue_url: Option<String>,
    issue_metadata: Vec<(String, Box<dyn Display + Send + Sync + 'static>)>,
    issue_filter: Arc<IssueFilterCallback>,
}
```

## Theme System

### Built-in Themes

```rust
impl Theme {
    pub fn dark() -> Self {
        Self {
            file: style().purple(),
            line_number: style().purple(),
            active_line: style().white().bold(),
            error: style().bright_red(),
            help_info_note: style().bright_cyan(),
            help_info_warning: style().bright_yellow(),
            help_info_suggestion: style().bright_cyan(),
            help_info_error: style().bright_red(),
            dependency_code: style().green(),
            crate_code: style().bright_red(),
            code_hash: style().bright_black(),
            panic_header: style().red(),
            panic_message: style().cyan(),
            spantrace_target: style().bright_red(),
            spantrace_fields: style().bright_cyan(),
            hidden_frames: style().bright_cyan(),
        }
    }

    pub fn light() -> Self { /* ... */ }
}
```

### Custom Theme Creation

```rust
use color_eyre::config::Theme;
use owo_colors::style;

let theme = Theme::new()
    .error(style().red().bold())
    .file(style().blue())
    .help_info_note(style().green())
    .help_info_warning(style().yellow())
    .help_info_suggestion(style().cyan());

color_eyre::config::HookBuilder::default()
    .theme(theme)
    .install()?;
```

## Frame Filtering System

### Default Frame Filters

color-eyre applies several filters by default:

```rust
fn default_frame_filter(frames: &mut Vec<&Frame>) {
    // Find post-panic frames (skip these)
    let top_cutoff = frames
        .iter()
        .rposition(|x| x.is_post_panic_code())
        .map(|x| x + 2)
        .unwrap_or(0);

    // Find runtime init frames (skip these)
    let bottom_cutoff = frames
        .iter()
        .position(|x| x.is_runtime_init_code())
        .unwrap_or(frames.len());

    let rng = top_cutoff..=bottom_cutoff;
    frames.retain(|x| rng.contains(&x.n))
}

fn eyre_frame_filters(frames: &mut Vec<&Frame>) {
    let filters = &[
        "<color_eyre::Handler as eyre::EyreHandler>::default",
        "eyre::",
        "color_eyre::",
    ];

    frames.retain(|frame| {
        !filters.iter().any(|f| {
            frame.name.as_ref()
                .map(|n| n.starts_with(f))
                .unwrap_or(true)
        })
    });
}
```

### Frame Classification

```rust
impl Frame {
    fn is_dependency_code(&self) -> bool {
        const SYM_PREFIXES: &[&str] = &[
            "std::", "core::", "backtrace::backtrace::",
            "_rust_begin_unwind", "__rust_", "___rust_",
            "main", "_start", "__libc_start_main",
        ];
        // Check symbol prefixes and file paths
    }

    fn is_post_panic_code(&self) -> bool {
        const SYM_PREFIXES: &[&str] = &[
            "_rust_begin_unwind",
            "core::result::unwrap_failed",
            "std::panicking::begin_panic",
            // ...
        ];
        // Check if frame is after panic started
    }

    fn is_runtime_init_code(&self) -> bool {
        const SYM_PREFIXES: &[&str] = &[
            "std::rt::lang_start::",
            "test::run_test::run_test_inner::",
            // ...
        ];
        // Check if frame is runtime initialization
    }
}
```

### Custom Frame Filter

```rust
use color_eyre::config::HookBuilder;

HookBuilder::default()
    .add_frame_filter(Box::new(|frames| {
        // Remove all frames from specific modules
        frames.retain(|frame| {
            !frame.name.as_ref().map(|n| {
                n.starts_with("my_crate::internal::")
            }).unwrap_or(true)
        });
    }))
    .install()?;
```

## Section System

### HelpInfo Types

```rust
// In color-eyre/src/section/help.rs
pub enum HelpInfo {
    Custom(Box<dyn Display + Send + Sync + 'static>),
    Error(Box<dyn StdError + Send + Sync + 'static>),
    Note(Box<dyn Display + Send + Sync + 'static>),
    Warning(Box<dyn Display + Send + Sync + 'static>),
    Suggestion(Box<dyn Display + Send + Sync + 'static>),
}
```

### Section Implementation

```rust
impl Section for Result<T, Report> {
    type Return = Self;

    fn section<D>(self, section: D) -> Self::Return
    where
        D: Display + Send + Sync + 'static,
    {
        self.map_err(|mut e| {
            if let Some(handler) = e.handler_mut().downcast_mut::<Handler>() {
                handler.sections.push(HelpInfo::Custom(Box::new(section)));
            }
            e
        })
    }

    fn note<D>(self, note: D) -> Self::Return
    where
        D: Display + Send + Sync + 'static,
    {
        self.map_err(|mut e| {
            if let Some(handler) = e.handler_mut().downcast_mut::<Handler>() {
                handler.sections.push(HelpInfo::Note(Box::new(note)));
            }
            e
        })
    }
    // ... other methods
}
```

### SectionExt for Headered Content

```rust
pub trait SectionExt: Sized {
    fn header<C>(self, header: C) -> IndentedSection<C, Self>
    where
        C: Display + Send + Sync + 'static;
}

impl<T> SectionExt for T
where
    T: Display + Send + Sync + 'static,
{
    fn header<C>(self, header: C) -> IndentedSection<C, Self> {
        IndentedSection { body: self, header }
    }
}

// IndentedSection display
impl<H, B> fmt::Display for IndentedSection<H, B>
where
    H: Display + Send + Sync + 'static,
    B: Display + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut headered = f.header(&self.header);
        let mut indented = indenter::indented(&mut headered.ready())
            .with_format(indenter::Format::Uniform { indentation: "   " });
        write!(&mut indented, "{}", self.body)?;
        Ok(())
    }
}
```

## Backtrace Formatting

### Frame Structure

```rust
#[derive(Debug)]
pub struct Frame {
    pub n: usize,
    pub name: Option<String>,
    pub lineno: Option<u32>,
    pub filename: Option<PathBuf>,
}
```

### Styled Frame Display

```rust
impl fmt::Display for StyledFrame<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (frame, theme) = (self.0, self.1);
        let is_dependency_code = frame.is_dependency_code();

        // Print frame index
        write!(f, "{:>2}: ", frame.n)?;

        // Parse function name (strip hash suffix)
        let name = frame.name.as_deref().unwrap_or("<unknown>");
        let (name, hash) = parse_hash_suffix(name);

        // Color based on dependency status
        if is_dependency_code {
            write!(f, "{}", name.style(theme.dependency_code))?;
        } else {
            write!(f, "{}", name.style(theme.crate_code))?;
        }
        write!(f, "{}", hash.style(theme.code_hash))?;

        // Print source location
        write!(
            f,
            "\n    at {}:{}",
            file.style(theme.file),
            lineno.style(theme.line_number),
        )?;

        // Print source code if full verbosity
        if verbosity >= Verbosity::Full {
            write!(f, "{}", SourceSection(frame, theme))?;
        }

        Ok(())
    }
}
```

### Backtrace Display with Hidden Frames

```rust
impl fmt::Display for BacktraceFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:━^80}", " BACKTRACE ")?;

        // Collect frames from backtrace
        let frames: Vec<_> = self.inner.frames()
            .iter()
            .flat_map(|frame| frame.symbols())
            .zip(1usize..)
            .map(|(sym, n)| Frame {
                name: sym.name().map(|x| x.to_string()),
                lineno: sym.lineno(),
                filename: sym.filename().map(|x| x.into()),
                n,
            })
            .collect();

        // Apply filters
        let mut filtered_frames = frames.iter().collect();
        for filter in self.filters {
            filter(&mut filtered_frames);
        }

        // Display with hidden frame indicators
        let mut last_n = 0;
        for frame in &filtered_frames {
            let hidden = frame.n - last_n - 1;
            if hidden != 0 {
                print_hidden!(hidden);  // "⋮ N frames hidden ⋮"
            }
            write!(f, "{}", StyledFrame(frame, self.theme))?;
            last_n = frame.n;
        }

        Ok(())
    }
}
```

## SpanTrace Integration

### SpanTrace Capture

```rust
impl EyreHook {
    pub(crate) fn default(&self, error: &(dyn std::error::Error + 'static)) -> Handler {
        // ... backtrace capture ...

        #[cfg(feature = "capture-spantrace")]
        let span_trace = if self.spantrace_capture_enabled()
            && crate::handler::get_deepest_spantrace(error).is_none()
        {
            Some(tracing_error::SpanTrace::capture())
        } else {
            None
        };

        Handler {
            // ...
            span_trace,
            // ...
        }
    }

    fn spantrace_capture_enabled(&self) -> bool {
        std::env::var("RUST_SPANTRACE")
            .map(|val| val != "0")
            .unwrap_or(self.capture_span_trace_by_default)
    }
}
```

### Getting Deepest SpanTrace

```rust
#[cfg(feature = "capture-spantrace")]
pub(crate) fn get_deepest_spantrace<'a>(
    error: &'a (dyn std::error::Error + 'static),
) -> Option<&'a SpanTrace> {
    eyre::Chain::new(error)
        .rev()  // Start from root cause
        .flat_map(|error| error.span_trace())
        .next()  // Get the deepest (most recent) span trace
}
```

### Formatted SpanTrace Display

```rust
struct FormattedSpanTrace<'a>(&'a SpanTrace);

impl fmt::Display for FormattedSpanTrace<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n{:━^80}\n", " SPANTRACE ")?;

        let mut n = 0;
        self.0.with_spans(|metadata, fields| {
            if n > 0 {
                writeln!(f)?;
            }
            write!(
                f,
                "{:>2}: {} with {}",
                n,
                metadata.target().style(theme.spantrace_target),
                fields.style(theme.spantrace_fields),
            )?;
            write!(
                f,
                "\n    at {}:{}",
                metadata.file().unwrap_or("<unknown>").style(theme.file),
                metadata.line().unwrap_or(0).style(theme.line_number),
            )?;
            n += 1;
            true  // Continue iteration
        });

        Ok(())
    }
}
```

## Panic Hook Integration

### PanicReport Structure

```rust
pub struct PanicReport<'a> {
    hook: &'a PanicHook,
    panic_info: &'a PanicInfo<'a>,
    backtrace: Option<backtrace::Backtrace>,
    span_trace: Option<tracing_error::SpanTrace>,
}
```

### Panic Hook Installation

```rust
impl PanicHook {
    pub fn install(self) {
        std::panic::set_hook(self.into_panic_hook());
    }

    pub fn into_panic_hook(self) -> Box<dyn Fn(&PanicInfo<'_>) + Send + Sync> {
        Box::new(move |panic_info| {
            eprintln!("{}", self.panic_report(panic_info));
        })
    }

    pub fn panic_report<'a>(&'a self, panic_info: &'a PanicInfo<'_>) -> PanicReport<'a> {
        let v = panic_verbosity();
        let capture_bt = v != Verbosity::Minimal;

        #[cfg(feature = "capture-spantrace")]
        let span_trace = if self.spantrace_capture_enabled() {
            Some(tracing_error::SpanTrace::capture())
        } else {
            None
        };

        let backtrace = if capture_bt {
            Some(backtrace::Backtrace::new())
        } else {
            None
        };

        PanicReport {
            panic_info,
            span_trace,
            backtrace,
            hook: self,
        }
    }
}
```

### Default Panic Message

```rust
struct DefaultPanicMessage(Theme);

impl PanicMessage for DefaultPanicMessage {
    fn display(&self, pi: &PanicInfo<'_>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}",
            "The application panicked (crashed).".style(self.0.panic_header)
        )?;

        let payload = pi.payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| pi.payload().downcast_ref::<&str>().cloned())
            .unwrap_or("<non string panic payload>");

        write!(f, "Message:  ")?;
        writeln!(f, "{}", payload.style(self.0.panic_message))?;

        write!(f, "Location: ")?;
        if let Some(loc) = pi.location() {
            write!(f, "{}:{}", loc.file().style(self.0.panic_file),
                          loc.line().style(self.0.panic_line_number))?;
        } else {
            write!(f, "<unknown>")?;
        }

        Ok(())
    }
}
```

## GitHub Issue Integration

### Issue Section Generation

```rust
#[cfg(feature = "issue-url")]
pub(crate) struct IssueSection<'a> {
    url: &'a str,
    payload: &'a str,
    backtrace: Option<&'a backtrace::Backtrace>,
    span_trace: Option<&'a SpanTrace>,
    location: Option<&'a Location<'a>>,
    metadata: &'a [(String, Box<dyn Display>)],
}

impl fmt::Display for IssueSection<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        writeln!(f, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        writeln!(f, "REPORT AN ISSUE")?;
        writeln!(f, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        writeln!(f)?;
        writeln!(f, "To report this error, visit:")?;
        writeln!(f, "  {}", generate_issue_url(self))?;
        Ok(())
    }
}

fn generate_issue_url(section: &IssueSection<'_>) -> String {
    let mut url = Url::parse(section.url).unwrap();
    url.query_pairs_mut()
        .append_pair("title", &format!("Panic: {}", truncate(section.payload, 80)))
        .append_pair("body", &generate_issue_body(section));
    url.to_string()
}
```

### Issue Filter

```rust
pub type IssueFilterCallback = dyn Fn(ErrorKind<'_>) -> bool + Send + Sync + 'static;

pub enum ErrorKind<'a> {
    NonRecoverable(&'a dyn Any),      // Panic payload
    Recoverable(&'a dyn std::error::Error),
}

// Example filter
issue_filter(|kind| match kind {
    ErrorKind::NonRecoverable(payload) => {
        let msg = payload.downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| payload.downcast_ref::<&str>().cloned())
            .unwrap_or("");
        !msg.contains("expected panic message")
    }
    ErrorKind::Recoverable(error) => {
        !error.is::<std::fmt::Error>()  // Don't create issues for fmt errors
    }
})
```

## Verbosity Levels

```rust
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) enum Verbosity {
    Minimal,    // Just error messages
    Medium,     // + Backtrace
    Full,       // + Source code
}

pub(crate) fn panic_verbosity() -> Verbosity {
    match env::var("RUST_BACKTRACE") {
        Ok(s) if s == "full" => Verbosity::Full,
        Ok(s) if s != "0" => Verbosity::Medium,
        _ => Verbosity::Minimal,
    }
}

pub(crate) fn lib_verbosity() -> Verbosity {
    match env::var("RUST_LIB_BACKTRACE")
        .or_else(|_| env::var("RUST_BACKTRACE"))
    {
        Ok(s) if s == "full" => Verbosity::Full,
        Ok(s) if s != "0" => Verbosity::Medium,
        _ => Verbosity::Minimal,
    }
}
```

## Example Output Formats

### Minimal (default)

```
Error:
   0: Unable to read config
   1: No such file or directory (os error 2)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ SPANTRACE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

 0: myapp::read_file with path="config.json"
    at src/main.rs:42
```

### Medium (RUST_LIB_BACKTRACE=1)

```
Error:
   0: Unable to read config
   1: No such file or directory (os error 2)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ SPANTRACE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

 0: myapp::read_file with path="config.json"
    at src/main.rs:42

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ BACKTRACE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 ⋮ 5 frames hidden ⋮
 6: myapp::read_file::h1234567890abcdef
    at /path/to/src/main.rs:45
 7: myapp::main::h0987654321fedcba
    at /path/to/src/main.rs:10
 ⋮ 10 frames hidden ⋮
```

### Full (RUST_LIB_BACKTRACE=full)

```
Error:
   0: Unable to read config
   1: No such file or directory (os error 2)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ SPANTRACE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

 0: myapp::read_file with path="config.json"
    at src/main.rs:42
      40 │ }
      41 │
    > 42 │ #[instrument]
      43 │ fn read_file(path: &str) -> Result<()> {
      44 │     info!("Reading file");

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ BACKTRACE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 ⋮ 5 frames hidden ⋮
 6: myapp::read_file::h1234567890abcdef
    at /path/to/src/main.rs:45
      43 │ fn read_file(path: &str) -> Result<()> {
      44 │     info!("Reading file");
    > 45 │     Ok(fs::read_to_string(path).map(drop)?)
      46 │ }
      47 │
 ⋮ 10 frames hidden ⋮
```
