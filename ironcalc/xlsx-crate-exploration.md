# ironcalc (xlsx) Crate Deep Dive

## Overview

The `ironcalc` crate (located in the `xlsx/` directory) provides XLSX file format import and export capabilities for the IronCalc spreadsheet engine.

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.ironcalc/IronCalc/xlsx/`

## Cargo.toml

```toml
[package]
name = "ironcalc"
version = "0.2.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
zip = "0.6"                      # XLSX is a ZIP archive
roxmltree = "0.19"               # XML parsing
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
ironcalc_base = { path = "../base", version = "0.2" }
itertools = "0.12"
chrono = "0.4"
bitcode = "0.6.0"

[dev-dependencies]
uuid = { version = "1.2.2", features = ["serde", "v4"] }

[lib]
name = "ironcalc"
path = "src/lib.rs"

[[bin]]
name = "test"
path = "src/bin/test.rs"

[[bin]]
name = "documentation"
path = "src/bin/documentation.rs"
```

## Module Structure

```
xlsx/src/
├── lib.rs                     # Public API, re-exports ironcalc_base
├── error.rs                   # XlsxError type
├── compare.rs                 # Workbook comparison
├── import/                    # XLSX import
│   ├── mod.rs                 # Main import logic
│   ├── colors.rs              # Color parsing
│   ├── metadata.rs            # Document metadata
│   ├── shared_strings.rs      # Shared string table
│   ├── styles.rs              # Style parsing
│   ├── tables.rs              # Table parsing
│   ├── util.rs                # XML utilities
│   ├── workbook.rs            # Workbook XML parsing
│   └── worksheets.rs          # Worksheet XML parsing
├── export/                    # XLSX export
│   ├── mod.rs                 # Main export logic
│   ├── _rels.rs               # Package relationships
│   ├── doc_props.rs           # Document properties
│   ├── escape.rs              # XML escaping
│   ├── shared_strings.rs      # Shared string table
│   ├── styles.rs              # Style generation
│   ├── workbook.rs            # Workbook XML generation
│   ├── workbook_xml_rels.rs   # Workbook relationships
│   ├── worksheets.rs          # Worksheet XML generation
│   └── xml_constants.rs       # XML constants
└── bin/
    ├── test.rs                # Test binary
    └── documentation.rs       # Documentation generator
```

## Public API

### Import Functions

```rust
// import/mod.rs

/// Load a Model from an xlsx file on disk
pub fn load_from_xlsx(file_name: &str, locale: &str, tz: &str) -> Result<Model, XlsxError>

/// Load a Workbook from bytes (useful for network transfer)
pub fn load_from_xlsx_bytes(
    bytes: &[u8],
    name: &str,
    locale: &str,
    tz: &str,
) -> Result<Workbook, XlsxError>

/// Load from .icalc format (IronCalc binary format)
pub fn load_from_icalc(file_name: &str) -> Result<Model, XlsxError>
```

### Export Functions

```rust
// export/mod.rs

/// Save Model to xlsx file on disk
pub fn save_to_xlsx(model: &Model, file_name: &str) -> Result<(), XlsxError>

/// Save Model to xlsx format in a writer
pub fn save_xlsx_to_writer<W: Write + Seek>(model: &Model, writer: W) -> Result<W, XlsxError>

/// Save to .icalc binary format
pub fn save_to_icalc(model: &Model, file_name: &str) -> Result<(), XlsxError>
```

## Error Handling

```rust
// error.rs

