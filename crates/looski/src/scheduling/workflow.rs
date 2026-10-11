// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use crate::contacts::{ContactInput, IdentifierInput};
use crate::{contacts, crm, jobs};
const PERMISSIONS: &[&str] = &[
    "scheduling.recipes.manage",
    "contacts.read",
    "contacts.create",
    "contacts.intake.create",
    "contacts.intake.read",
    "crm.opportunities.read",
    "crm.opportunities.create",
    "crm.opportunities.participants.manage",
];
pub(super) async fn validate_mapping(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    m: &forms::recipes::Mapping,
    form: Option<&forms::FormVersion>,
) -> Result<(), Error> {
    for p in PERMISSIONS {
        access::require(db, a.identity, a.workspace, p).await?;
    }
    modules::require_crm(db, a).await?;
    if m.name_fields.len() > 3
        || m.title_prefix.trim().is_empty()
        || m.title_prefix.chars().count() > 120
        || m.title_prefix.chars().any(char::is_control)
        || m.email_field.is_none() && m.phone_field.is_none()
        || m.phone_country
            .as_ref()
            .is_some_and(|s| s.len() != 2 || !s.bytes().all(|b| b.is_ascii_uppercase()))
    {
        return Err(Error::Invalid(
            "Choose approved name/email/phone mappings and an inquiry title",
        ));
    }
    let kind = |key: &str| {
        if key == "attendee_name" {
            Some(forms::definition::FieldKind::ShortText)
        } else if key == "attendee_email" {
            Some(forms::definition::FieldKind::Email)
        } else {
            form.and_then(|f| {
                f.definition
                    .fields
                    .iter()
                    .find(|f| f.key == key)
                    .map(|f| f.kind)
            })
        }
    };
    if m.name_fields
        .iter()
        .any(|k| kind(k) != Some(forms::definition::FieldKind::ShortText))
        || m.email_field
            .as_ref()
            .is_some_and(|k| kind(k) != Some(forms::definition::FieldKind::Email))
        || m.phone_field
            .as_ref()
            .is_some_and(|k| kind(k) != Some(forms::definition::FieldKind::Phone))
    {
        return Err(Error::Invalid(
            "Mappings must use attendee fields or matching published intake questions",
        ));
    }
    if form.is_some_and(|f| {
        f.definition
            .fields
            .iter()
            .any(|f| ["attendee_name", "attendee_email"].contains(&f.key.as_str()))
    }) {
        return Err(Error::Invalid(
            "Intake keys attendee_name and attendee_email are reserved for booking fields",
        ));
    }
    if form.is_some() {
        access::require(db, a.identity, a.workspace, "forms.submissions.read").await?;
    }
    crm::assignable_stage(db, a, m.pipeline_id, m.stage_id).await?;
    if let Some(id) = m.owner_membership_id {
        access::require_active_member(db, a.workspace, id).await?;
    }
    Ok(())
}
pub(super) async fn enqueue(
    db: &mut PgConnection,
    t: &TypeRow,
    id: Uuid,
    version: i64,
    first: bool,
) -> Result<(), Error> {
    if first && t.view.settings.crm.is_some() {
        let job = jobs::enqueue_recorded(
            db,
            t.workspace,
            t.user,
            t.member,
            &t.issuer,
            &t.subject,
            &format!("booking:{id}"),
            "scheduling.crm_inquiry",
        )
        .await?;
        sqlx::query(
            "UPDATE scheduling.appointments SET recipe_job_id=$3 WHERE workspace_id=$1 AND id=$2",
        )
        .bind(t.workspace)
        .bind(id)
        .bind(job)
        .execute(&mut *db)
        .await?;
    }
    if t.view.settings.reminder_minutes > 0 {
        let job = jobs::enqueue_recorded(
            db,
            t.workspace,
            t.user,
            t.member,
            &t.issuer,
            &t.subject,
            &format!("booking:{id}:reminder:{version}"),
            "scheduling.reminder",
        )
        .await?;
        sqlx::query("INSERT INTO scheduling.reminders(workspace_id,job_id,appointment_id,version) VALUES($1,$2,$3,$4)").bind(t.workspace).bind(job).bind(id).bind(version).execute(&mut *db).await?;
        sqlx::query("UPDATE suite.jobs SET available_at=(SELECT GREATEST(now(),starts_at-$3*interval '1 minute') FROM scheduling.appointments WHERE workspace_id=$1 AND id=$4) WHERE workspace_id=$1 AND id=$2").bind(t.workspace).bind(job).bind(t.view.settings.reminder_minutes).bind(id).execute(&mut *db).await?;
    }
    Ok(())
}
pub(crate) async fn needs_forms(db: &mut PgConnection, ws: Uuid, job: Uuid) -> Result<bool, Error> {
    Ok(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM scheduling.appointments WHERE workspace_id=$1 AND recipe_job_id=$2 AND form_id IS NOT NULL)").bind(ws).bind(job).fetch_one(db).await?)
}
pub(crate) async fn execute(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    job: Uuid,
) -> Result<(), Error> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM scheduling.appointments WHERE workspace_id=$1 AND recipe_job_id=$2",
    )
    .bind(a.workspace)
    .bind(job)
    .fetch_optional(&mut *db)
    .await?
    .ok_or(Error::NotFound)?;
    let (_, b) = raw(db, a, id, "scheduling.recipes.manage").await?;
    if matches!(b.recipe_state.as_str(), "completed" | "dismissed") {
        return Ok(());
    }
    for p in PERMISSIONS {
        access::require(db, a.identity, a.workspace, p).await?;
    }
    let mapping = sqlx::query_scalar::<_, Json<forms::recipes::Mapping>>(
        "SELECT mapping FROM scheduling.appointments WHERE workspace_id=$1 AND id=$2",
    )
    .bind(a.workspace)
    .bind(id)
    .fetch_one(&mut *db)
    .await?
    .0;
    let mut answers = if let (Some(form), Some(submission)) = (b.form_id, b.submission_id) {
        forms::booking_submission(db, a, form, submission)
            .await?
            .answers
    } else {
        serde_json::Map::new()
    };
    answers.insert(
        "attendee_name".into(),
        serde_json::Value::String(b.attendee_name.clone()),
    );
    answers.insert(
        "attendee_email".into(),
        serde_json::Value::String(b.attendee_email.clone()),
    );
    let text = |key: &str| {
        answers
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let name = mapping
        .name_fields
        .iter()
        .filter_map(|k| text(k))
        .collect::<Vec<_>>()
        .join(" ");
    let name = if name.is_empty() {
        b.attendee_name.clone()
    } else {
        name
    };
    let name: String = name.chars().take(200).collect();
    let intake = if let Some(id) = b.intake_id {
        contacts::intake::get_in(db, a, id).await?
    } else {
        let mut identifiers = Vec::new();
        if let Some(email) = mapping.email_field.as_ref().and_then(|k| text(k)) {
            identifiers.push(IdentifierInput {
                kind: "email".into(),
                value: email,
                country: None,
                extension: None,
            });
        }
        if let Some(phone) = mapping.phone_field.as_ref().and_then(|k| text(k)) {
            identifiers.push(IdentifierInput {
                kind: "phone".into(),
                value: phone,
                country: mapping.phone_country.clone(),
                extension: None,
            });
        }
        let intake = contacts::intake::submit_in(
            db,
            a,
            &format!("booking:{id}:intake"),
            contacts::intake::SubmitIntake {
                source_ref: format!("scheduling:appointment:{id}"),
                proposed: ContactInput {
                    display_name: name.clone(),
                    notes: String::new(),
                    identifiers,
                    owner_membership_id: mapping.owner_membership_id,
                    allow_shared: false,
                },
                contact_id: None,
            },
        )
        .await?;
        sqlx::query(
            "UPDATE scheduling.appointments SET intake_id=$3 WHERE workspace_id=$1 AND id=$2",
        )
        .bind(a.workspace)
        .bind(id)
        .bind(intake.id)
        .execute(&mut *db)
        .await?;
        intake
    };
    let contact = match (intake.state.as_str(), intake.contact_id) {
        ("resolved", Some(id)) => id,
        ("dismissed", _) => {
            sqlx::query("UPDATE scheduling.appointments SET recipe_state='dismissed' WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(id).execute(db).await?;
            return Ok(());
        }
        _ => {
            sqlx::query("UPDATE scheduling.appointments SET recipe_state='awaiting_review' WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(id).execute(db).await?;
            return Ok(());
        }
    };
    let opportunity = crm::save_opportunity_in(
        db,
        a,
        None,
        None,
        &format!("booking:{id}:opportunity"),
        crm::OpportunityInput {
            title: format!("{}: {name}", mapping.title_prefix.trim())
                .chars()
                .take(200)
                .collect(),
            notes: format!("Recorded from appointment {} ({id}).", b.name),
            owner_membership_id: mapping.owner_membership_id,
            pipeline_id: mapping.pipeline_id,
            stage_id: mapping.stage_id,
            expected_value: None,
            currency_code: None,
            expected_close_on: None,
        },
    )
    .await?;
    crm::participants::create_in(
        db,
        a,
        crm::participants::Target::OpportunityContact,
        opportunity.id,
        &format!("booking:{id}:participant"),
        crm::participants::NewParticipant {
            expected_version: opportunity.version,
            reference_id: contact,
            role_label: "Attendee".into(),
            is_primary: true,
        },
    )
    .await?;
    sqlx::query("UPDATE scheduling.appointments SET recipe_state='completed',contact_id=$3,opportunity_id=$4 WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(id).bind(contact).bind(opportunity.id).execute(db).await?;
    Ok(())
}
pub(crate) async fn reminder(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    job: Uuid,
    outbox: &crate::mail::Outbox,
) -> Result<(), Error> {
    let (id,version,sent,skipped)=sqlx::query_as::<_,(Uuid,i64,bool,bool)>("SELECT appointment_id,version,sent_at IS NOT NULL,skipped FROM scheduling.reminders WHERE workspace_id=$1 AND job_id=$2").bind(a.workspace).bind(job).fetch_optional(&mut *db).await?.ok_or(Error::NotFound)?;
    if sent || skipped {
        return Ok(());
    }
    let (_, b) = raw(db, a, id, "scheduling.manage").await?;
    let past = sqlx::query_scalar::<_, bool>(
        "SELECT starts_at<=now() FROM scheduling.appointments WHERE workspace_id=$1 AND id=$2",
    )
    .bind(a.workspace)
    .bind(id)
    .fetch_one(&mut *db)
    .await?;
    if b.version != version || b.state != "confirmed" || past {
        sqlx::query(
            "UPDATE scheduling.reminders SET skipped=true WHERE workspace_id=$1 AND job_id=$2",
        )
        .bind(a.workspace)
        .bind(job)
        .execute(db)
        .await?;
        return Ok(());
    }
    outbox.send(&b.attendee_email,&format!("Appointment reminder: {}",b.name),format!("Your appointment \"{}\" starts at {} (UTC). Your booking timezone is {}.\n\nUse the booking page and private code you saved to review or cancel your appointment.\nBooking reference: {}",b.name,b.starts_at,b.timezone,b.id)).await?;
    sqlx::query(
        "UPDATE scheduling.reminders SET sent_at=now() WHERE workspace_id=$1 AND job_id=$2",
    )
    .bind(a.workspace)
    .bind(job)
    .execute(db)
    .await?;
    Ok(())
}
