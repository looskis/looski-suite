// SPDX-License-Identifier: AGPL-3.0-only
//! Form definitions and answer validation. Field keys are the stable identity
//! used by submissions and recipe mappings; labels may change freely.
use crate::application::OperationError as Error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

#[derive(Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = FormDefinition)]
pub struct Definition {
    pub fields: Vec<Field>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub help: String,
    #[serde(default)]
    pub options: Vec<FieldOption>,
    /// Shown only when an earlier choice or checkbox answer matches.
    pub show_if: Option<Condition>,
}
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    /// Display text only; it never receives an answer.
    Statement,
    ShortText,
    LongText,
    Email,
    Phone,
    Number,
    Date,
    Choice,
    MultiChoice,
    Checkbox,
}
impl FieldKind {
    fn has_options(self) -> bool {
        matches!(self, Self::Choice | Self::MultiChoice)
    }
    pub(crate) fn is_text(self) -> bool {
        matches!(
            self,
            Self::ShortText | Self::LongText | Self::Email | Self::Phone
        )
    }
}
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldOption {
    pub key: String,
    pub label: String,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub field: String,
    /// Option keys, or `true`/`false` for a checkbox.
    pub equals: Vec<String>,
}

fn valid_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    key.len() <= 40
        && bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
fn printable(text: &str, max: usize) -> bool {
    text.chars().count() <= max && !text.chars().any(|c| c.is_control() && c != '\n')
}

/// Drafts may be incomplete; publishing requires at least one question.
pub fn validate(definition: &Definition, publishing: bool) -> Result<(), Error> {
    if definition.fields.len() > 100 {
        return Err(Error::Invalid("A form has at most 100 fields"));
    }
    // Reserve room for JSONB whitespace expansion, like the column constraint measures.
    if serde_json::to_vec_pretty(definition)
        .expect("definition serializes")
        .len()
        > 262144
    {
        return Err(Error::Invalid(
            "This form is too large; shorten its questions, help text or options",
        ));
    }
    if publishing
        && !definition
            .fields
            .iter()
            .any(|f| f.kind != FieldKind::Statement)
    {
        return Err(Error::Invalid(
            "Add at least one question before publishing",
        ));
    }
    for (index, field) in definition.fields.iter().enumerate() {
        if !valid_key(&field.key) {
            return Err(Error::Invalid(
                "Field keys must start with a lowercase letter and use at most 40 lowercase letters, digits or underscores",
            ));
        }
        if definition.fields[..index]
            .iter()
            .any(|f| f.key == field.key)
        {
            return Err(Error::Invalid("Field keys must be unique within a form"));
        }
        if field.label.trim().is_empty()
            || !printable(&field.label, 500)
            || !printable(&field.help, 1000)
        {
            return Err(Error::Invalid(
                "Field labels must be 1–500 characters and help at most 1000",
            ));
        }
        if field.kind == FieldKind::Statement && field.required {
            return Err(Error::Invalid("A statement cannot be required"));
        }
        if field.kind.has_options() {
            if field.options.is_empty() || field.options.len() > 50 {
                return Err(Error::Invalid("Choice questions need 1–50 options"));
            }
            for (i, option) in field.options.iter().enumerate() {
                if !valid_key(&option.key)
                    || option.label.trim().is_empty()
                    || !printable(&option.label, 200)
                {
                    return Err(Error::Invalid(
                        "Options need a valid key and a 1–200 character label",
                    ));
                }
                if field.options[..i].iter().any(|o| o.key == option.key) {
                    return Err(Error::Invalid(
                        "Option keys must be unique within a question",
                    ));
                }
            }
        } else if !field.options.is_empty() {
            return Err(Error::Invalid("Only choice questions have options"));
        }
        if let Some(condition) = &field.show_if {
            // Only earlier questions can control visibility, so conditions cannot cycle.
            let Some(source) = definition.fields[..index]
                .iter()
                .find(|f| f.key == condition.field)
            else {
                return Err(Error::Invalid(
                    "A condition must refer to an earlier question",
                ));
            };
            let allowed: Vec<&str> = match source.kind {
                FieldKind::Choice | FieldKind::MultiChoice => {
                    source.options.iter().map(|o| o.key.as_str()).collect()
                }
                FieldKind::Checkbox => vec!["true", "false"],
                _ => {
                    return Err(Error::Invalid(
                        "Conditions can only use choice or checkbox answers",
                    ));
                }
            };
            if condition.equals.is_empty()
                || condition.equals.len() > 50
                || condition
                    .equals
                    .iter()
                    .any(|v| !allowed.contains(&v.as_str()))
            {
                return Err(Error::Invalid(
                    "A condition must list values offered by its question",
                ));
            }
        }
    }
    Ok(())
}