#[derive(Error, Debug)]
pub enum XlsxError {
    #[error("IO error: {0}")]
    IO(String),

    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("XML parsing error: {0}")]
    Xml(#[from] roxmltree::Error),

    #[error("Workbook error: {0}")]
    Workbook(String),

    #[error("Number parsing error: {0}")]
    ParseNumber(#[from] std::num::ParseFloatError),

    #[error("Integer parsing error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
}
```

## Import Architecture

### Main Import Flow

```rust
fn load_xlsx_from_reader<R: Read + Seek>(
    name: String,
    reader: R,
    locale: &str,
    tz: &str,
) -> Result<Workbook, XlsxError> {
    let mut archive = zip::ZipArchive::new(reader)?;

    // 1. Read shared strings first (referenced by cells)
    let mut shared_strings = read_shared_strings(&mut archive)?;

    // 2. Load workbook metadata
    let workbook = load_workbook(&mut archive)?;

    // 3. Load relationships (maps sheet IDs to files)
    let rels = load_relationships(&mut archive)?;

    // 4. Load tables
    let mut tables = HashMap::new();

    // 5. Load worksheets (references shared_strings)
    let (worksheets, selected_sheet) = load_sheets(
        &mut archive,
        &rels,
        &workbook,
        &mut tables,
        &mut shared_strings,
    )?;

    // 6. Load styles
    let styles = load_styles(&mut archive)?;

    // 7. Load metadata (optional)
    let metadata = load_metadata(&mut archive).unwrap_or_default();

    // 8. Build Workbook struct
    Ok(Workbook {
        shared_strings,
        defined_names: workbook.defined_names,
        worksheets,
        styles,
        name,
        settings: WorkbookSettings {
            tz: tz.to_string(),
            locale: locale.to_string(),
        },
        metadata,
        tables,
        views: /* ... */,
    })
}
```

### Worksheet XML Parsing

```rust
// import/worksheets.rs

pub fn load_worksheet<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    path: &str,
    shared_strings: &[String],
) -> Result<Worksheet, XlsxError> {
    let mut file = archive.by_name(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let doc = roxmltree::Document::parse(&content)?;
    let root = doc.root_element();

    // Parse sheet data
    let mut sheet_data: SheetData = HashMap::new();

    for sheet_data_el in root.children().find(|n| n.has_tag_name("sheetData")).into_iter() {
        for row_el in sheet_data_el.children().filter(|n| n.has_tag_name("row")) {
            let row_idx = get_attribute(row_el, "r")?.parse::<i32>()?;

            for cell_el in row_el.children().filter(|n| n.has_tag_name("c")) {
                let cell_ref = get_attribute(cell_el, "r")?;
                let (col, row) = parse_cell_reference(&cell_ref)?;

                let cell = parse_cell(cell_el, shared_strings)?;
                sheet_data.entry(row).or_insert_with(HashMap::new).insert(col, cell);
            }
        }
    }

    // Parse other worksheet properties...
    Ok(Worksheet {
        name: /* ... */,
        sheet_id: /* ... */,
        sheet_data,
        // ... other fields
    })
}
```

### Cell Parsing

```rust
fn parse_cell(
    cell_el: Node,
    shared_strings: &[String],
) -> Result<Cell, XlsxError> {
    let cell_type = get_attribute(cell_el, "t").unwrap_or("n");
    let style = get_attribute(cell_el, "s")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);

    let value_el = cell_el.children().find(|n| n.has_tag_name("v"));
    let value = value_el.and_then(|v| v.text()).unwrap_or("");

    match cell_type {
        "n" => {
            // Number
            let num = value.parse::<f64>().unwrap_or(0.0);
            Ok(Cell::NumberCell { v: num, s: style })
        }
        "s" => {
            // Shared string
            let si = value.parse::<i32>().unwrap_or(0);
            Ok(Cell::SharedString { si, s: style })
        }
        "b" => {
            // Boolean
            let v = value == "1" || value.to_lowercase() == "true";
            Ok(Cell::BooleanCell { v, s: style })
        }
        "e" => {
            // Error
            let error = parse_error(value)?;
            Ok(Cell::ErrorCell { ei: error, s: style })
        }
        "str" => {
            // String formula result
            let si = get_shared_string_index(value, shared_strings);
            Ok(Cell::SharedString { si, s: style })
        }
        _ => Ok(Cell::EmptyCell { s: style }),
    }
}
```

### Formula Parsing

```rust
fn parse_formula_cell(
    cell_el: Node,
    shared_strings: &[String],
) -> Result<Cell, XlsxError> {
    let style = /* ... */;

    if let Some(formula_el) = cell_el.children().find(|n| n.has_tag_name("f")) {
        let formula = formula_el.text().unwrap_or("");
        let formula_index = shared_formulas.len() as i32;
        shared_formulas.push(formula.to_string());

        // Check for cached value
        if let Some(value_el) = cell_el.children().find(|n| n.has_tag_name("v")) {
            let value = value_el.text().unwrap_or("");
            // Return formula cell with cached value
            // (will be re-evaluated by engine)
        }

        Ok(Cell::CellFormula { f: formula_index, s: style })
    } else {
        // No formula, check for value
        Ok(Cell::EmptyCell { s: style })
    }
}
```

## Export Architecture

### Main Export Flow

```rust
// export/mod.rs

