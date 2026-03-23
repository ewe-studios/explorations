# facet-serialize and facet-deserialize Deep Dive

## Overview

These crates provide the core framework for format-agnostic serialization and deserialization using Facet's reflection system.

## facet-serialize

### Serializer Trait

```rust
pub trait Serializer<'shape> {
    type Error;

    // Primitives
    fn serialize_u64(&mut self, value: u64) -> Result<(), Self::Error>;
    fn serialize_u128(&mut self, value: u128) -> Result<(), Self::Error>;
    fn serialize_i64(&mut self, value: i64) -> Result<(), Self::Error>;
    fn serialize_i128(&mut self, value: i128) -> Result<(), Self::Error>;
    fn serialize_f64(&mut self, value: f64) -> Result<(), Self::Error>;
    fn serialize_bool(&mut self, value: bool) -> Result<(), Self::Error>;
    fn serialize_char(&mut self, value: char) -> Result<(), Self::Error>;
    fn serialize_str(&mut self, value: &str) -> Result<(), Self::Error>;
    fn serialize_bytes(&mut self, value: &[u8]) -> Result<(), Self::Error>;

    // Special values
    fn serialize_none(&mut self) -> Result<(), Self::Error>;
    fn serialize_unit(&mut self) -> Result<(), Self::Error>;
    fn serialize_unit_variant(
        &mut self,
        variant_index: usize,
        variant_name: &'shape str,
    ) -> Result<(), Self::Error>;

    // Containers
    fn start_object(&mut self, len: Option<usize>) -> Result<(), Self::Error>;
    fn serialize_field_name(&mut self, name: &'shape str) -> Result<(), Self::Error>;
    fn start_array(&mut self, len: Option<usize>) -> Result<(), Self::Error>;
    fn start_map(&mut self, len: Option<usize>) -> Result<(), Self::Error>;
}
```

### Default Implementations

```rust
impl<'shape, S: Serializer<'shape>> Serializer<'shape> for &mut S {
    type Error = S::Error;

    #[inline(always)]
    fn serialize_u8(&mut self, value: u8) -> Result<(), Self::Error> {
        (**self).serialize_u8(value)
    }

    // ... delegating implementations
}
```

### Core Serialization Function

```rust
pub fn serialize<'shape, S: Serializer<'shape>>(
    peek: &Peek<'_, '_, 'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    // Handle transparent wrappers
    if peek.shape().inner.is_some() {
        return serialize_transparent(peek, serializer);
    }

    match peek.shape().def {
        Def::Scalar(scalar_def) => serialize_scalar(peek, scalar_def, serializer),

        Def::Struct(_) => serialize_struct(peek, serializer),

        Def::Enum(_) => serialize_enum(peek, serializer),

        Def::List(_) => serialize_list(peek, serializer),

        Def::Map(_) => serialize_map(peek, serializer),

        Def::Array(_) => serialize_array(peek, serializer),

        Def::Option(_) => serialize_option(peek, serializer),

        Def::SmartPointer(_) => serialize_smart_pointer(peek, serializer),

        Def::Undefined => serialize_undefined(peek, serializer),

        Def::Set(_) => serialize_set(peek, serializer),

        Def::Slice(_) => serialize_slice(peek, serializer),
    }
}
```

### Scalar Serialization

