//! Pure Rust Java .class file constant pool parser for Spring Bean discovery.
//!
//! Uses zero external dependencies — only `std::io::Cursor` with manual binary
//! reads to extract @Bean, @Component (and stereotype), @Import, and
//! @ConfigurationProperties definitions from compiled .class files.

use crate::classpath::DiscoveryMethod;
use std::io::{Cursor, Read, Seek, SeekFrom};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Parsed .class file with discovered Bean definitions.
#[derive(Debug, Clone)]
pub struct ParsedClass {
    /// Fully-qualified class name in internal JVM form (e.g. `com/example/MyConfig`).
    pub class_name: String,
    /// Beans discovered in this class (from @Bean methods or class-level stereotypes).
    pub beans: Vec<DiscoveredBean>,
    /// Fully-qualified class names referenced in @Import annotations (dot-separated).
    pub imported_classes: Vec<String>,
    /// Interfaces implemented by this class (simple names, e.g. "OAuth2AuthorizationService").
    pub interfaces: Vec<String>,
}

/// A single Bean definition discovered in a .class file.
#[derive(Debug, Clone)]
pub struct DiscoveredBean {
    /// Spring-compatible bean name (method name or explicit `name` attribute).
    pub bean_name: String,
    /// Simple class name of the bean type (derived from return-type descriptor).
    pub bean_type: String,
    /// Fully-qualified class name (dot-separated) — e.g. "javax.sql.DataSource"
    pub bean_type_fqcn: String,
    /// How this bean was discovered.
    pub discovery_method: DiscoveryMethod,
}

// ---------------------------------------------------------------------------
// .class file format constants (JVMS 4.4)
// ---------------------------------------------------------------------------

const CONSTANT_UTF8: u8 = 1;

// ---------------------------------------------------------------------------
// Binary read helpers
// ---------------------------------------------------------------------------