fn matches(condition: &Condition, answer: Option<&Value>) -> bool {
    match answer {
        Some(Value::String(choice)) => condition.equals.contains(choice),
        Some(Value::Array(choices)) => choices
            .iter()
            .filter_map(Value::as_str)
            .any(|c| condition.equals.iter().any(|e| e == c)),
        Some(Value::Bool(checked)) => condition.equals.contains(&checked.to_string()),
        _ => false,
    }
}

/// Keys of the questions visible for these answers, in definition order.
pub fn visible<'a>(definition: &'a Definition, answers: &Map<String, Value>) -> Vec<&'a Field> {
    let mut shown: Vec<&Field> = Vec::new();
    for field in &definition.fields {
        let visible = field.show_if.as_ref().is_none_or(|c| {
            shown.iter().any(|f| f.key == c.field) && matches(c, answers.get(&c.field))
        });
        if visible {
            shown.push(field);
        }
    }
    shown
}

fn valid_date(text: &str) -> bool {
    let p: Vec<_> = text.split('-').collect();
    if p.len() != 3
        || p[0].len() != 4
        || p[1].len() != 2
        || p[2].len() != 2
        || !p.iter().all(|v| v.bytes().all(|b| b.is_ascii_digit()))
    {
        return false;
    }
    let (y, m, d) = (
        p[0].parse::<u32>().unwrap_or(0),
        p[1].parse::<u32>().unwrap_or(0),
        p[2].parse::<u32>().unwrap_or(0),
    );
    let max = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    };
    y > 0 && d > 0 && d <= max
}

fn answer(field: &Field, value: Value) -> Result<Option<Value>, Error> {
    let invalid = || Error::Invalid("An answer does not match its question");
    Ok(match (field.kind, value) {
        (_, Value::Null) => None,
        (FieldKind::Statement, _) => return Err(invalid()),
        (kind, Value::String(text)) if kind.is_text() => {
            let text = text.trim();
            let max = match kind {
                FieldKind::LongText => 10000,
                FieldKind::ShortText => 500,
                FieldKind::Email => 254,
                _ => 40,
            };
            if text.chars().count() > max || !printable(text, max) {
                return Err(Error::Invalid("A text answer is too long or not printable"));
            }
            if text.is_empty() {
                None
            } else {
                if kind == FieldKind::Email
                    && text.split_once('@').is_none_or(|(local, domain)| {
                        local.is_empty()
                            || !domain.contains('.')
                            || domain.contains('@')
                            || text.chars().any(char::is_whitespace)
                    })
                {
                    return Err(Error::Invalid("Enter a valid email address"));
                }
                if kind == FieldKind::Phone
                    && !text
                        .chars()
                        .all(|c| c.is_ascii_digit() || " +-().".contains(c))
                {
                    return Err(Error::Invalid("Enter a valid phone number"));
                }
                Some(Value::String(text.into()))
            }
        }
        (FieldKind::Number, Value::Number(n)) if n.as_f64().is_some_and(f64::is_finite) => {
            Some(Value::Number(n))
        }
        (FieldKind::Date, Value::String(text)) if text.is_empty() => None,
        (FieldKind::Date, Value::String(text)) if valid_date(&text) => Some(Value::String(text)),
        (FieldKind::Choice, Value::String(key)) if field.options.iter().any(|o| o.key == key) => {
            Some(Value::String(key))
        }
        (FieldKind::Choice, Value::String(key)) if key.is_empty() => None,
        (FieldKind::MultiChoice, Value::Array(keys)) => {
            let mut chosen = Vec::new();
            for key in keys {
                let Value::String(key) = key else {
                    return Err(invalid());
                };
                if !field.options.iter().any(|o| o.key == key) || chosen.contains(&key) {
                    return Err(invalid());
                }
                chosen.push(key);
            }
            if chosen.is_empty() {
                None
            } else {
                Some(Value::Array(
                    chosen.into_iter().map(Value::String).collect(),
                ))
            }
        }
        (FieldKind::Checkbox, Value::Bool(checked)) => Some(Value::Bool(checked)),
        _ => return Err(invalid()),
    })
}