```rust
fn serialize_scalar<'shape, S: Serializer<'shape>>(
    peek: &Peek<'_, '_, 'shape>,
    scalar_def: ScalarDef<'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    match scalar_def.affinity {
        ScalarAffinity::Number(number_affinity) => {
            match number_affinity.bits {
                NumberBits::Integer { size, sign } => {
                    match (size, sign) {
                        (IntegerSize::Fixed(8), Signedness::Unsigned) => {
                            let v = peek.as_scalar().unwrap().as_u8().unwrap();
                            serializer.serialize_u8(v)
                        }
                        (IntegerSize::Fixed(32), Signedness::Unsigned) => {
                            let v = peek.as_scalar().unwrap().as_u32().unwrap();
                            serializer.serialize_u64(v as u64)
                        }
                        (IntegerSize::Fixed(64), Signedness::Unsigned) => {
                            let v = peek.as_scalar().unwrap().as_u64().unwrap();
                            serializer.serialize_u64(v)
                        }
                        (IntegerSize::Fixed(64), Signedness::Signed) => {
                            let v = peek.as_scalar().unwrap().as_i64().unwrap();
                            serializer.serialize_i64(v)
                        }
                        (IntegerSize::Fixed(128), Signedness::Unsigned) => {
                            let v = peek.as_scalar().unwrap().as_u128().unwrap();
                            serializer.serialize_u128(v)
                        }
                        // ... more cases
                        _ => Err(S::Error::custom("unsupported integer type")),
                    }
                }
                NumberBits::Float { sign_bits, exponent_bits, mantissa_bits } => {
                    match (sign_bits, exponent_bits, mantissa_bits) {
                        (1, 8, 23) => {
                            // f32
                            let v = peek.as_scalar().unwrap().as_f32().unwrap();
                            serializer.serialize_f64(v as f64)
                        }
                        (1, 11, 52) => {
                            // f64
                            let v = peek.as_scalar().unwrap().as_f64().unwrap();
                            serializer.serialize_f64(v)
                        }
                        _ => Err(S::Error::custom("unsupported float type")),
                    }
                }
                _ => Err(S::Error::custom("unsupported number format")),
            }
        }

        ScalarAffinity::String(_) => {
            if let Some(ScalarType::Str(s)) = peek.as_scalar() {
                serializer.serialize_str(s)
            } else if let Some(ScalarType::String(s)) = peek.as_scalar() {
                serializer.serialize_str(s)
            } else {
                Err(S::Error::custom("expected string value"))
            }
        }

        ScalarAffinity::Boolean(_) => {
            let v = peek.as_scalar().unwrap().as_bool().unwrap();
            serializer.serialize_bool(v)
        }

        ScalarAffinity::Char(_) => {
            let v = peek.as_scalar().unwrap().as_char().unwrap();
            serializer.serialize_char(v)
        }

        // Handle opaque types that might have Display
        ScalarAffinity::Opaque(_) | ScalarAffinity::Other(_) => {
            // Try to get string representation via Display
            if let Some(display_str) = peek.display_to_string() {
                serializer.serialize_str(&display_str)
            } else {
                Err(S::Error::custom("cannot serialize opaque type"))
            }
        }

        _ => Err(S::Error::custom("unsupported scalar affinity")),
    }
}
```

### Struct Serialization

```rust
fn serialize_struct<'shape, S: Serializer<'shape>>(
    peek: &Peek<'_, '_, 'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    let struct_val = peek.into_struct()
        .map_err(|e| S::Error::custom(&format!("not a struct: {}", e)))?;

    // Get fields to serialize (may skip some based on flags)
    let fields_iter = struct_val.fields_for_serialize();
    let field_count = fields_iter.clone().count();

    serializer.start_object(Some(field_count))?;

    for field_result in fields_iter {
        let (name, field_peek) = field_result;

        // Check if we should skip this field
        if should_skip_field(&field_peek, name) {
            continue;
        }

        serializer.serialize_field_name(name)?;
        serialize(&field_peek, serializer)?;
    }

    serializer.end_object()
}

fn should_skip_field<'shape>(field_peek: &Peek<'_, '_, 'shape>, name: &str) -> bool {
    // Check FieldFlags::SKIP_SERIALIZING
    // Check FieldFlags::SENSITIVE (if configured)
    // Check skip_serializing_if function
    false
}
```

### Enum Serialization

