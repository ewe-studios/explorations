---
name: WebVTT Parser
description: Full WebVTT subtitle parser implementation with timing, cues, and styling support
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/webvtt-parser/
---

# WebVTT Parser - Subtitle Parsing Library

## Overview

The WebVTT Parser is a **zero-copy WebVTT subtitle parsing library** that implements the full [WebVTT specification](https://w3c.github.io/webvtt/). It provides efficient parsing of subtitle files with support for:

- Cue timing and positioning
- Text formatting (bold, italic, underline)
- Ruby annotations (for Japanese/Chinese text)
- Voice spans and semantic classes
- Line and position styling
- Region definitions

The parser is designed for **zero-copy operation**, borrowing from the input string to avoid allocations during parsing.

## Directory Structure

```
webvtt-parser/
├── src/
│   ├── lib.rs              # Main module and parser
│   ├── cue.rs              # Cue structure and settings
│   ├── time.rs             # Time parsing and representation
│   ├── region.rs           # Region definitions
│   ├── styling.rs          # Text styling and classes
│   └── error.rs            # Parse error types
├── Cargo.toml
├── README.md
└── examples/
    └── parse_vtt.rs        # Example parser usage
```

## WebVTT Format

### Basic Structure

```webvtt
WEBVTT

00:00:00.000 --> 00:00:04.000
Hello, world!

00:00:04.500 --> 00:00:08.000 align:middle line:90%
This is a centered subtitle
with multiple lines

00:00:08.500 --> 00:00:12.000 position:10%,line-left align:left size:35%
This subtitle is positioned
on the left side of the screen
```

### Cue Structure

```rust
pub struct VttCue<'a> {
    /// Unique identifier for this cue (optional)
    pub name: Option<&'a str>,

    /// Start timestamp in milliseconds
    pub start: Time,

    /// End timestamp in milliseconds
    pub end: Time,

    /// Cue settings for positioning and alignment
    pub cue_settings: Option<VttCueSettings>,

    /// The subtitle text (may contain HTML-like tags)
    pub text: &'a str,

    /// Comment/note (optional)
    pub note: Option<&'a str>,
}

pub struct VttCueSettings {
    /// Writing direction: horizontal, vertical-lr, vertical-rl
    pub vertical: Option<Vertical>,

    /// Line position (number or percentage)
    pub line: Option<NumberOrPercentage>,

    /// Line alignment: start, center, end, auto
    pub line_align: Option<LineAlign>,

    /// Horizontal position (percentage)
    pub position: Option<u8>,

    /// Position alignment: line-left, center, line-right, auto
    pub position_align: Option<PositionAlign>,

    /// Size of the text area (percentage)
    pub size: Option<u8>,

    /// Text alignment: left, center, right, start, end
    pub align: Option<Align>,
}
```

## Parser Implementation

### Main Parser

```rust
// From webvtt-parser/src/lib.rs
pub struct Vtt<'a> {
    /// List of cues in display order
    pub cues: Vec<VttCue<'a>>,

    /// Defined regions (optional)
    pub regions: Vec<VttRegion<'a>>,
}

impl<'a> Vtt<'a> {
    /// Parse a WebVTT string (zero-copy)
    pub fn parse(input: &'a str) -> Result<Self, ParseError> {
        let mut parser = Parser::new(input);

        // Validate header
        parser.expect("WEBVTT")?;

        // Skip any header comments/metadata
        parser.skip_whitespace_and_comments();

        let mut cues = Vec::new();
        let mut regions = Vec::new();

        while !parser.is_empty() {
            if parser.starts_with("REGION") {
                let region = parser.parse_region()?;
                regions.push(region);
            } else {
                let cue = parser.parse_cue()?;
                cues.push(cue);
            }
        }

        Ok(Vtt { cues, regions })
    }

    /// Get cue active at a specific time
    pub fn cue_at(&self, time_ms: u64) -> Option<&VttCue<'a>> {
        self.cues.iter().find(|cue| {
            time_ms >= cue.start.0 && time_ms < cue.end.0
        })
    }

    /// Get all cues active at a time (for overlapping subtitles)
    pub fn cues_at(&self, time_ms: u64) -> Vec<&VttCue<'a>> {
        self.cues.iter()
            .filter(|cue| time_ms >= cue.start.0 && time_ms < cue.end.0)
            .collect()
    }
}
```

### Time Parsing

```rust
// From webvtt-parser/src/time.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time(pub u64); // Milliseconds

impl Time {
    /// Parse WebVTT timestamp: HH:MM:SS.mmm
    pub fn parse(s: &str) -> Result<Self, TimeParseError> {
        // Format: HH:MM:SS.mmm or MM:SS.mmm
        let parts: Vec<&str> = s.split(':').collect();

        let (hours, minutes, seconds_and_ms): (u64, u64, &str) = match parts.as_slice() {
            [minutes, seconds_and_ms] => (0, minutes.parse()?, seconds_and_ms),
            [hours, minutes, seconds_and_ms] => {
                (hours.parse()?, minutes.parse()?, seconds_and_ms)
            }
            _ => return Err(TimeParseError::InvalidFormat),
        };

        // Parse seconds and milliseconds
        let sec_parts: Vec<&str> = seconds_and_ms.split('.').collect();
        let (seconds, milliseconds): (u64, u64) = match sec_parts.as_slice() {
            [seconds, milliseconds] => {
                let seconds = seconds.parse()?;
                // Handle variable precision milliseconds
                let ms_str = if milliseconds.len() > 3 {
                    &milliseconds[..3]
                } else {
                    milliseconds
                };
                let ms = ms_str.parse::<u64>().unwrap_or(0);
                // Scale to milliseconds if fewer digits
                let ms = ms * 10u64.pow(3 - ms_str.len() as u32);
                (seconds, ms)
            }
            _ => return Err(TimeParseError::InvalidSecondsFormat),
        };

        let total_ms = hours * 3600_000
            + minutes * 60_000
            + seconds * 1000
            + milliseconds;

        Ok(Time(total_ms))
    }

    /// Format as WebVTT timestamp
    pub fn format(&self) -> String {
        let ms = self.0;
        let hours = ms / 3600_000;
        let minutes = (ms % 3600_000) / 60_000;
        let seconds = (ms % 60_000) / 1000;
        let millis = ms % 1000;

        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    }

    /// Get hours component
    pub fn hours(&self) -> u64 {
        self.0 / 3600_000
    }

    /// Get minutes component (within hour)
    pub fn minutes(&self) -> u64 {
        (self.0 % 3600_000) / 60_000
    }

    /// Get seconds component (within minute)
    pub fn seconds(&self) -> u64 {
        (self.0 % 60_000) / 1000
    }

    /// Get milliseconds component
    pub fn millis(&self) -> u64 {
        self.0 % 1000
    }
}
```

### Cue Parser

```rust
// From webvtt-parser/src/lib.rs
struct Parser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn parse_cue(&mut self) -> Result<VttCue<'a>, ParseError> {
        self.skip_whitespace();

        // Optional cue name (identifier line)
        let name = if !self.starts_with_timestamp() {
            let name = self.read_line();
            self.skip_whitespace();
            Some(name)
        } else {
            None
        };

        // Parse timestamp line
        let timing_line = self.read_line().trim();
        let (start, end, settings) = self.parse_timing_line(timing_line)?;

        // Read cue text (until blank line or EOF)
        let text = self.read_cue_text();

        Ok(VttCue {
            name,
            start,
            end,
            cue_settings: Some(settings),
            text,
            note: None,
        })
    }

    fn parse_timing_line(
        &mut self,
        line: &str,
    ) -> Result<(Time, Time, VttCueSettings), ParseError> {
        // Format: start --> end [settings...]
        let parts: Vec<&str> = line.split("-->").collect();
        if parts.len() < 2 {
            return Err(ParseError::InvalidTimestamp);
        }

        let start = Time::parse(parts[0].trim())?;
        let rest = parts[1];

        // Split end time and settings
        let mut rest_parts = rest.splitn(2, char::is_whitespace);
        let end = Time::parse(rest_parts.next().unwrap().trim())?;

        // Parse settings
        let mut settings = VttCueSettings::default();
        if let Some(settings_str) = rest_parts.next() {
            settings = self.parse_cue_settings(settings_str)?;
        }

        Ok((start, end, settings))
    }

    fn parse_cue_settings(&mut self, s: &str) -> Result<VttCueSettings, ParseError> {
        let mut settings = VttCueSettings::default();

        for setting in s.split_whitespace() {
            if let Some((key, value)) = setting.split_once(':') {
                match key {
                    "vertical" => {
                        settings.vertical = Some(match value {
                            "rl" => Vertical::RightToLeft,
                            "lr" => Vertical::LeftToRight,
                            _ => return Err(ParseError::InvalidVertical),
                        });
                    }
                    "line" => {
                        settings.line = Some(self.parse_number_or_percentage(value)?);
                    }
                    "lineAlign" => {
                        settings.line_align = Some(match value {
                            "start" => LineAlign::Start,
                            "center" => LineAlign::Center,
                            "end" => LineAlign::End,
                            _ => return Err(ParseError::InvalidLineAlign),
                        });
                    }
                    "position" => {
                        settings.position = Some(
                            value.trim_end_matches('%').parse()?
                        );
                    }
                    "positionAlign" => {
                        settings.position_align = Some(match value {
                            "line-left" => PositionAlign::LineLeft,
                            "center" => PositionAlign::Center,
                            "line-right" => PositionAlign::LineRight,
                            "auto" => PositionAlign::Auto,
                            _ => return Err(ParseError::InvalidPositionAlign),
                        });
                    }
                    "size" => {
                        settings.size = Some(
                            value.trim_end_matches('%').parse()?
                        );
                    }
                    "align" => {
                        settings.align = Some(match value {
                            "left" => Align::Left,
                            "center" => Align::Center,
                            "right" => Align::Right,
                            "start" => Align::Start,
                            "end" => Align::End,
                            _ => return Err(ParseError::InvalidAlign),
                        });
                    }
                    _ => {
                        // Unknown setting, skip
                    }
                }
            }
        }

        Ok(settings)
    }

    fn read_cue_text(&mut self) -> &'a str {
        let start = self.position;

        // Read until blank line or EOF
        while !self.is_empty() {
            if self.starts_with("\n\n") {
                break;
            }
            self.advance();
        }

        let text = &self.input[start..self.position];
        self.skip_whitespace();
        text.trim()
    }
}
```

## Text Formatting Support

### WebVTT Tags

```rust
// Supported WebVTT tags
pub enum VttTag<'a> {
    /// Bold: <b>text</b>
    Bold(&'a str),

    /// Italic: <i>text</i>
    Italic(&'a str),

    /// Underline: <u>text</u>
    Underline(&'a str),

    /// Ruby annotation: <ruby>base<rt>annotation</ruby>
    Ruby { base: &'a str, rt: &'a str },

    /// Voice span: <v.name>text</v>
    Voice { name: &'a str, text: &'a str },

    /// Class span: <c.classname>text</c>
    Class { name: &'a str, text: &'a str },

    /// Timestamp within cue: <00:00:02.500>
    Timestamp(Time),

    /// Line break: <br>
    LineBreak,
}

impl<'a> Vtt<'a> {
    /// Parse cue text into structured tags
    pub fn parse_cue_text(text: &'a str) -> Vec<VttTag<'a>> {
        let mut tags = Vec::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            if let Some(pos) = remaining.find('<') {
                // Push text before tag
                if pos > 0 {
                    tags.push(VttTag::Text(&remaining[..pos]));
                }

                // Parse tag
                let tag_start = pos + 1;
                if let Some(tag_end) = remaining[tag_start..].find('>') {
                    let tag_content = &remaining[tag_start..tag_start + tag_end];
                    let tag = Self::parse_single_tag(tag_content);
                    tags.push(tag);
                    remaining = &remaining[tag_start + tag_end + 1..];
                } else {
                    // Malformed tag, treat as text
                    tags.push(VttTag::Text(remaining));
                    break;
                }
            } else {
                // No more tags
                tags.push(VttTag::Text(remaining));
                break;
            }
        }

        tags
    }

    fn parse_single_tag(content: &str) -> VttTag {
        if content == "br" {
            return VttTag::LineBreak;
        }

        if content == "b" || content == "/b" {
            return VttTag::BoldStart;
        }

        if content == "i" || content == "/i" {
            return VttTag::ItalicStart;
        }

        if content == "u" || content == "/u" {
            return VttTag::UnderlineStart;
        }

        // Voice span: v.name or /v
        if let Some(name) = content.strip_prefix('v') {
            if let Some(name) = name.strip_prefix('.') {
                return VttTag::VoiceStart(name);
            }
            return VttTag::VoiceEnd;
        }

        // Class span: c.classname or /c
        if let Some(name) = content.strip_prefix('c') {
            if let Some(name) = name.strip_prefix('.') {
                return VttTag::ClassStart(name);
            }
            return VttTag::ClassEnd;
        }

        // Ruby annotation
        if content == "ruby" || content == "/ruby" || content == "rt" {
            return VttTag::Ruby;
        }

        // Timestamp
        if let Ok(time) = Time::parse(content) {
            return VttTag::Timestamp(time);
        }

        VttTag::Unknown
    }
}
```

## Integration with FFrames

```rust
// Usage in fframes crate
impl Frame {
    /// Get subtitle phrase for current frame
    pub fn get_subtitle_phrase<'a>(
        &self,
        subtitles: &'a Vtt<'a>,
    ) -> Option<&'a str> {
        let frame_time_ms = (self.current_second * 1000.0) as u64;

        subtitles
            .cues
            .iter()
            .find(|cue| {
                frame_time_ms >= cue.start.0 && frame_time_ms < cue.end.0
            })
            .map(|cue| cue.text)
    }

    /// Get full cue with metadata
    pub fn get_subtitle_cue<'a>(
        &self,
        subtitles: &'a Vtt<'a>,
    ) -> Option<&'a VttCue<'a>> {
        let frame_time_ms = (self.current_second * 1000.0) as u64;

        subtitles.cues.iter().find(|cue| {
            frame_time_ms >= cue.start.0 && frame_time_ms < cue.end.0
        })
    }

    /// Get stack of active cues (for overlapping subtitles)
    pub fn get_cue_stack<'a>(
        &self,
        subtitles: &'a Vtt<'a>,
        overlap_ms: u64,
    ) -> Vec<&'a VttCue<'a>> {
        let frame_time_ms = (self.current_second * 1000.0) as u64;

        // Get cues that overlap with current time
        subtitles.cues.iter().filter(|cue| {
            // Cue starts before frame ends
            cue.start.0 < frame_time_ms + overlap_ms
            // Cue ends after frame starts
            && cue.end.0 > frame_time_ms - overlap_ms
        }).collect()
    }

    /// Render subtitle with positioning
    pub fn render_subtitle<'a>(
        &self,
        cue: &'a VttCue<'a>,
        ctx: &FFramesContext,
    ) -> Svgr {
        let settings = cue.cue_settings.as_ref();

        // Calculate position based on settings
        let x = settings
            .and_then(|s| s.position)
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "50%".to_string());

        let y = self.calculate_subtitle_y(settings);

        let align = settings
            .and_then(|s| s.align)
            .unwrap_or(Align::Center)
            .to_css();

        svgr!(
            <text
                x={x}
                y={y.to_string()}
                font-family="Roboto"
                font-size="36"
                fill="white"
                text-anchor={align}
            >
                {cue.text}
            </text>
        )
    }

    fn calculate_subtitle_y(&self, settings: Option<&VttCueSettings>) -> f32 {
        // Default: bottom of screen with margin
        let default_y = 1080.0 - 100.0;

        settings
            .and_then(|s| s.line)
            .map(|line| {
                match line {
                    NumberOrPercentage::Percentage(p) => {
                        1080.0 * (p as f32 / 100.0)
                    }
                    NumberOrPercentage::Number(n) => {
                        // Line number to pixels (approximate)
                        n as f32 * 40.0
                    }
                }
            })
            .unwrap_or(default_y)
    }
}
```

## Example Usage

```rust
use webvtt_parser::{Vtt, Time};

fn main() {
    let vtt_content = r#"
WEBVTT

00:00:00.000 --> 00:00:04.000 align:middle
Hello, world!

00:00:04.500 --> 00:00:08.000 position:10%,line-left align:left
This is a left-aligned subtitle

00:00:08.500 --> 00:00:12.000
This subtitle has <b>bold</b> and <i>italic</i> text.
"#;

    let vtt = Vtt::parse(vtt_content).unwrap();

    // Get cue at specific time
    if let Some(cue) = vtt.cue_at(2000) {
        println!("Cue at 2s: {}", cue.text);
    }

    // Iterate over all cues
    for cue in &vtt.cues {
        println!(
            "{} - {}: {}",
            Time(cue.start.0).format(),
            Time(cue.end.0).format(),
            cue.text
        );
    }

    // Parse cue text formatting
    for cue in &vtt.cues {
        let tags = Vtt::parse_cue_text(cue.text);
        for tag in tags {
            println!("Tag: {:?}", tag);
        }
    }
}
```

## Performance Considerations

### Zero-Copy Design

The parser uses borrowed lifetimes throughout to avoid allocations:

```rust
// All string data borrows from input
pub struct VttCue<'a> {
    pub text: &'a str,       // Borrows from input
    pub name: Option<&'a str>, // Borrows from input
    // ...
}
```

This means:
- No string allocations during parsing
- Input string must outlive the parsed `Vtt` struct
- Multiple `Vtt` can share the same input string

### LRU Cache for Parsed Subtitles

```rust
// In fframes integration
pub struct SubtitleCache {
    cache: LRUCache<String, Vtt<'static>>,
}

impl SubtitleCache {
    pub fn get_or_parse(&mut self, path: &str, content: &str) -> &Vtt {
        if !self.cache.contains(path) {
            // Parse and store (requires careful lifetime management)
            let vtt = Vtt::parse(content);
            // In practice, need to box or use arena for dynamic lifetimes
        }
        self.cache.get(path).unwrap()
    }
}
```

## Related Documents

- [FFrames Core](./fframes-core-exploration.md) - Frame integration
- [FFrames Editor](./fframes-editor-exploration.md) - Editor subtitle track

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/webvtt-parser/`
- WebVTT Spec: https://w3c.github.io/webvtt/
- FFFrames Main Exploration: `../../fframes/exploration.md`