pub fn save_xlsx_to_writer<W: Write + Seek>(
    model: &Model,
    writer: W,
) -> Result<W, XlsxError> {
    let workbook = &model.workbook;
    let mut zip = zip::ZipWriter::new(writer);
    let options = zip::write::FileOptions::default();

    // 1. [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(get_content_types_xml(workbook).as_bytes())?;

    // 2. docProps/
    zip.add_directory("docProps", options)?;
    zip.start_file("docProps/app.xml", options)?;
    zip.write_all(get_app_xml(workbook).as_bytes())?;
    zip.start_file("docProps/core.xml", options)?;
    zip.write_all(get_core_xml(workbook, timestamp).as_bytes())?;

    // 3. _rels/.rels
    zip.add_directory("_rels", options)?;
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(get_dot_rels(workbook).as_bytes())?;

    // 4. xl/
    zip.add_directory("xl", options)?;

    // 5. xl/sharedStrings.xml
    zip.start_file("xl/sharedStrings.xml", options)?;
    zip.write_all(get_shared_strings_xml(workbook).as_bytes())?;

    // 6. xl/styles.xml
    zip.start_file("xl/styles.xml", options)?;
    zip.write_all(get_styles_xml(workbook).as_bytes())?;

    // 7. xl/workbook.xml
    zip.start_file("xl/workbook.xml", options)?;
    zip.write_all(get_workbook_xml(workbook, selected_sheet).as_bytes())?;

    // 8. xl/_rels/workbook.xml.rels
    zip.add_directory("xl/_rels", options)?;
    zip.start_file("xl/_rels/workbook.xml.rels", options)?;
    zip.write_all(get_workbook_xml_rels(workbook).as_bytes())?;

    // 9. xl/worksheets/sheetN.xml
    zip.add_directory("xl/worksheets", options)?;
    for (sheet_index, worksheet) in workbook.worksheets.iter().enumerate() {
        zip.start_file(format!("xl/worksheets/sheet{}.xml", sheet_index + 1), options)?;
        let formulas = &model.parsed_formulas[sheet_index];
        zip.write_all(
            get_worksheet_xml(worksheet, formulas, dimension, is_selected).as_bytes()
        )?;
    }

    Ok(zip.finish()?)
}
```

### Content Types XML

```rust
fn get_content_types_xml(workbook: &Workbook) -> String {
    let mut content = vec![
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#.to_string(),
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#.to_string(),
        r#"<Default Extension="xml" ContentType="application/xml"/>"#.to_string(),
        r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#.to_string(),
    ];

    // Add worksheet types
    for worksheet in 0..workbook.worksheets.len() {
        content.push(format!(
            r#"<Override PartName="/xl/worksheets/sheet{}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
            worksheet + 1
        ));
    }

    content.extend([
        r#"<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#.to_string(),
        r#"<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>"#.to_string(),
        r#"<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>"#.to_string(),
        r#"<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>"#.to_string(),
        r#"</Types>"#.to_string(),
    ]);

    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", content.join(""))
}
```

### Worksheet XML Generation

```rust
// export/worksheets.rs