```rust
fn serialize_enum<'shape, S: Serializer<'shape>>(
    peek: &Peek<'_, '_, 'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    let enum_val = peek.into_enum()
        .map_err(|e| S::Error::custom(&format!("not an enum: {}", e)))?;

    let variant = enum_val.variant();
    let variant_name = variant.name;
    let variant_index = enum_val.variant_index();

    // Unit variant
    if variant.data.fields.is_empty() {
        return serializer.serialize_unit_variant(variant_index, variant_name);
    }

    // Newtype variant (single field, tuple-like)
    if variant_is_newtype_like(variant) {
        let field = enum_val.data().field(0).unwrap();
        return serialize(&field, serializer);
    }

    // Struct-like variant
    serializer.start_object(Some(variant.data.fields.len()))?;
    serializer.serialize_field_name(variant_name)?;

    let variant_data = enum_val.data();
    for field_result in variant_data.fields() {
        let (name, field_peek) = field_result;
        serializer.serialize_field_name(name)?;
        serialize(&field_peek, serializer)?;
    }

    serializer.end_object()
}
```

### List/Array Serialization

```rust
fn serialize_list<'shape, S: Serializer<'shape>>(
    peek: &Peek<'_, '_, 'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    let list = peek.into_list_like()
        .map_err(|e| S::Error::custom(&format!("not a list: {}", e)))?;

    serializer.start_array(Some(list.len()))?;

    for item in list.iter() {
        serialize(&item, serializer)?;
    }

    serializer.end_array()
}

fn serialize_array<'shape, S: Serializer<'shape>>(
    peek: &Peek<'_, '_, 'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    let array = peek.into_list_like()
        .map_err(|e| S::Error::custom(&format!("not an array: {}", e)))?;

    serializer.start_array(Some(array.len()))?;

    for item in array.iter() {
        serialize(&item, serializer)?;
    }

    serializer.end_array()
}
```

### Map Serialization

```rust
fn serialize_map<'shape, S: Serializer<'shape>>(
    peek: &Peek<'_, '_, 'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    let map = peek.into_map()
        .map_err(|e| S::Error::custom(&format!("not a map: {}", e)))?;

    serializer.start_map(Some(map.len()))?;

    for (key, value) in map.iter() {
        serialize(&key, serializer)?;
        serialize(&value, serializer)?;
    }

    serializer.end_map()
}
```

### Option Serialization

```rust
fn serialize_option<'shape, S: Serializer<'shape>>(
    peek: &Peek<'_, '_, 'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    let opt = peek.into_option()
        .map_err(|e| S::Error::custom(&format!("not an option: {}", e)))?;

    if opt.is_some() {
        let inner = opt.get().unwrap();
        serialize(&inner, serializer)
    } else {
        serializer.serialize_none()
    }
}
```

## facet-deserialize

### Deserializer Trait

```rust
pub trait Deserializer<'input> {
    type Error;

    /// Get next input element
    fn next(&mut self) -> Result<Outcome<'input>, Self::Error>;

    /// Peek at next element without consuming
    fn peek(&mut self) -> Result<Option<&Outcome<'input>>, Self::Error>;

    /// Expect a specific outcome
    fn expect(&mut self, expected: Expectation) -> Result<Outcome<'input>, Self::Error>;
}
```

### Outcome Types

```rust
#[derive(PartialEq, Debug, Clone)]
pub enum Outcome<'input> {
    Scalar(Scalar<'input>),
    ListStarted,
    ListEnded,
    ObjectStarted,
    ObjectEnded,
    ObjectKey(Cow<'input, str>),
    Resegmented(Vec<Subspan>),
}

#[derive(PartialEq, Debug, Clone)]
pub enum Scalar<'input> {
    String(Cow<'input, str>),
    U64(u64),
    I64(i64),
    F64(f64),
    U128(u128),
    I128(i128),
    Bool(bool),
    Null,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Expectation {
    Value,
    ObjectKeyOrObjectClose,
    ObjectVal,
    ListItemOrListClose,
}
```

