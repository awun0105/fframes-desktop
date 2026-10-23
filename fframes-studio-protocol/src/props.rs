//! Props of components a video marks editable, and the `props.json` document that overrides
//! them. Kept apart from timing: a props document changes what frames draw, never the
//! timeline.
use crate::preview::{EditorObjectIdentity, PreviewEnvelope};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

/// The worker accepts `PreviewRequest::SetProps` and `PreviewRequest::PropsInfo`. A worker
/// lists it in Hello only when its video registered props hooks; clients send neither
/// request otherwise.
pub const SET_PROPS_CAPABILITY: &str = "set_props_v1";
pub const PROPS_DOC_VERSION: u32 = 1;
/// Upper bound on a serialized props document, like `MAX_TIMING_DOC_BYTES`: escaped into a
/// `SetPropsRequest` it stays below the 1 MiB control message limit.
pub const MAX_PROPS_DOC_BYTES: usize = 512 * 1024;
pub const MAX_PROPS_OBJECTS: usize = 4096;
pub const MAX_PROPS_FIELDS: usize = 256;
pub const MAX_PROP_NAME_BYTES: usize = 128;

/// Field values of one editable object, keyed by the object's editor identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropsEntry {
    pub identity: EditorObjectIdentity,
    pub values: BTreeMap<String, Value>,
}

/// `props.json`: values that replace the code's values of editable objects. Objects and
/// fields the video does not have are ignored when it renders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropsDocument {
    pub version: u32,
    #[serde(default)]
    pub objects: Vec<PropsEntry>,
}

impl Default for PropsDocument {
    fn default() -> Self {
        Self {
            version: PROPS_DOC_VERSION,
            objects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropsDocError {
    TooLarge { bytes: usize },
    Json(String),
    Version(u32),
    TooManyObjects(usize),
    TooManyFields { object: String, fields: usize },
    InvalidIdentity(String),
    DuplicateObject(String),
    InvalidField { object: String, field: String },
}

impl fmt::Display for PropsDocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { bytes } => write!(
                f,
                "props document is {bytes} bytes, more than {MAX_PROPS_DOC_BYTES}"
            ),
            Self::Json(error) => write!(f, "props document is not valid: {error}"),
            Self::Version(version) => write!(
                f,
                "props document version {version} is not supported (expected {PROPS_DOC_VERSION})"
            ),
            Self::TooManyObjects(count) => write!(
                f,
                "props document has {count} objects, more than {MAX_PROPS_OBJECTS}"
            ),
            Self::TooManyFields { object, fields } => write!(
                f,
                "object {object} has {fields} fields, more than {MAX_PROPS_FIELDS}"
            ),
            Self::InvalidIdentity(object) => write!(
                f,
                "object {object} needs four non-empty identity parts of at most 256 bytes"
            ),
            Self::DuplicateObject(object) => write!(f, "object {object} is listed twice"),
            Self::InvalidField { object, field } => {
                write!(f, "object {object} has an invalid field name {field:?}")
            }
        }
    }
}

impl std::error::Error for PropsDocError {}

/// `scene/component/object/repeat`, for messages.
pub fn identity_label(identity: &EditorObjectIdentity) -> String {
    format!(
        "{}/{}/{}/{}",
        identity.scene_instance_key,
        identity.component_key,
        identity.object_key,
        identity.repeat_key
    )
}

fn valid_identity(identity: &EditorObjectIdentity) -> bool {
    [
        &identity.scene_instance_key,
        &identity.component_key,
        &identity.object_key,
        &identity.repeat_key,
    ]
    .iter()
    .all(|part| !part.is_empty() && part.len() <= 256 && !part.chars().any(char::is_control))
}

fn valid_field(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_PROP_NAME_BYTES
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl PropsDocument {
    /// Parses and validates a whole document; any error rejects all of it.
    pub fn parse(text: &str) -> Result<Self, PropsDocError> {
        if text.len() > MAX_PROPS_DOC_BYTES {
            return Err(PropsDocError::TooLarge { bytes: text.len() });
        }
        let doc: Self =
            serde_json::from_str(text).map_err(|error| PropsDocError::Json(error.to_string()))?;
        doc.validate()?;
        Ok(doc)
    }

    pub fn validate(&self) -> Result<(), PropsDocError> {
        if self.version != PROPS_DOC_VERSION {
            return Err(PropsDocError::Version(self.version));
        }
        if self.objects.len() > MAX_PROPS_OBJECTS {
            return Err(PropsDocError::TooManyObjects(self.objects.len()));
        }
        let mut seen = std::collections::HashSet::new();
        for entry in &self.objects {
            let label = identity_label(&entry.identity);
            if !valid_identity(&entry.identity) {
                return Err(PropsDocError::InvalidIdentity(label));
            }
            if !seen.insert(&entry.identity) {
                return Err(PropsDocError::DuplicateObject(label));
            }
            if entry.values.len() > MAX_PROPS_FIELDS {
                return Err(PropsDocError::TooManyFields {
                    object: label,
                    fields: entry.values.len(),
                });
            }
            if let Some(field) = entry.values.keys().find(|field| !valid_field(field)) {
                return Err(PropsDocError::InvalidField {
                    object: label,
                    field: field.clone(),
                });
            }
        }
        Ok(())
    }

    /// The document as written to `props.json`: pretty JSON with a trailing newline.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("props documents serialize");
        text.push('\n');
        text
    }

    pub fn values(&self, identity: &EditorObjectIdentity) -> Option<&BTreeMap<String, Value>> {
        self.objects
            .iter()
            .find(|entry| &entry.identity == identity)
            .map(|entry| &entry.values)
    }

    /// Sets one field; `None` removes it, and an object left without fields is removed.
    pub fn set(&mut self, identity: &EditorObjectIdentity, field: &str, value: Option<Value>) {
        let index = self
            .objects
            .iter()
            .position(|entry| &entry.identity == identity);
        match (index, value) {
            (Some(index), Some(value)) => {
                self.objects[index].values.insert(field.to_owned(), value);
            }
            (Some(index), None) => {
                self.objects[index].values.remove(field);
                if self.objects[index].values.is_empty() {
                    self.objects.remove(index);
                }
            }
            (None, Some(value)) => self.objects.push(PropsEntry {
                identity: identity.clone(),
                values: BTreeMap::from([(field.to_owned(), value)]),
            }),
            (None, None) => {}
        }
    }
}