pub fn get_worksheet_xml(
    worksheet: &Worksheet,
    formulas: &[Vec<Node>],
    dimension: &str,
    is_selected: bool,
) -> String {
    let mut content = vec![
        format!(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#
        ),
        format!(r#"<dimension ref="{}"/>"#, dimension),
        r#"<sheetData>"#.to_string(),
    ];

    // Generate rows
    let mut sorted_rows: Vec<_> = worksheet.sheet_data.keys().collect();
    sorted_rows.sort();

    for row_idx in sorted_rows {
        let row_data = &worksheet.sheet_data[row_idx];
        content.push(format!(r#"<row r="{}">"#, row_idx));

        let mut sorted_cols: Vec<_> = row_data.keys().collect();
        sorted_cols.sort();

        for col_idx in sorted_cols {
            let cell = &row_data[col_idx];
            let col_str = number_to_column(*col_idx).unwrap();
            let cell_ref = format!("{}{}", col_str, row_idx);

            content.push(get_cell_xml(&cell_ref, cell, formulas, &worksheet.shared_formulas));
        }

        content.push(r#"</row>"#.to_string());
    }

    content.extend([
        r#"</sheetData>"#.to_string(),
        // ... mergeCells, sheetViews, etc.
        r#"</worksheet>"#.to_string(),
    ]);

    content.join("")
}
```

### Cell XML Generation

```rust
fn get_cell_xml(
    cell_ref: &str,
    cell: &Cell,
    formulas: &[Vec<Node>],
    shared_formulas: &[String],
) -> String {
    match cell {
        Cell::EmptyCell { s } => {
            format!(r#"<c r="{}" s="{}"/>"#, cell_ref, s)
        }
        Cell::NumberCell { v, s } => {
            format!(r#"<c r="{}" s="{}"><v>{}</v></c>"#, cell_ref, s, v)
        }
        Cell::BooleanCell { v, s } => {
            format!(r#"<c r="{}" t="b" s="{}"><v>{}</v></c>"#,
                cell_ref, s, if *v { 1 } else { 0 })
        }
        Cell::SharedString { si, s } => {
            format!(r#"<c r="{}" t="s" s="{}"><v>{}</v></c>"#, cell_ref, s, si)
        }
        Cell::CellFormula { f, s } => {
            let formula = &shared_formulas[*f as usize];
            format!(
                r#"<c r="{}" s="{}"><f>{}</f></c>"#,
                cell_ref, s, escape_xml(formula)
            )
        }
        Cell::CellFormulaNumber { f, v, s } => {
            let formula = &shared_formulas[*f as usize];
            format!(
                r#"<c r="{}" s="{}"><f>{}</f><v>{}</v></c>"#,
                cell_ref, s, escape_xml(formula), v
            )
        }
        Cell::ErrorCell { ei, s } => {
            let error_str = ei.to_string();
            format!(r#"<c r="{}" t="e" s="{}"><v>{}</v></c>"#, cell_ref, s, error_str)
        }
        // ... other cell types
    }
}
```

### Shared Strings

```rust
// export/shared_strings.rs

pub fn get_shared_strings_xml(workbook: &Workbook) -> String {
    let mut content = vec![
        format!(
            r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{}" uniqueCount="{}">"#,
            workbook.shared_strings.len(),
            workbook.shared_strings.len()
        ),
    ];

    for s in &workbook.shared_strings {
        content.push(format!(
            r#"<si><t>{}</t></si>"#,
            escape_xml(s)
        ));
    }

    content.push(r#"</sst>"#.to_string());
    content.join("")
}
```

### Styles XML

```rust
// export/styles.rs

pub fn get_styles_xml(workbook: &Workbook) -> String {
    let mut content = vec![
        r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#.to_string(),
    ];

    // Number formats
    content.push(format!(
        r#"<numFmts count="{}">"#,
        workbook.styles.num_fmts.len()
    ));
    for fmt in &workbook.styles.num_fmts {
        content.push(format!(
            r#"<numFmt numFmtId="{}" formatCode="{}"/>"#,
            fmt.id, escape_xml(&fmt.code)
        ));
    }
    content.push(r#"</numFmts>"#.to_string());

    // Fonts
    content.push(format!(r#"<fonts count="{}">"#, workbook.styles.fonts.len()));
    for font in &workbook.styles.fonts {
        content.push(get_font_xml(font));
    }
    content.push(r#"</fonts>"#.to_string());

    // Fills, Borders, CellXfs...
    // ...

    content.push(r#"</styleSheet>"#.to_string());
    content.join("")
}
```

## XLSX File Structure

```
workbook.xlsx
├── [Content_Types].xml       # List of all file types
├── docProps/
│   ├── app.xml               # Application properties
│   └── core.xml              # Core properties (author, dates)
├── _rels/
│   └── .rels                 # Package relationships
└── xl/
    ├── _rels/
    │   └── workbook.xml.rels # Workbook relationships
    ├── worksheets/
    │   ├── sheet1.xml
    │   ├── sheet2.xml
    │   └── ...
    ├── sharedStrings.xml     # Shared string table
    ├── styles.xml            # Styles definition
    └── workbook.xml          # Workbook definition
```

## XML Constants

```rust
// xml_constants.rs

pub const XML_DECLARATION: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;

pub const WORKBOOK_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub const RELATIONSHIPS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
pub const DOCUMENT_PROPS_NS: &str = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
```

## Escape Utilities

```rust
// escape.rs

pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
```

## Comparison Module

```rust
// compare.rs

pub fn compare_workbooks(wb1: &Workbook, wb2: &Workbook) -> Vec<String> {
    let mut differences = Vec::new();

    // Compare sheet count
    if wb1.worksheets.len() != wb2.worksheets.len() {
        differences.push(format!(
            "Sheet count differs: {} vs {}",
            wb1.worksheets.len(),
            wb2.worksheets.len()
        ));
    }

    // Compare sheet names
    for (i, (s1, s2)) in wb1.worksheets.iter().zip(wb2.worksheets.iter()).enumerate() {
        if s1.name != s2.name {
            differences.push(format!("Sheet {} name differs", i));
        }
    }

    // ... more comparisons

    differences
}
```

## Testing

```rust
#[cfg(test)]
mod test {
    use crate::{import::load_from_xlsx, export::save_to_xlsx, Model};

    #[test]
    fn test_round_trip() {
        // Create model
        let mut model = Model::new_empty("test", "en", "UTC").unwrap();
        model.set_user_input(0, 1, 1, "=SUM(A2:A3)".to_string());
        model.set_user_input(0, 2, 1, "10".to_string());
        model.set_user_input(0, 3, 1, "20".to_string());
        model.evaluate();

        // Save to file
        save_to_xlsx(&model, "/tmp/test.xlsx").unwrap();

        // Load back
        let model2 = load_from_xlsx("/tmp/test.xlsx", "en", "UTC").unwrap();

        // Verify
        assert_eq!(
            model.get_formatted_cell_value(0, 1, 1).unwrap(),
            model2.get_formatted_cell_value(0, 1, 1).unwrap()
        );
    }
}
```

## Integration with ironcalc_base

```rust
// lib.rs

pub use ironcalc_base as base;
pub use ironcalc_base::{Model, UserModel};

pub mod export;
pub mod import;
pub mod error;
pub mod compare;

/// High-level API
pub fn new_empty(name: &str, locale: &str, tz: &str) -> Result<Model, String> {
    Model::new_empty(name, locale, tz)
}

pub fn load_xlsx(path: &str, locale: &str, tz: &str) -> Result<Model, XlsxError> {
    import::load_from_xlsx(path, locale, tz)
}

pub fn save_xlsx(model: &Model, path: &str) -> Result<(), XlsxError> {
    export::save_to_xlsx(model, path)
}
```

## Performance Considerations

1. **Streaming**: Read/write worksheets one at a time
2. **Shared Strings**: Deduplicate strings across workbook
3. **Minimal XML**: Only write non-default values
4. **ZIP Compression**: Default compression for balance of speed/size