### Core Deserialization Function

```rust
pub fn deserialize<'input, 'shape, T: Facet<'shape>>(
    deserializer: &mut impl Deserializer<'input>,
) -> Result<T, DeserializerError> {
    let mut partial = Partial::alloc::<T>()?;
    deserialize_into(&mut partial, deserializer)?;
    Ok(partial.build()?)
}

pub fn deserialize_into<'input, 'shape>(
    partial: &mut Partial<'shape>,
    deserializer: &mut impl Deserializer<'input>,
) -> Result<(), DeserializerError> {
    let shape = partial.shape();

    match shape.def {
        Def::Scalar(_) => deserialize_scalar(partial, deserializer),
        Def::Struct(_) => deserialize_struct(partial, deserializer),
        Def::Enum(_) => deserialize_enum(partial, deserializer),
        Def::List(_) => deserialize_list(partial, deserializer),
        Def::Map(_) => deserialize_map(partial, deserializer),
        Def::Array(_) => deserialize_array(partial, deserializer),
        Def::Option(_) => deserialize_option(partial, deserializer),
        _ => Err(DeserializerError::UnsupportedType {
            shape: shape.type_identifier,
        }),
    }
}
```

### Scalar Deserialization

```rust
fn deserialize_scalar<'input, 'shape>(
    partial: &mut Partial<'shape>,
    deserializer: &mut impl Deserializer<'input>,
) -> Result<(), DeserializerError> {
    let outcome = deserializer.next()?;

    match outcome {
        Outcome::Scalar(scalar) => {
            let shape = partial.shape();

            // Match scalar to target type
            match scalar {
                Scalar::U64(v) => {
                    // Check if target type can accept u64
                    if shape.is_type::<u64>() {
                        partial.set(v)?;
                    } else if shape.is_type::<u32>() && v <= u32::MAX as u64 {
                        partial.set(v as u32)?;
                    } else if shape.is_type::<i64>() && v <= i64::MAX as u64 {
                        partial.set(v as i64)?;
                    } else {
                        return Err(DeserializerError::TypeMismatch {
                            expected: shape.type_identifier,
                            got: "u64",
                        });
                    }
                }

                Scalar::I64(v) => {
                    if shape.is_type::<i64>() {
                        partial.set(v)?;
                    } else if shape.is_type::<i32>()
                        && v >= i32::MIN as i64
                        && v <= i32::MAX as i64
                    {
                        partial.set(v as i32)?;
                    } else {
                        return Err(DeserializerError::TypeMismatch {
                            expected: shape.type_identifier,
                            got: "i64",
                        });
                    }
                }

                Scalar::F64(v) => {
                    if shape.is_type::<f64>() {
                        partial.set(v)?;
                    } else if shape.is_type::<f32>() && (v as f32) as f64 == v {
                        partial.set(v as f32)?;
                    } else {
                        return Err(DeserializerError::TypeMismatch {
                            expected: shape.type_identifier,
                            got: "f64",
                        });
                    }
                }

                Scalar::Bool(v) => {
                    if shape.is_type::<bool>() {
                        partial.set(v)?;
                    } else {
                        return Err(DeserializerError::TypeMismatch {
                            expected: shape.type_identifier,
                            got: "bool",
                        });
                    }
                }

                Scalar::String(s) => {
                    // Try string types
                    if shape.is_type::<String>() {
                        partial.set(s.into_owned())?;
                    } else if shape.is_type::<&str>() {
                        // Can't set &str from owned String
                        // Need to handle this specially
                        return Err(DeserializerError::TypeMismatch {
                            expected: shape.type_identifier,
                            got: "&str",
                        });
                    } else {
                        // Try parsing
                        return try_parse_string(s.as_ref(), partial, shape);
                    }
                }

                Scalar::Null => {
                    // Only valid for Option types
                    if !matches!(shape.def, Def::Option(_)) {
                        return Err(DeserializerError::UnexpectedNull);
                    }
                    partial.set_none()?;
                }

                _ => {
                    return Err(DeserializerError::UnsupportedScalar {
                        scalar_type: format!("{:?}", scalar),
                    });
                }
            }

            Ok(())
        }

        _ => Err(DeserializerError::ExpectedScalar {
            got: format!("{:?}", outcome),
        }),
    }
}

fn try_parse_string<'input, 'shape>(
    s: &str,
    partial: &mut Partial<'shape>,
    shape: &'shape Shape<'shape>,
) -> Result<(), DeserializerError> {
    // Try using the shape's parse function
    if let Some(parse_fn) = shape.vtable.parse() {
        let layout = shape.layout.sized_layout()
            .map_err(|_| DeserializerError::UnsizedType)?;

        let ptr = unsafe { alloc::alloc::alloc(layout) };
        let uninit = PtrUninit::new(ptr);

        match unsafe { parse_fn(s, uninit) } {
            Ok(_) => {
                // Successfully parsed, now set in partial
                // This requires some pointer manipulation
                Ok(())
            }
            Err(e) => Err(DeserializerError::ParseError {
                type_name: shape.type_identifier,
                value: s.to_string(),
                error: format!("{:?}", e),
            }),
        }
    } else {
        Err(DeserializerError::CannotParse {
            type_name: shape.type_identifier,
        })
    }
}
```