/// Validate answers against a published definition, returning the normalized
/// visible answers that become the immutable submission.
pub fn answers(
    definition: &Definition,
    input: &Map<String, Value>,
) -> Result<Map<String, Value>, Error> {
    if input
        .keys()
        .any(|key| !definition.fields.iter().any(|f| &f.key == key))
    {
        return Err(Error::Invalid("Answers refer to an unknown question"));
    }
    // Visibility uses submitted choices, which are validated below before use.
    let shown = visible(definition, input);
    let mut result = Map::new();
    for field in &definition.fields {
        let value = input.get(&field.key).cloned().unwrap_or(Value::Null);
        if !shown.iter().any(|f| f.key == field.key) {
            if !value.is_null() {
                return Err(Error::Invalid("Answer supplied for a hidden question"));
            }
            continue;
        }
        let value = answer(field, value)?;
        let missing = match &value {
            None => true,
            Some(Value::Bool(checked)) => !checked,
            Some(_) => false,
        };
        if field.required && missing {
            return Err(Error::Invalid("Answer every required question"));
        }
        if let Some(value) = value {
            result.insert(field.key.clone(), value);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn form() -> Definition {
        serde_json::from_value(json!({"fields":[
            {"key":"name","label":"Name","kind":"short_text","required":true},
            {"key":"email","label":"Email","kind":"email"},
            {"key":"topic","label":"Topic","kind":"choice","required":true,
             "options":[{"key":"sales","label":"Sales"},{"key":"support","label":"Support"}]},
            {"key":"budget","label":"Budget","kind":"number","required":true,
             "show_if":{"field":"topic","equals":["sales"]}},
            {"key":"consent","label":"I agree","kind":"checkbox","required":true}
        ]}))
        .unwrap()
    }

    #[test]
    fn definitions_reject_forward_conditions_duplicate_keys_and_bad_options() {
        assert!(validate(&form(), true).is_ok());
        let mut forward = form();
        forward.fields[3].show_if = Some(Condition {
            field: "consent".into(),
            equals: vec!["true".into()],
        });
        assert!(validate(&forward, true).is_err());
        let mut duplicate = form();
        duplicate.fields[1].key = "name".into();
        assert!(validate(&duplicate, true).is_err());
        let mut unknown = form();
        unknown.fields[3].show_if.as_mut().unwrap().equals = vec!["other".into()];
        assert!(validate(&unknown, true).is_err());
        assert!(validate(&Definition::default(), false).is_ok());
        assert!(validate(&Definition::default(), true).is_err());
    }

    #[test]
    fn oversized_definitions_are_rejected_before_the_database_constraint() {
        let field = |index: usize| Field {
            key: format!("q{index}"),
            label: "x".repeat(500),
            kind: FieldKind::Choice,
            required: false,
            help: "x".repeat(1000),
            options: (0..50)
                .map(|o| FieldOption {
                    key: format!("o{o}"),
                    label: "x".repeat(200),
                })
                .collect(),
            show_if: None,
        };
        let large = Definition {
            fields: (0..100).map(field).collect(),
        };
        assert!(validate(&large, false).is_err());
    }

    #[test]
    fn conditional_questions_follow_earlier_answers() {
        let sales = json!({"name":" Ada ","topic":"sales","budget":5000,"consent":true});
        let result = answers(&form(), sales.as_object().unwrap()).unwrap();
        assert_eq!(result["name"], "Ada");
        assert_eq!(result["budget"], 5000);
        let missing = json!({"name":"Ada","topic":"sales","consent":true});
        assert!(answers(&form(), missing.as_object().unwrap()).is_err());
        let hidden = json!({"name":"Ada","topic":"support","budget":1,"consent":true});
        assert!(answers(&form(), hidden.as_object().unwrap()).is_err());
        let support = json!({"name":"Ada","topic":"support","consent":true,"email":""});
        let result = answers(&form(), support.as_object().unwrap()).unwrap();
        assert!(!result.contains_key("budget") && !result.contains_key("email"));
    }

    #[test]
    fn answers_are_typed_and_required_checkboxes_must_be_checked() {
        for bad in [
            json!({"name":"Ada","topic":"other","consent":true}),
            json!({"name":"Ada","topic":"support","consent":false}),
            json!({"name":"Ada","topic":"support","consent":true,"email":"not-an-email"}),
            json!({"name":"Ada","topic":"support","consent":true,"extra":"x"}),
            json!({"name":5,"topic":"support","consent":true}),
        ] {
            assert!(answers(&form(), bad.as_object().unwrap()).is_err(), "{bad}");
        }
    }
}
