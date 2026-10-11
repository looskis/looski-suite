// SPDX-License-Identifier: AGPL-3.0-only
//! Content type fields, entry value validation and the media byte allow-list.
//! Field keys are the stable identity of a value; labels may change freely.
use crate::application::OperationError as Error;
use crate::rich_text;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;
use uuid::Uuid;

pub const MAX_FIELDS: usize = 40;
const MAX_VALUES_BYTES: usize = 524288;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentField {
    pub key: String,
    pub label: String,
    pub kind: ContentFieldKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub help: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContentFieldKind {
    Text,
    /// Plain source in the rich text subset; never HTML.
    RichText,
    Number,
    Boolean,
    Date,
    Url,
    /// The id of a media item in this workspace.
    Media,
    /// The id of a form in this workspace, embedded when it is accepting answers.
    Form,
}
fn clean(value: &str, limit: usize) -> bool {
    value.chars().count() <= limit && !value.chars().any(char::is_control)
}
pub(super) fn validate_fields(fields: &[ContentField]) -> Result<(), Error> {
    if fields.len() > MAX_FIELDS {
        return Err(Error::Invalid("A content type has at most 40 fields"));
    }
    let mut seen = std::collections::HashSet::new();
    for field in fields {
        let key = field.key.as_bytes();
        if key.is_empty()
            || key.len() > 64
            || !key[0].is_ascii_lowercase()
            || !key
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
            || !seen.insert(field.key.as_str())
        {
            return Err(Error::Invalid(
                "Field keys must be unique, start with a letter and use lowercase letters, digits or _",
            ));
        }
        if field.label.trim().is_empty() || !clean(&field.label, 120) || !clean(&field.help, 500) {
            return Err(Error::Invalid(
                "Field labels must be 1–120 printable characters and help at most 500",
            ));
        }
    }
    Ok(())
}
/// A key keeps its kind so stored drafts and revisions stay meaningful.
pub(super) fn kinds_unchanged(before: &[ContentField], after: &[ContentField]) -> bool {
    after.iter().all(|field| {
        before
            .iter()
            .find(|old| old.key == field.key)
            .is_none_or(|old| old.kind == field.kind)
    })
}
pub(super) fn validate_slug(slug: &str) -> Result<(), Error> {
    let valid = !slug.is_empty()
        && slug.len() <= 120
        && slug.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        });
    if !valid {
        return Err(Error::Invalid(
            "Addresses use lowercase letters, digits and single hyphens, up to 120 characters",
        ));
    }
    Ok(())
}
pub(super) fn validate_title(title: &str) -> Result<(), Error> {
    if title.trim().is_empty() || !clean(title, 200) {
        return Err(Error::Invalid("Titles must be 1–200 printable characters"));
    }
    Ok(())
}
pub(super) fn reference(value: &Value) -> Option<Uuid> {
    value.as_str().and_then(|id| Uuid::parse_str(id).ok())
}
fn empty(value: &Value) -> bool {
    value.is_null() || value.as_str().is_some_and(|s| s.trim().is_empty())
}
fn date(value: &str) -> bool {
    value.len() == 10 && chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}