### Struct Deserialization

```rust
fn deserialize_struct<'input, 'shape>(
    partial: &mut Partial<'shape>,
    deserializer: &mut impl Deserializer<'input>,
) -> Result<(), DeserializerError> {
    // Expect object start
    let outcome = deserializer.next()?;
    if !matches!(outcome, Outcome::ObjectStarted) {
        return Err(DeserializerError::ExpectedObject {
            got: format!("{:?}", outcome),
        });
    }

    // Check for default attribute
    let use_default = partial.shape().has_default_attr();

    // Process fields
    loop {
        let key_outcome = deserializer.next()?;

        match key_outcome {
            Outcome::ObjectEnded => break,  // End of object

            Outcome::ObjectKey(key) => {
                let field_name = key.as_ref();

                // Find matching field
                if let Err(e) = partial.begin_field(field_name) {
                    if partial.shape().has_deny_unknown_fields_attr() {
                        return Err(DeserializerError::UnknownField {
                            field: field_name.to_string(),
                            type_name: partial.shape().type_identifier,
                        });
                    }
                    // Skip unknown field value
                    skip_value(deserializer)?;
                    continue;
                }

                // Deserialize field value
                deserialize_into(partial, deserializer)?;
                partial.end()?;
            }

            _ => {
                return Err(DeserializerError::ExpectedObjectKey {
                    got: format!("{:?}", key_outcome),
                });
            }
        }
    }

    // Fill missing fields with defaults if attribute is set
    if use_default {
        fill_defaults(partial)?;
    }

    Ok(())
}

fn skip_value<'input>(
    deserializer: &mut impl Deserializer<'input>,
) -> Result<(), DeserializerError> {
    // Skip next value (handles nested structures)
    let outcome = deserializer.next()?;
    match outcome {
        Outcome::Scalar(_) => Ok(()),
        Outcome::ListStarted => {
            loop {
                let item = deserializer.next()?;
                match item {
                    Outcome::ListEnded => break,
                    Outcome::ObjectStarted | Outcome::ListStarted => skip_value(deserializer)?,
                    _ => {}
                }
            }
            Ok(())
        }
        Outcome::ObjectStarted => {
            loop {
                let key = deserializer.next()?;
                match key {
                    Outcome::ObjectEnded => break,
                    Outcome::ObjectKey(_) => {
                        skip_value(deserializer)?;
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
```

### Enum Deserialization