fn read_u16(cursor: &mut Cursor<&[u8]>) -> std::io::Result<u16> {
    let mut buf = [0u8; 2];
    cursor.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

// ---------------------------------------------------------------------------
// String table — maps constant-pool indices to UTF-8 strings (1-indexed)
// ---------------------------------------------------------------------------
// Main parse entry point
// ---------------------------------------------------------------------------

/// Parse a .class file from raw bytes and extract Spring Bean definitions.
///
/// Returns a `ParsedClass` with discovered beans, imported classes, and the
/// fully-qualified class name.  Errors are returned as human-readable `String`
/// messages because this module is intentionally dependency-free.
pub fn parse_class(bytes: &[u8]) -> Result<ParsedClass, String> {
    let mut cursor = Cursor::new(bytes);

    // 1. Magic number: 0xCAFEBABE
    let magic = read_u32(&mut cursor).map_err(|e| format!("read magic: {e}"))?;
    if magic != 0xCAFEBABE {
        return Err(format!("not a valid class file (magic={magic:#X})"));
    }

    // 2. Version (minor_version + major_version) — skip
    read_u16(&mut cursor).map_err(|e| format!("read minor_version: {e}"))?;
    read_u16(&mut cursor).map_err(|e| format!("read major_version: {e}"))?;

    // 3. Constant pool (JVMS 4.4)
    let pool_count = read_u16(&mut cursor).map_err(|e| format!("read pool_count: {e}"))?;
    let strings = parse_constant_pool(&mut cursor, pool_count)?;

    // 4. Access flags, this_class, super_class
    read_u16(&mut cursor).map_err(|e| format!("read access_flags: {e}"))?;
    let this_class_idx = read_u16(&mut cursor).map_err(|e| format!("read this_class: {e}"))?;
    let class_name = strings
        .get(this_class_idx)
        .unwrap_or("Unknown")
        .to_string();
    let simple_class_name = class_name
        .rsplit('/')
        .next()
        .unwrap_or(&class_name)
        .to_string();
    read_u16(&mut cursor).map_err(|e| format!("read super_class: {e}"))?;

    // 5. Interfaces — extract for interface→impl matching (004)
    let iface_count = read_u16(&mut cursor).map_err(|e| format!("read iface_count: {e}"))?;
    let mut interfaces: Vec<String> = Vec::new();
    for _ in 0..iface_count {
        let iface_idx = read_u16(&mut cursor).map_err(|e| format!("read interface: {e}"))?;
        if let Some(iface_name) = strings.get(iface_idx) {
            // Extract simple name from JVM internal form
            let simple = iface_name.rsplit('/').next().unwrap_or(iface_name).to_string();
            interfaces.push(simple);
        }
    }

    // 6. Fields — skip entirely
    let field_count = read_u16(&mut cursor).map_err(|e| format!("read field_count: {e}"))?;
    for _ in 0..field_count {
        skip_field_or_method(&mut cursor)?;
    }

    // 7. Methods — inspect RuntimeVisibleAnnotations for @Bean
    let mut beans: Vec<DiscoveredBean> = Vec::new();
    let method_count = read_u16(&mut cursor).map_err(|e| format!("read method_count: {e}"))?;
    for _ in 0..method_count {
        read_u16(&mut cursor).map_err(|e| format!("read method access_flags: {e}"))?;
        let name_idx = read_u16(&mut cursor).map_err(|e| format!("read method name_index: {e}"))?;
        let desc_idx =
            read_u16(&mut cursor).map_err(|e| format!("read method descriptor_index: {e}"))?;
        let method_name = strings.get(name_idx).unwrap_or("").to_string();
        let method_desc = strings.get(desc_idx).unwrap_or("").to_string();

        let attr_count =
            read_u16(&mut cursor).map_err(|e| format!("read method attributes_count: {e}"))?;
        let mut has_bean_annotation = false;
        let mut bean_name_override: Option<String> = None;

        for _ in 0..attr_count {
            let attr_name_idx =
                read_u16(&mut cursor).map_err(|e| format!("read method attr name_index: {e}"))?;
            let attr_len =
                read_u32(&mut cursor).map_err(|e| format!("read method attr length: {e}"))?;
            let attr_name = strings.get(attr_name_idx).unwrap_or("");

            if attr_name == "RuntimeVisibleAnnotations" {
                let (found_bean, explicit_name) =
                    parse_method_annotations(&mut cursor, &strings, attr_len)?;
                has_bean_annotation = found_bean;
                bean_name_override = explicit_name;
            } else {
                cursor
                    .seek(SeekFrom::Current(attr_len as i64))
                    .map_err(|e| format!("skip method attr data: {e}"))?;
            }
        }

        if has_bean_annotation {
            let (bean_type, bean_type_fqcn) = parse_return_type_full(&method_desc);
            let bean_name = bean_name_override.unwrap_or(method_name);
            beans.push(DiscoveredBean {
                bean_name,
                bean_type,
                bean_type_fqcn,
                discovery_method: DiscoveryMethod::BeanAnnotation,
            });
        }
    }

    // 8. Class-level attributes — inspect RuntimeVisibleAnnotations for
    //    stereotypes, @Import, and @ConfigurationProperties.
    let mut imported_classes: Vec<String> = Vec::new();
    let class_attr_count =
        read_u16(&mut cursor).map_err(|e| format!("read class attributes_count: {e}"))?;
    for _ in 0..class_attr_count {
        let attr_name_idx =
            read_u16(&mut cursor).map_err(|e| format!("read class attr name_index: {e}"))?;
        let attr_len =
            read_u32(&mut cursor).map_err(|e| format!("read class attr length: {e}"))?;
        let attr_name = strings.get(attr_name_idx).unwrap_or("");

        if attr_name == "RuntimeVisibleAnnotations" {
            let (found_stereotype, import_classes, has_config_props) =
                parse_class_annotations(&mut cursor, &strings)?;

            imported_classes.extend(import_classes);

            if found_stereotype {
                let fqcn = class_name.replace('/', ".");
                beans.push(DiscoveredBean {
                    bean_name: to_bean_name(&simple_class_name),
                    bean_type: simple_class_name.clone(),
                    bean_type_fqcn: fqcn,
                    discovery_method: DiscoveryMethod::ComponentStereotype,
                });
            }

            if has_config_props {
                let fqcn = class_name.replace('/', ".");
                beans.push(DiscoveredBean {
                    bean_name: to_bean_name(&simple_class_name),
                    bean_type: simple_class_name.clone(),
                    bean_type_fqcn: fqcn,
                    discovery_method: DiscoveryMethod::ConfigurationProperties,
                });
            }
        } else {
            cursor
                .seek(SeekFrom::Current(attr_len as i64))
                .map_err(|e| format!("skip class attr data: {e}"))?;
        }
    }

    Ok(ParsedClass {
        class_name,
        beans,
        imported_classes,
        interfaces,
    })
}

// ---------------------------------------------------------------------------
// Constant pool parsing
// ---------------------------------------------------------------------------

/// Parse the constant pool entries and build a `StringTable` of all UTF-8
/// constants (indexed by pool entry number).
/// Pre-resolved pool: each index maps to its UTF-8 string value (after following CONSTANT_Class indirection).
struct ResolvedPool {
    strings: Vec<String>, // 1-indexed
}

fn parse_constant_pool(cursor: &mut Cursor<&[u8]>, pool_count: u16) -> Result<ResolvedPool, String> {
    // Phase 1: read raw entries
    let mut raw: Vec<Option<Raw>> = vec![None]; // index 0 = null
    let mut i: u16 = 1;
    while i < pool_count {
        let mut tb = [0u8; 1];
        cursor.read_exact(&mut tb).map_err(|e| format!("tag at {i}: {e}"))?;
        let tag = tb[0];
        let data = match tag {
            1 => { let len = read_u16(cursor).map_err(|e| format!("utf8 len at {i}: {e}"))?; let mut b = vec![0u8; len as usize]; cursor.read_exact(&mut b).map_err(|e| format!("utf8 data at {i}: {e}"))?; b }
            3|4 => { cursor.seek(SeekFrom::Current(4)).map_err(|e| format!("skip int/float at {i}: {e}"))?; vec![] }
            5|6 => { cursor.seek(SeekFrom::Current(8)).map_err(|e| format!("skip long/double at {i}: {e}"))?; raw.push(None); i += 1; vec![] }
            15 => { let mut b = vec![0u8; 3]; cursor.read_exact(&mut b).map_err(|e| format!("MethodHandle at {i}: {e}"))?; b }
            7|8|16|19|20 => { let mut b = vec![0u8; 2]; cursor.read_exact(&mut b).map_err(|e| format!("2-byte at {i}: {e}"))?; b }
            9|10|11|12|17|18 => { let mut b = vec![0u8; 4]; cursor.read_exact(&mut b).map_err(|e| format!("4-byte at {i}: {e}"))?; b }
            _ => return Err(format!("unknown tag {tag} at {i}"))
        };
        raw.push(Some(Raw { tag, data }));
        i += 1;
    }

    // Phase 2: resolve each index to its UTF-8 string
    let mut strings = vec![String::new()]; // index 0
    for i in 1..raw.len() {
        let s = match &raw[i] {
            None => String::new(),
            Some(r) => match r.tag {
                1 => String::from_utf8_lossy(&r.data).to_string(),
                7 => {
                    if r.data.len() >= 2 {
                        let ni = u16::from_be_bytes([r.data[0], r.data[1]]) as usize;
                        if ni < raw.len() {
                            resolve_raw_to_utf8(&raw, ni)
                        } else { String::new() }
                    } else { String::new() }
                }
                _ => String::new()
            }
        };
        strings.push(s);
    }

    Ok(ResolvedPool { strings })
}

struct Raw { tag: u8, data: Vec<u8> }
fn resolve_raw_to_utf8(raw: &[Option<Raw>], idx: usize) -> String {
    match &raw[idx] {
        None => String::new(),
        Some(r) => match r.tag {
            1 => String::from_utf8_lossy(&r.data).to_string(),
            7 => {
                if r.data.len() >= 2 {
                    let ni = u16::from_be_bytes([r.data[0], r.data[1]]) as usize;
                    resolve_raw_to_utf8(raw, ni)
                } else { String::new() }
            }
            _ => String::new()
        }
    }
}

impl ResolvedPool {
    fn get(&self, idx: u16) -> Option<&str> {
        self.strings.get(idx as usize).map(|s| s.as_str()).filter(|s| !s.is_empty())
    }
}

// ---------------------------------------------------------------------------
// Field / method skipping
// ---------------------------------------------------------------------------

/// Skip over a field_info or method_info structure (JVMS 4.5 / 4.6).
///
/// The caller is expected to have already consumed the access_flags, name_index
/// and descriptor_index for methods (as those are needed for inspection).
/// For fields we consume those here before skipping attributes.
fn skip_field_or_method(cursor: &mut Cursor<&[u8]>) -> Result<(), String> {
    // access_flags, name_index, descriptor_index
    read_u16(cursor).map_err(|e| format!("skip field/method access_flags: {e}"))?;
    read_u16(cursor).map_err(|e| format!("skip field/method name_index: {e}"))?;
    read_u16(cursor).map_err(|e| format!("skip field/method descriptor_index: {e}"))?;

    let attr_count =
        read_u16(cursor).map_err(|e| format!("skip field/method attributes_count: {e}"))?;
    for _ in 0..attr_count {
        read_u16(cursor).map_err(|e| format!("skip attr name_index: {e}"))?;
        let len = read_u32(cursor).map_err(|e| format!("skip attr length: {e}"))?;
        cursor
            .seek(SeekFrom::Current(len as i64))
            .map_err(|e| format!("skip attr data: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Class-level annotation parsing
// ---------------------------------------------------------------------------

/// Parse the `RuntimeVisibleAnnotations` attribute at the CLASS level.
///
/// Returns:
/// - `found_stereotype`: true if a Spring stereotype annotation was found
/// - `imported_classes`: class names from @Import
/// - `has_config_properties`: true if @ConfigurationProperties was found
fn parse_class_annotations(
    cursor: &mut Cursor<&[u8]>,
    strings: &ResolvedPool,
) -> Result<(bool, Vec<String>, bool), String> {
    let num_annotations =
        read_u16(cursor).map_err(|e| format!("class annotation num_annotations: {e}"))?;
    let mut found_stereotype = false;
    let mut imported = Vec::new();
    let mut has_config_props = false;

    /// Descriptors for Spring stereotype annotations (JVMS internal form).
    const STEREOTYPE_DESCRIPTORS: &[&str] = &[
        "Lorg/springframework/stereotype/Component;",
        "Lorg/springframework/stereotype/Service;",
        "Lorg/springframework/stereotype/Repository;",
        "Lorg/springframework/stereotype/Controller;",
        "Lorg/springframework/web/bind/annotation/RestController;",
        "Lorg/springframework/context/annotation/Configuration;",
        "Lorg/springframework/boot/autoconfigure/AutoConfiguration;",
    ];

    for _ in 0..num_annotations {
        let type_idx = read_u16(cursor).map_err(|e| format!("class ann type_index: {e}"))?;
        let desc = strings.get(type_idx).unwrap_or("").to_string();

        if STEREOTYPE_DESCRIPTORS.iter().any(|&s| desc == s) {
            found_stereotype = true;
        }

        if desc.contains("ConfigurationProperties") {
            has_config_props = true;
        }

        let num_pairs = read_u16(cursor)
            .map_err(|e| format!("class ann element_value_pairs: {e}"))?;

        if desc == "Lorg/springframework/context/annotation/Import;" {
            for _ in 0..num_pairs {
                let elem_name_idx =
                    read_u16(cursor).map_err(|e| format!("import element_name_index: {e}"))?;
                let elem_name = strings.get(elem_name_idx).unwrap_or("");
                let elem_value = read_element_value(cursor, strings)?;

                if elem_name == "value" {
                    match elem_value {
                        ElemValue::Array(classes) => {
                            for c in classes {
                                if let ElemValue::EnumOrClass(s) = c {
                                    if let Some(class_name) =
                                        s.strip_suffix(".class").map(|s| s.replace('/', "."))
                                    {
                                        imported.push(class_name);
                                    }
                                }
                            }
                        }
                        ElemValue::EnumOrClass(s) => {
                            if let Some(class_name) =
                                s.strip_suffix(".class").map(|s| s.replace('/', "."))
                            {
                                imported.push(class_name);
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else {
            // Skip element_value pairs for non-@Import annotations.
            for _ in 0..num_pairs {
                read_u16(cursor)
                    .map_err(|e| format!("skip ann element_name_index: {e}"))?;
                skip_element_value(cursor, strings)?;
            }
        }
    }

    Ok((found_stereotype, imported, has_config_props))
}

// ---------------------------------------------------------------------------
// Method-level annotation parsing (@Bean)
// ---------------------------------------------------------------------------

/// Parse the `RuntimeVisibleAnnotations` attribute at the METHOD level.
///
/// Returns:
/// - `has_bean`: true if a `@Bean` annotation was found
/// - `explicit_name`: the value of the `name` or `value` attribute on @Bean,
///   if present.
fn parse_method_annotations(
    cursor: &mut Cursor<&[u8]>,
    strings: &ResolvedPool,
    attr_len: u32,
) -> Result<(bool, Option<String>), String> {
    let end_pos = cursor.position() + attr_len as u64;
    let num_annotations =
        read_u16(cursor).map_err(|e| format!("method ann num_annotations: {e}"))?;
    let mut has_bean = false;
    let mut bean_name: Option<String> = None;

    for _ in 0..num_annotations {
        if cursor.position() >= end_pos {
            break;
        }
        let type_idx = read_u16(cursor).map_err(|e| format!("method ann type_index: {e}"))?;
        let desc = strings.get(type_idx).unwrap_or("").to_string();
        let num_pairs = read_u16(cursor)
            .map_err(|e| format!("method ann element_value_pairs: {e}"))?;

        if desc == "Lorg/springframework/context/annotation/Bean;" {
            has_bean = true;
            for _ in 0..num_pairs {
                let elem_name_idx =
                    read_u16(cursor).map_err(|e| format!("bean element_name_index: {e}"))?;
                let elem_name = strings.get(elem_name_idx).unwrap_or("");
                let elem_value = read_element_value(cursor, strings)?;

                if elem_name == "name" || elem_name == "value" {
                    match elem_value {
                        ElemValue::String(s) => {
                            bean_name = Some(s);
                        }
                        ElemValue::Array(arr) => {
                            // `@Bean(name = {"foo", "bar"})` — use the first name.
                            if let Some(ElemValue::String(s)) = arr.first() {
                                bean_name = Some(s.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else {
            for _ in 0..num_pairs {
                read_u16(cursor)
                    .map_err(|e| format!("skip method ann element_name_index: {e}"))?;
                skip_element_value(cursor, strings)?;
            }
        }
    }

    // Seek to the exact end of the attribute data (in case parsing terminated
    // early due to an unexpected structure).
    cursor
        .seek(SeekFrom::Start(end_pos))
        .map_err(|e| format!("seek to end of method annotations: {e}"))?;

    Ok((has_bean, bean_name))
}

// ---------------------------------------------------------------------------
// Annotation element_value parser (JVMS 4.7.16.1)
// ---------------------------------------------------------------------------

/// Lightweight representation of a JVM `element_value` union.
enum ElemValue {
    /// A string constant (`'s'` tag).
    String(String),
    /// An enum constant (`'e'` tag) or class literal (`'c'` tag).
    EnumOrClass(String),
    /// An array of element values (`'['` tag).
    Array(Vec<ElemValue>),
    /// Any other tag (primitives, nested annotations) — not needed for parsing.
    Null,
}

/// Read a single `element_value` from the cursor.
///
/// See JVMS 4.7.16.1 for the grammar:
/// ```text
/// element_value {
///     u1 tag;
///     union {
///         B, C, D, F, I, J, S, Z: u2 const_value_index
///         s:                    u2 const_value_index
///         e:                    u2 type_name_index  u2 const_name_index
///         c:                    u2 class_info_index
///         @:                    annotation
///         [:                    u2 num_values  element_value values[num_values]
///     }
/// }
/// ```
fn read_element_value(
    cursor: &mut Cursor<&[u8]>,
    strings: &ResolvedPool,
) -> Result<ElemValue, String> {
    let mut tag_buf = [0u8; 1];
    cursor
        .read_exact(&mut tag_buf)
        .map_err(|e| format!("read element_value tag: {e}"))?;
    let tag = tag_buf[0] as char;

    match tag {
        // String constant
        's' => {
            let idx = read_u16(cursor).map_err(|e| format!("str const_value_index: {e}"))?;
            Ok(ElemValue::String(
                strings.get(idx).unwrap_or("").to_string(),
            ))
        }
        // Enum constant: type_name_index + const_name_index
        'e' => {
            read_u16(cursor).map_err(|e| format!("enum type_name_index: {e}"))?; // type_name_index
            let name_idx =
                read_u16(cursor).map_err(|e| format!("enum const_name_index: {e}"))?;
            Ok(ElemValue::EnumOrClass(
                strings.get(name_idx).unwrap_or("").to_string(),
            ))
        }
        // Class literal: class_info_index
        'c' => {
            let idx = read_u16(cursor).map_err(|e| format!("class class_info_index: {e}"))?;
            Ok(ElemValue::EnumOrClass(
                strings.get(idx).unwrap_or("").to_string(),
            ))
        }
        // Array
        '[' => {
            let count = read_u16(cursor).map_err(|e| format!("array num_values: {e}"))?;
            let mut arr = Vec::with_capacity(count as usize);
            for _ in 0..count {
                arr.push(read_element_value(cursor, strings)?);
            }
            Ok(ElemValue::Array(arr))
        }
        // Nested annotation: type_index + num_pairs + pairs
        '@' => {
            read_u16(cursor).map_err(|e| format!("nested ann type_index: {e}"))?;
            let num_pairs =
                read_u16(cursor).map_err(|e| format!("nested ann num_pairs: {e}"))?;
            for _ in 0..num_pairs {
                read_u16(cursor)
                    .map_err(|e| format!("nested ann element_name_index: {e}"))?;
                skip_element_value(cursor, strings)?;
            }
            Ok(ElemValue::Null)
        }
        // Primitives (B, C, D, F, I, J, S, Z) or unrecognised: 2-byte
        // const_value_index — skip.
        _ => {
            read_u16(cursor)
                .map_err(|e| format!("primitive const_value_index (tag={tag}): {e}"))?;
            Ok(ElemValue::Null)
        }
    }
}

/// Skip over an `element_value` (discard the result).
fn skip_element_value(
    cursor: &mut Cursor<&[u8]>,
    strings: &ResolvedPool,
) -> Result<(), String> {
    read_element_value(cursor, strings)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Method descriptor return-type extraction
// ---------------------------------------------------------------------------

/// Extract the return type from a JVM method descriptor.
///
/// Examples:
/// - `()Ljavax/sql/DataSource;` → `"DataSource"`
/// - `(Ljava/lang/String;)I` → `"int"`
/// - `()[Ljava/lang/String;` → `"String[]"`
/// Return the simple class name from a method descriptor.
fn parse_return_type(descriptor: &str) -> String {
    parse_return_type_full(descriptor).0
}

/// Return (simple_name, fully_qualified_name) from a method descriptor.
/// e.g., `()Ljavax/sql/DataSource;` → ("DataSource", "javax.sql.DataSource")
fn parse_return_type_full(descriptor: &str) -> (String, String) {
    let ret = match descriptor.rfind(')') {
        Some(pos) => &descriptor[pos + 1..],
        None => return ("Unknown".to_string(), String::new()),
    };

    match ret {
        "V" => ("void".to_string(), String::new()),
        "Z" => ("boolean".to_string(), String::new()),
        "B" => ("byte".to_string(), String::new()),
        "C" => ("char".to_string(), String::new()),
        "S" => ("short".to_string(), String::new()),
        "I" => ("int".to_string(), String::new()),
        "J" => ("long".to_string(), String::new()),
        "F" => ("float".to_string(), String::new()),
        "D" => ("double".to_string(), String::new()),
        _ if ret.starts_with('L') => {
            let class_path = &ret[1..ret.len() - 1]; // javax/sql/DataSource
            let fqcn = class_path.replace('/', ".");   // javax.sql.DataSource
            let simple = class_path.rsplit('/').next().unwrap_or(class_path).to_string();
            (simple, fqcn)
        }
        _ if ret.starts_with('[') => {
            let component = ret.trim_start_matches('[');
            if component.starts_with('L') {
                let c = &component[1..component.len() - 1];
                let fqcn = format!("{}[]", c.replace('/', "."));
                let simple = format!("{}[]", c.rsplit('/').next().unwrap_or(c));
                (simple, fqcn)
            } else {
                (primitive_array_name(component), String::new())
            }
        }
        _ => (ret.to_string(), String::new()),
    }
}

/// Map a primitive array component to its readable name.
fn primitive_array_name(component: &str) -> String {
    match component {
        "Z" => "boolean[]",
        "B" => "byte[]",
        "C" => "char[]",
        "S" => "short[]",
        "I" => "int[]",
        "J" => "long[]",
        "F" => "float[]",
        "D" => "double[]",
        _ => "Array",
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Spring bean name conversion
// ---------------------------------------------------------------------------

/// Convert a class name to a Spring bean name by lowercasing the first
/// character (following `AnnotationBeanNameGenerator` conventions).
///
/// Examples:
/// - `"DataSource"` → `"dataSource"`
/// - `"OAuth2AuthorizationService"` → `"oAuth2AuthorizationService"`
fn to_bean_name(class_name: &str) -> String {
    let mut chars: Vec<char> = class_name.chars().collect();
    if let Some(first) = chars.first_mut() {
        *first = first.to_lowercase().next().unwrap_or(*first);
    }
    chars.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_return_type_primitives() {
        assert_eq!(parse_return_type("()V"), "void");
        assert_eq!(parse_return_type("()Z"), "boolean");
        assert_eq!(parse_return_type("()B"), "byte");
        assert_eq!(parse_return_type("()C"), "char");
        assert_eq!(parse_return_type("()S"), "short");
        assert_eq!(parse_return_type("()I"), "int");
        assert_eq!(parse_return_type("()J"), "long");
        assert_eq!(parse_return_type("()F"), "float");
        assert_eq!(parse_return_type("()D"), "double");
    }

    #[test]
    fn test_parse_return_type_objects() {
        assert_eq!(
            parse_return_type("()Ljavax/sql/DataSource;"),
            "DataSource"
        );
        assert_eq!(parse_return_type("()Ljava/lang/String;"), "String");
        assert_eq!(
            parse_return_type("(Ljava/lang/String;)I"),
            "int"
        );
    }

    #[test]
    fn test_parse_return_type_arrays() {
        assert_eq!(
            parse_return_type("()[Ljava/lang/String;"),
            "String[]"
        );
        assert_eq!(parse_return_type("()[I"), "int[]");
        assert_eq!(parse_return_type("()[B"), "byte[]");
    }

    #[test]
    fn test_bean_name_conversion() {
        assert_eq!(to_bean_name("DataSource"), "dataSource");
        assert_eq!(
            to_bean_name("OAuth2AuthorizationService"),
            "oAuth2AuthorizationService"
        );
        assert_eq!(to_bean_name("MyClass"), "myClass");
        // Single-char names
        assert_eq!(to_bean_name("A"), "a");
    }

    #[test]
    fn test_invalid_class_file() {
        let result = parse_class(b"not a class file");
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_class_file_header() {
        // Valid magic + version, but pool_count=0 and truncated — should
        // fail cleanly on the first read past the constant pool.
        // CAFEBABE 0000 003C 0000
        let data = vec![0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x3C, 0x00, 0x00];
        let result = parse_class(&data);
        assert!(result.is_err(), "Expected error for truncated class file");
    }

    #[test]
    fn test_bad_magic_number() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00];
        let result = parse_class(&data);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("not a valid class file"),
            "Expected magic number error"
        );
    }
}