/// Check each supplied value against its field; empty values are removed. Required
/// fields are enforced only when publishing, so an unfinished draft can always be saved.
pub(super) fn validate_values(
    fields: &[ContentField],
    values: &Map<String, Value>,
) -> Result<Map<String, Value>, Error> {
    let mut out = Map::new();
    for (key, value) in values {
        let Some(field) = fields.iter().find(|f| &f.key == key) else {
            return Err(Error::Invalid(
                "A value names a field this type does not have",
            ));
        };
        if empty(value) {
            continue;
        }
        let ok = match field.kind {
            ContentFieldKind::Text => value.as_str().is_some_and(|s| {
                s.chars().count() <= 5000
                    && !s.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
            }),
            ContentFieldKind::RichText => {
                if let Some(source) = value.as_str() {
                    rich_text::validate(source)?;
                    source.chars().count() <= 100000
                        && !source
                            .chars()
                            .any(|c| c.is_control() && c != '\n' && c != '\t')
                } else {
                    false
                }
            }
            ContentFieldKind::Number => value.as_f64().is_some_and(f64::is_finite),
            ContentFieldKind::Boolean => value.is_boolean(),
            ContentFieldKind::Date => value.as_str().is_some_and(date),
            ContentFieldKind::Url => value.as_str().is_some_and(|s| {
                rich_text::safe_href(s) && s.to_ascii_lowercase().starts_with("http")
            }),
            ContentFieldKind::Media | ContentFieldKind::Form => reference(value).is_some(),
        };
        if !ok {
            return Err(Error::Invalid(
                "A value does not fit its field: check its type and length; web addresses must use http or https",
            ));
        }
        out.insert(key.clone(), value.clone());
    }
    // Reserve room for JSONB whitespace expansion, like the column constraint measures.
    if serde_json::to_vec_pretty(&out)
        .expect("values serialize")
        .len()
        > MAX_VALUES_BYTES
    {
        return Err(Error::Invalid("This entry is too long to save"));
    }
    Ok(out)
}
pub(super) fn require_complete(
    fields: &[ContentField],
    values: &Map<String, Value>,
) -> Result<(), Error> {
    if fields
        .iter()
        .any(|f| f.required && f.kind != ContentFieldKind::Boolean && !values.contains_key(&f.key))
    {
        return Err(Error::Conflict(
            "Fill in every required field before publishing",
        ));
    }
    Ok(())
}
/// Keep values whose field still exists with the same kind, reporting the others.
pub(super) fn retained(
    from: &[ContentField],
    to: &[ContentField],
    values: &Map<String, Value>,
) -> (Map<String, Value>, Vec<String>) {
    let mut kept = Map::new();
    let mut dropped = Vec::new();
    for (key, value) in values {
        let before = from.iter().find(|f| &f.key == key).map(|f| f.kind);
        let after = to.iter().find(|f| &f.key == key).map(|f| f.kind);
        if after.is_some() && after == before {
            kept.insert(key.clone(), value.clone());
        } else {
            dropped.push(key.clone());
        }
    }
    (kept, dropped)
}
pub(super) fn media_ids(fields: &[ContentField], values: &Map<String, Value>) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = fields
        .iter()
        .filter(|f| f.kind == ContentFieldKind::Media)
        .filter_map(|f| values.get(&f.key).and_then(reference))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}