```rust
fn deserialize_enum<'input, 'shape>(
    partial: &mut Partial<'shape>,
    deserializer: &mut impl Deserializer<'input>,
) -> Result<(), DeserializerError> {
    // Look ahead to determine variant
    let outcome = deserializer.peek()?;

    match outcome {
        Some(Outcome::Scalar(Scalar::String(variant_name))) => {
            // String variant name (external tagging)
            deserializer.next()?;  // Consume the peeked value

            // Set variant
            partial.set_variant(variant_name.as_ref())?;

            // Check what's next
            let next = deserializer.peek()?;
            match next {
                Some(Outcome::ObjectStarted) => {
                    // Variant data follows
                    deserializer.next()?;  // Consume ObjectStarted
                    deserialize_enum_fields(partial, deserializer)?;
                    deserializer.expect(Expectation::Value)?;  // Consume ObjectEnded
                }
                _ => {
                    // Unit variant or newtype with scalar
                }
            }

            Ok(())
        }

        Some(Outcome::ObjectStarted) => {
            // Could be internally tagged or adjacently tagged
            deserializer.next()?;  // Consume ObjectStarted

            let key_outcome = deserializer.next()?;
            match key_outcome {
                Outcome::ObjectKey(variant_name) => {
                    partial.set_variant(variant_name.as_ref())?;
                    deserialize_enum_fields(partial, deserializer)?;
                }
                _ => {
                    return Err(DeserializerError::ExpectedVariantName {
                        got: format!("{:?}", key_outcome),
                    });
                }
            }

            deserializer.expect(Expectation::ObjectKeyOrObjectClose)?;
            Ok(())
        }

        _ => Err(DeserializerError::ExpectedEnumVariant {
            got: format!("{:?}", outcome),
        }),
    }
}
```

### List Deserialization

```rust
fn deserialize_list<'input, 'shape>(
    partial: &mut Partial<'shape>,
    deserializer: &mut impl Deserializer<'input>,
) -> Result<(), DeserializerError> {
    // Expect list start
    let outcome = deserializer.next()?;
    if !matches!(outcome, Outcome::ListStarted) {
        return Err(DeserializerError::ExpectedList {
            got: format!("{:?}", outcome),
        });
    }

    // Process list items
    loop {
        let item_outcome = deserializer.peek()?;

        match item_outcome {
            Some(Outcome::ListEnded) => {
                deserializer.next()?;  // Consume ListEnded
                break;
            }

            _ => {
                partial.begin_list_item()?;
                deserialize_into(partial, deserializer)?;
                partial.end()?;
            }
        }
    }

    Ok(())
}
```

### Map Deserialization

```rust
fn deserialize_map<'input, 'shape>(
    partial: &mut Partial<'shape>,
    deserializer: &mut impl Deserializer<'input>,
) -> Result<(), DeserializerError> {
    // Expect map start
    let outcome = deserializer.next()?;
    if !matches!(outcome, Outcome::ObjectStarted)
        && !matches!(outcome, Outcome::ListStarted)
    {
        return Err(DeserializerError::ExpectedMap {
            got: format!("{:?}", outcome),
        });
    }

    loop {
        let key_outcome = deserializer.peek()?;

        match key_outcome {
            Some(Outcome::ObjectEnded) | Some(Outcome::ListEnded) => {
                deserializer.next()?;  // Consume the end marker
                break;
            }

            Some(Outcome::ObjectKey(key)) => {
                deserializer.next()?;  // Consume the key
                partial.begin_map_key(key.as_ref())?;
                deserialize_into(partial, deserializer)?;
                partial.end()?;
            }

            _ => {
                // Key might be a scalar in list format
                let key_outcome = deserializer.next()?;
                if let Outcome::Scalar(scalar) = key_outcome {
                    partial.begin_map_key(&scalar.to_string())?;
                    deserialize_into(partial, deserializer)?;
                    partial.end()?;
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}
```

### Error Handling