/// What a field holds, which decides the control Studio shows for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropKind {
    /// A JSON number.
    Number,
    /// A JSON integer.
    Integer,
    /// A JSON string.
    Text,
    /// A `#rrggbb` or `#rrggbbaa` string.
    Color,
    /// A JSON boolean.
    Toggle,
    /// One of `options`, as a string.
    Choice { options: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropField {
    pub name: String,
    pub kind: PropKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// The editable fields of a component type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropsSchema {
    pub component: String,
    pub fields: Vec<PropField>,
}

/// An editable object as it last rendered: its schema, the values its code gave it and
/// the values it drew with after the document's overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropsInfo {
    pub schema: PropsSchema,
    pub code_values: BTreeMap<String, Value>,
    pub values: BTreeMap<String, Value>,
}

/// Replaces the props document of the preview. `props_json` is a [`PropsDocument`] of at
/// most [`MAX_PROPS_DOC_BYTES`] (`None` restores the code's values); the worker rejects the
/// whole document on any error and keeps the previous one and its generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPropsRequest {
    pub envelope: PreviewEnvelope,
    pub props_generation: u64,
    pub props_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropsAppliedResponse {
    pub envelope: PreviewEnvelope,
    pub props_generation: u64,
}

/// Asks for the props of an object of a rendered frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropsInfoRequest {
    pub envelope: PreviewEnvelope,
    pub identity: EditorObjectIdentity,
}

/// `info` is `None` when the object is not editable or has not rendered yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropsInfoResponse {
    pub envelope: PreviewEnvelope,
    pub identity: EditorObjectIdentity,
    pub props_generation: u64,
    pub info: Option<PropsInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn identity(object: &str) -> EditorObjectIdentity {
        EditorObjectIdentity {
            scene_instance_key: "intro".into(),
            component_key: "LowerThird".into(),
            object_key: object.into(),
            repeat_key: "main".into(),
        }
    }

    #[test]
    fn set_adds_replaces_and_removes_fields() {
        let mut doc = PropsDocument::default();
        doc.set(&identity("title"), "name", Some(json!("Ada")));
        doc.set(&identity("title"), "x", Some(json!(60.0)));
        doc.set(&identity("title"), "name", Some(json!("Grace")));
        assert_eq!(doc.values(&identity("title")).unwrap()["name"], "Grace");
        doc.set(&identity("title"), "name", None);
        doc.set(&identity("title"), "x", None);
        assert!(doc.objects.is_empty());
        doc.set(&identity("other"), "x", None);
        assert!(doc.objects.is_empty());
    }

    #[test]
    fn round_trips_through_json() {
        let mut doc = PropsDocument::default();
        doc.set(&identity("title"), "name", Some(json!("Ada")));
        let text = doc.to_json();
        assert!(text.ends_with('\n'));
        assert_eq!(PropsDocument::parse(&text).unwrap(), doc);
    }

    #[test]
    fn rejects_invalid_documents_whole() {
        let reject = |text: &str| PropsDocument::parse(text).unwrap_err();
        assert!(matches!(reject("{"), PropsDocError::Json(_)));
        assert_eq!(
            reject(r#"{"version":2,"objects":[]}"#),
            PropsDocError::Version(2)
        );
        assert!(matches!(
            reject(r#"{"version":1,"objects":[],"extra":1}"#),
            PropsDocError::Json(_)
        ));
        let entry = |object: &str, field: &str| {
            format!(
                r#"{{"identity":{{"scene_instance_key":"intro","component_key":"LowerThird","object_key":"{object}","repeat_key":"main"}},"values":{{"{field}":1}}}}"#
            )
        };
        let doc =
            |entries: &[String]| format!(r#"{{"version":1,"objects":[{}]}}"#, entries.join(","));
        assert!(matches!(
            reject(&doc(&[entry("", "x")])),
            PropsDocError::InvalidIdentity(_)
        ));
        assert!(matches!(
            reject(&doc(&[entry("a", "x"), entry("a", "y")])),
            PropsDocError::DuplicateObject(_)
        ));
        assert!(matches!(
            reject(&doc(&[entry("a", "min-width")])),
            PropsDocError::InvalidField { .. }
        ));
        assert!(PropsDocument::parse(&doc(&[entry("a", "x"), entry("b", "x")])).is_ok());
        let big = " ".repeat(MAX_PROPS_DOC_BYTES + 1);
        assert!(matches!(reject(&big), PropsDocError::TooLarge { .. }));
    }

    #[test]
    fn schema_kinds_serialize_as_tagged_names() {
        let field = PropField {
            name: "align".into(),
            kind: PropKind::Choice {
                options: vec!["left".into(), "right".into()],
            },
            min: None,
            max: None,
            step: None,
            unit: None,
        };
        assert_eq!(
            serde_json::to_value(&field).unwrap(),
            json!({"name": "align", "kind": {"choice": {"options": ["left", "right"]}}})
        );
        assert_eq!(
            serde_json::to_value(PropKind::Number).unwrap(),
            json!("number")
        );
    }
}