/// Only formats browsers cannot execute. SVG and HTML can carry scripts, so they
/// are refused however they are declared; bytes must match the declared format.
pub(super) fn media_type_matches(media_type: &str, bytes: &[u8]) -> bool {
    match media_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(b"\xff\xd8\xff"),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/webp" => bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        "application/pdf" => bytes.starts_with(b"%PDF-"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn field(key: &str, kind: ContentFieldKind, required: bool) -> ContentField {
        ContentField {
            key: key.into(),
            label: key.into(),
            kind,
            required,
            help: String::new(),
        }
    }
    fn map(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }
    #[test]
    fn fields_require_unique_well_formed_keys() {
        assert!(validate_fields(&[field("a_1", ContentFieldKind::Text, false)]).is_ok());
        for key in ["", "A", "1a", "a-b", "a b", &"a".repeat(65)] {
            assert!(validate_fields(&[field(key, ContentFieldKind::Text, false)]).is_err());
        }
        let same = field("a", ContentFieldKind::Text, false);
        assert!(validate_fields(&[same.clone(), same]).is_err());
    }
    #[test]
    fn values_are_checked_against_their_kind() {
        let id = Uuid::new_v4().to_string();
        let good = [
            (ContentFieldKind::Text, json!("hello\nthere")),
            (ContentFieldKind::RichText, json!("# Hi <b>")),
            (ContentFieldKind::Number, json!(1.5)),
            (ContentFieldKind::Boolean, json!(false)),
            (ContentFieldKind::Date, json!("2026-02-28")),
            (ContentFieldKind::Url, json!("https://example.com")),
            (ContentFieldKind::Media, json!(id)),
            (ContentFieldKind::Form, json!(id)),
        ];
        for (kind, value) in good {
            let fields = [field("a", kind, false)];
            assert!(validate_values(&fields, &map(json!({"a":value}))).is_ok());
        }
        let bad = [
            (ContentFieldKind::Text, json!(1)),
            (ContentFieldKind::Text, json!("nul\u{0}")),
            (
                ContentFieldKind::RichText,
                json!("[x](javascript:alert(1))"),
            ),
            (ContentFieldKind::Number, json!("1")),
            (ContentFieldKind::Boolean, json!("true")),
            (ContentFieldKind::Date, json!("2026-02-30")),
            (ContentFieldKind::Url, json!("javascript:alert(1)")),
            (ContentFieldKind::Url, json!("/relative")),
            (ContentFieldKind::Media, json!("not-a-uuid")),
            (ContentFieldKind::Form, json!(7)),
        ];
        for (kind, value) in bad {
            let fields = [field("a", kind, false)];
            assert!(validate_values(&fields, &map(json!({"a":value}))).is_err());
        }
        let fields = [field("a", ContentFieldKind::Text, true)];
        assert!(validate_values(&fields, &map(json!({"other":"x"}))).is_err());
        assert!(
            validate_values(&fields, &map(json!({"a":"  "})))
                .unwrap()
                .is_empty(),
            "Blank values are removed"
        );
    }
    #[test]
    fn required_fields_block_publishing_only() {
        let fields = [field("a", ContentFieldKind::Text, true)];
        assert!(require_complete(&fields, &Map::new()).is_err());
        assert!(require_complete(&fields, &map(json!({"a":"x"}))).is_ok());
    }
    #[test]
    fn restoring_keeps_only_values_whose_kind_survives() {
        let from = [
            field("a", ContentFieldKind::Text, false),
            field("b", ContentFieldKind::Text, false),
            field("c", ContentFieldKind::Text, false),
        ];
        let to = [
            field("a", ContentFieldKind::Text, false),
            field("b", ContentFieldKind::Number, false),
        ];
        let (kept, dropped) = retained(&from, &to, &map(json!({"a":"1","b":"2","c":"3"})));
        assert_eq!(kept, map(json!({"a":"1"})));
        assert_eq!(dropped, ["b", "c"]);
        assert!(!kinds_unchanged(&from, &to));
        assert!(kinds_unchanged(&from, &from[..2]));
    }
    #[test]
    fn slugs_are_url_safe() {
        assert!(validate_slug("a-1-b").is_ok());
        for slug in [
            "",
            "A",
            "a--b",
            "-a",
            "a-",
            "a/b",
            "a b",
            "..",
            &"a".repeat(121),
        ] {
            assert!(validate_slug(slug).is_err(), "{slug}");
        }
    }
    #[test]
    fn media_bytes_must_match_an_allowed_format() {
        assert!(media_type_matches("image/png", b"\x89PNG\r\n\x1a\nrest"));
        assert!(media_type_matches("image/jpeg", b"\xff\xd8\xffrest"));
        assert!(media_type_matches("image/gif", b"GIF89arest"));
        assert!(media_type_matches("image/webp", b"RIFF0000WEBPrest"));
        assert!(media_type_matches("application/pdf", b"%PDF-1.7"));
        assert!(!media_type_matches(
            "image/png",
            b"<script>alert(1)</script>"
        ));
        assert!(!media_type_matches(
            "image/svg+xml",
            b"<svg onload=alert(1)>"
        ));
        assert!(!media_type_matches("text/html", b"<html>"));
    }
}