```rust
#[derive(Debug, Clone)]
pub enum DeserializerError {
    /// Type mismatch during deserialization
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },

    /// Unknown field (when deny_unknown_fields is set)
    UnknownField {
        field: String,
        type_name: String,
    },

    /// Missing required field
    MissingField {
        field: String,
        type_name: String,
    },

    /// Invalid enum variant
    InvalidVariant {
        variant: String,
        expected: Vec<String>,
    },

    /// Parse error for string-to-type conversion
    ParseError {
        type_name: String,
        value: String,
        error: String,
    },

    /// Unexpected null value
    UnexpectedNull,

    /// Expected scalar but got something else
    ExpectedScalar {
        got: String,
    },

    /// Expected object but got something else
    ExpectedObject {
        got: String,
    },

    /// Expected list but got something else
    ExpectedList {
        got: String,
    },

    /// Expected map but got something else
    ExpectedMap {
        got: String,
    },

    /// Expected enum variant but got something else
    ExpectedEnumVariant {
        got: String,
    },

    /// Unsupported scalar type
    UnsupportedScalar {
        scalar_type: String,
    },

    /// Unsupported target type
    UnsupportedType {
        shape: &'static str,
    },

    /// Reflect error from Partial
    Reflect(String),

    /// Input error from deserializer
    Input(String),
}

impl core::fmt::Display for DeserializerError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            DeserializerError::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {}, got {}", expected, got)
            }
            DeserializerError::UnknownField { field, type_name } => {
                write!(f, "unknown field '{}' in {}", field, type_name)
            }
            DeserializerError::MissingField { field, type_name } => {
                write!(f, "missing required field '{}' in {}", field, type_name)
            }
            // ... other variants
        }
    }
}
```

## Format Implementations

### JSON (facet-json)

```rust
// In facet-json/src/de.rs

pub struct JsonDeserializer<'input> {
    input: &'input [u8],
    pos: usize,
    current: Option<Outcome<'input>>,
}

impl<'input> JsonDeserializer<'input> {
    pub fn new(input: &'input [u8]) -> Self {
        Self {
            input,
            pos: 0,
            current: None,
        }
    }

    fn parse_next(&mut self) -> Result<Outcome<'input>, JsonError> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Ok(Outcome::ObjectEnded);  // EOF
        }

        let byte = self.input[self.pos];

        match byte {
            b'{' => {
                self.pos += 1;
                Ok(Outcome::ObjectStarted)
            }
            b'}' => {
                self.pos += 1;
                Ok(Outcome::ObjectEnded)
            }
            b'[' => {
                self.pos += 1;
                Ok(Outcome::ListStarted)
            }
            b']' => {
                self.pos += 1;
                Ok(Outcome::ListEnded)
            }
            b'"' => self.parse_string(),
            b'0'..=b'9' | b'-' => self.parse_number(),
            b't' => self.parse_true(),
            b'f' => self.parse_false(),
            b'n' => self.parse_null(),
            _ => Err(JsonError::UnexpectedCharacter(byte, self.pos)),
        }
    }
}

impl<'input> Deserializer<'input> for JsonDeserializer<'input> {
    type Error = JsonError;

    fn next(&mut self) -> Result<Outcome<'input>, Self::Error> {
        let outcome = self.parse_next()?;
        self.current = Some(outcome.clone());
        Ok(outcome)
    }

    fn peek(&mut self) -> Result<Option<&Outcome<'input>>, Self::Error> {
        if self.current.is_none() {
            self.current = Some(self.parse_next()?);
        }
        Ok(self.current.as_ref())
    }
}
```

## Summary

The serialization/deserialization framework in facet:

1. **Format-agnostic**: Works with any format that implements the traits
2. **Type-driven**: Uses Shape metadata for type information
3. **Incremental**: Builds values field-by-field via Partial
4. **Error-tolerant**: Clear error messages for debugging
5. **Flexible**: Supports multiple tagging strategies for enums
