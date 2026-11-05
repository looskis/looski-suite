use super::{Filter, RecordField, valid_value};
use crate::{application::OperationError as Error, contacts::ListContacts};
use serde_json::Value;
use sqlx::{Postgres, QueryBuilder, types::Json};
use uuid::Uuid;

pub fn validate_filter(filter: &Filter, fields: &[RecordField]) -> Result<(), Error> {
    fn walk(
        node: &Filter,
        fields: &[RecordField],
        depth: usize,
        count: &mut usize,
    ) -> Result<(), Error> {
        *count += 1;
        if depth > 5 || *count > 60 {
            return Err(Error::Invalid(
                "Filters support at most 60 rules and 5 nested groups",
            ));
        }
        match node {
            Filter::Group { operator, rules } => {
                if !["and", "or"].contains(&operator.as_str()) || rules.len() > 20 {
                    return Err(Error::Invalid(
                        "Groups require AND or OR and at most 20 children",
                    ));
                }
                for rule in rules {
                    walk(rule, fields, depth + 1, count)?;
                }
            }
            Filter::Condition {
                field,
                operator,
                value,
            } => {
                let f = fields
                    .iter()
                    .find(|f| f.key == *field)
                    .ok_or(Error::Invalid("Unknown filter field"))?;
                if ["is_empty", "is_not_empty"].contains(&operator.as_str()) {
                    return Ok(());
                }
                let text = ["text", "email", "url"].contains(&f.field_type.as_str());
                if !["eq", "neq", "in", "not_in"].contains(&operator.as_str())
                    && !(text
                        && ["contains", "not_contains", "starts_with"].contains(&operator.as_str()))
                    && !(["number", "date"].contains(&f.field_type.as_str())
                        && ["gt", "gte", "lt", "lte"].contains(&operator.as_str()))
                {
                    return Err(Error::Invalid(
                        "Operator is not supported for this field type",
                    ));
                }
                let values = if ["in", "not_in"].contains(&operator.as_str()) {
                    let a = value
                        .as_array()
                        .ok_or(Error::Invalid("Membership filters require an array"))?;
                    if a.is_empty() || a.len() > 50 {
                        return Err(Error::Invalid("Use 1–50 filter values"));
                    }
                    a.iter().collect::<Vec<_>>()
                } else {
                    vec![value]
                };
                for v in values {
                    let valid = if text
                        && ["contains", "not_contains", "starts_with"].contains(&operator.as_str())
                    {
                        v.as_str()
                            .is_some_and(|s| s.chars().count() <= 2000 && !s.contains('\0'))
                    } else {
                        valid_value(f, v)
                    };
                    if v.is_null() || !valid {
                        return Err(Error::Invalid("Filter value does not match the field type"));
                    }
                }
            }
        }
        Ok(())
    }
    walk(filter, fields, 0, &mut 0)
}
// Every field is selected from the registry; all paths and values are bound.
fn expression(q: &mut QueryBuilder<Postgres>, kind: &str, alias: &str, field: &RecordField) {
    let key = field.key.as_str();
    let column = match key {
        "name" => Some(if kind == "contacts" {
            "display_name"
        } else {
            "name"
        }),
        "notes" => Some("notes"),
        "website_url" => Some("website_url"),
        "created_at" => Some("created_at"),
        "updated_at" => Some("updated_at"),
        _ => None,
    };
    if let Some(column) = column {
        if ["created_at", "updated_at"].contains(&column) {
            q.push("to_jsonb(to_char(")
                .push(alias)
                .push(".")
                .push(column)
                .push(" AT TIME ZONE 'UTC','YYYY-MM-DD'))");
        } else {
            q.push("to_jsonb(")
                .push(alias)
                .push(".")
                .push(column)
                .push(")");
        }
    } else if kind == "contacts"
        && ["email", "phone", "primary_email", "secondary_email"].contains(&key)
    {
        let id_kind = if key == "phone" { "phone" } else { "email" };
        if key == "email" || key == "phone" {
            q.push("(SELECT to_jsonb(string_agg(ci.original_value, E'\\n' ORDER BY ci.position,i.normalized_value))");
        } else {
            q.push("(SELECT to_jsonb(ci.original_value)");
        }
        q.push(" FROM contacts.contact_identifiers ci JOIN contacts.identifiers i ON (i.workspace_id,i.id)=(ci.workspace_id,ci.identifier_id) WHERE ci.workspace_id=").push(alias).push(".workspace_id AND ci.contact_id=").push(alias).push(".id AND ci.ended_at IS NULL AND i.kind=").push_bind(id_kind.to_owned());
        if key == "primary_email" || key == "secondary_email" {
            q.push(" ORDER BY ci.position,i.normalized_value LIMIT 1 OFFSET ")
                .push(if key == "secondary_email" { "1" } else { "0" });
        }
        q.push(")");
    } else {
        q.push("(")
            .push(alias)
            .push(".properties #> ")
            .push_bind(key.split('.').map(str::to_owned).collect::<Vec<_>>())
            .push("::text[])");
    }
}
fn comparable(q: &mut QueryBuilder<Postgres>, kind: &str, alias: &str, f: &RecordField) {
    if ["text", "email", "url", "select"].contains(&f.field_type.as_str()) {
        q.push("to_jsonb(lower(");
        expression(q, kind, alias, f);
        q.push(" #>> '{}'))");
    } else {
        q.push("NULLIF(");
        expression(q, kind, alias, f);
        q.push(", 'null'::jsonb)");
    }
}
fn normalized(f: &RecordField, v: &Value) -> Value {
    if ["text", "email", "url", "select"].contains(&f.field_type.as_str()) {
        Value::String(v.as_str().unwrap_or("").to_lowercase())
    } else {
        v.clone()
    }
}
pub fn append_filter(
    q: &mut QueryBuilder<Postgres>,
    kind: &str,
    fields: &[RecordField],
    filter: &Filter,
) {
    q.push("(");
    match filter {
        Filter::Group { operator, rules } => {
            if rules.is_empty() {
                q.push(if operator == "and" { "TRUE" } else { "FALSE" });
            }
            for (i, rule) in rules.iter().enumerate() {
                if i > 0 {
                    q.push(if operator == "and" { " AND " } else { " OR " });
                }
                append_filter(q, kind, fields, rule);
            }
        }
        Filter::Condition {
            field,
            operator,
            value,
        } => {
            let f = fields
                .iter()
                .find(|f| f.key == *field)
                .expect("validated field");
            match operator.as_str() {
                "is_empty" | "is_not_empty" => {
                    q.push(if operator == "is_not_empty" {
                        "NOT "
                    } else {
                        ""
                    })
                    .push("(");
                    expression(q, kind, "r", f);
                    q.push(" IS NULL OR ");
                    expression(q, kind, "r", f);
                    q.push(" IN ('null'::jsonb,'\"\"'::jsonb))");
                }
                "contains" | "not_contains" | "starts_with" => {
                    if operator == "starts_with" {
                        q.push("starts_with(lower(");
                        expression(q, kind, "r", f);
                        q.push(" #>> '{}'),lower(")
                            .push_bind(value.as_str().unwrap().to_owned())
                            .push("))");
                    } else {
                        q.push("strpos(lower(");
                        expression(q, kind, "r", f);
                        q.push(" #>> '{}'),lower(")
                            .push_bind(value.as_str().unwrap().to_owned())
                            .push(if operator == "contains" {
                                "))>0"
                            } else {
                                "))=0"
                            });
                    }
                }
                "in" | "not_in" => {
                    comparable(q, kind, "r", f);
                    q.push(if operator == "in" {
                        " IN ("
                    } else {
                        " NOT IN ("
                    });
                    for (i, v) in value.as_array().unwrap().iter().enumerate() {
                        if i > 0 {
                            q.push(",");
                        }
                        q.push_bind(Json(normalized(f, v)));
                    }
                    q.push(")");
                }
                _ => {
                    comparable(q, kind, "r", f);
                    q.push(match operator.as_str() {
                        "eq" => " = ",
                        "neq" => " <> ",
                        "gt" => " > ",
                        "gte" => " >= ",
                        "lt" => " < ",
                        "lte" => " <= ",
                        _ => unreachable!(),
                    })
                    .push_bind(Json(normalized(f, value)));
                }
            }
        }
    }
    q.push(")");
}
pub fn append_sort(
    q: &mut QueryBuilder<Postgres>,
    kind: &str,
    fields: &[RecordField],
    input: &ListContacts,
    workspace: Uuid,
) -> Result<(), Error> {
    if input.sort.is_none() {
        if let Some(after) = input.after {
            q.push(" AND (r.created_at,r.id) > (SELECT created_at,id FROM ")
                .push(if kind == "contacts" {
                    "contacts.contacts"
                } else {
                    "contacts.organizations"
                })
                .push(" WHERE workspace_id=")
                .push_bind(workspace)
                .push(" AND id=")
                .push_bind(after)
                .push(")");
        }
        q.push(" ORDER BY r.created_at,r.id");
        return Ok(());
    }
    let f = fields
        .iter()
        .find(|f| f.key == input.sort.as_deref().unwrap_or("name"))
        .ok_or(Error::Invalid("Unknown sort field"))?;
    let descending = match input.direction.as_deref().unwrap_or("asc") {
        "asc" => false,
        "desc" => true,
        _ => return Err(Error::Invalid("Sort direction must be asc or desc")),
    };
    let table = if kind == "contacts" {
        "contacts.contacts"
    } else {
        "contacts.organizations"
    };
    if let Some(after) = input.after {
        let cursor = |q: &mut QueryBuilder<Postgres>| {
            q.push("(SELECT ");
            comparable(q, kind, "cursor", f);
            q.push(" FROM ")
                .push(table)
                .push(" cursor WHERE cursor.workspace_id=")
                .push_bind(workspace)
                .push(" AND cursor.id=")
                .push_bind(after)
                .push(")");
        };
        q.push(" AND ((");
        comparable(q, kind, "r", f);
        q.push(" IS NULL AND ");
        cursor(q);
        q.push(" IS NOT NULL) OR (");
        comparable(q, kind, "r", f);
        q.push(if descending { " < " } else { " > " });
        cursor(q);
        q.push(") OR (");
        comparable(q, kind, "r", f);
        q.push(" IS NOT DISTINCT FROM ");
        cursor(q);
        q.push(" AND r.id > ").push_bind(after).push("))");
    }
    q.push(" ORDER BY ");
    comparable(q, kind, "r", f);
    q.push(if descending {
        " DESC NULLS LAST"
    } else {
        " ASC NULLS LAST"
    })
    .push(",r.id ASC");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_fields_and_type_mismatches() {
        let fields = super::super::builtin("organizations").unwrap();
        for raw in [
            r#"{"type":"condition","field":"employee_count","operator":"eq","value":"ten"}"#,
            r#"{"type":"condition","field":"name;drop table x","operator":"eq","value":"a"}"#,
            r#"{"type":"condition","field":"status","operator":"contains","value":"lead"}"#,
        ] {
            let f: Filter = serde_json::from_str(raw).unwrap();
            assert!(validate_filter(&f, &fields).is_err());
        }
    }
    #[test]
    fn nested_filters_bind_values() {
        let fields = super::super::builtin("contacts").unwrap();
        let f:Filter=serde_json::from_str(r#"{"type":"group","operator":"or","rules":[{"type":"condition","field":"name","operator":"contains","value":"' OR TRUE --"},{"type":"condition","field":"job_title","operator":"is_empty"}]}"#).unwrap();
        validate_filter(&f, &fields).unwrap();
        let mut sql = QueryBuilder::<Postgres>::new("");
        append_filter(&mut sql, "contacts", &fields, &f);
        assert!(!sql.sql().as_str().contains("OR TRUE"));
        assert!(sql.sql().as_str().contains(" OR "));
    }
    #[test]
    fn limits_depth() {
        let mut f = Filter::Group {
            operator: "and".into(),
            rules: vec![],
        };
        for _ in 0..7 {
            f = Filter::Group {
                operator: "and".into(),
                rules: vec![f],
            };
        }
        assert!(validate_filter(&f, &super::super::builtin("contacts").unwrap()).is_err());
    }
}
