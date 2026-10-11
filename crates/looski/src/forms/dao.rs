// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    Definition, Form, FormVersion, ListForms, Recipient, SaveForm, Submission, VersionRef,
};
use crate::application::OperationError as Error;
use sqlx::{PgConnection, Row, postgres::PgRow, types::Json};
use uuid::Uuid;

const FORM: &str = "SELECT f.id,f.name,f.description,f.confirmation_message,f.draft,f.public_key,f.published_version_id,v.number,
 (v.id IS NULL OR v.definition<>f.draft OR v.name<>f.name) AS unpublished_changes,f.accepting,f.archived_at IS NOT NULL AS archived,
 COALESCE((SELECT array_agg(r.membership_id ORDER BY r.membership_id) FROM forms.notification_recipients r WHERE r.workspace_id=f.workspace_id AND r.form_id=f.id),'{}') AS recipients,
 (SELECT count(*) FROM forms.submissions s WHERE s.workspace_id=f.workspace_id AND s.form_id=f.id) AS submission_count,
 f.version,f.created_at::text AS created_at,f.updated_at::text AS updated_at
 FROM forms.forms f LEFT JOIN forms.versions v ON v.workspace_id=f.workspace_id AND v.id=f.published_version_id";
fn decode(row: PgRow) -> Result<Form, Error> {
    let published: Option<Uuid> = row.try_get("published_version_id")?;
    Ok(Form {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        confirmation_message: row.try_get("confirmation_message")?,
        draft: row.try_get::<Json<Definition>, _>("draft")?.0,
        public_key: row.try_get("public_key")?,
        published_version: match published {
            Some(id) => Some(VersionRef {
                id,
                number: row.try_get("number")?,
            }),
            None => None,
        },
        unpublished_changes: row.try_get("unpublished_changes")?,
        accepting: row.try_get("accepting")?,
        archived: row.try_get("archived")?,
        notification_membership_ids: row.try_get("recipients")?,
        submission_count: row.try_get("submission_count")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
pub(super) async fn get(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    lock: bool,
) -> Result<Form, Error> {
    let sql = format!(
        "{FORM} WHERE f.workspace_id=$1 AND f.id=$2 {}",
        if lock { "FOR UPDATE OF f" } else { "" }
    );
    decode(
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    ws: Uuid,
    q: &ListForms,
    limit: i64,
) -> Result<Vec<Form>, Error> {
    let sql = format!(
        "{FORM} WHERE f.workspace_id=$1 AND (f.archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (f.created_at,f.id)>(SELECT created_at,id FROM forms.forms WHERE workspace_id=$1 AND id=$3)) AND ($4::text IS NULL OR strpos(lower(f.name),lower($4))>0) ORDER BY f.created_at,f.id LIMIT $5"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(q.archived.unwrap_or(false))
        .bind(q.after)
        .bind(&q.search)
        .bind(limit)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode)
        .collect()
}
pub(super) async fn insert(
    db: &mut PgConnection,
    ws: Uuid,
    input: &SaveForm,
    public_key: &str,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO forms.forms(workspace_id,name,description,confirmation_message,draft,public_key) VALUES($1,$2,$3,$4,$5,$6) RETURNING id")
        .bind(ws).bind(input.name.trim()).bind(&input.description).bind(&input.confirmation_message).bind(Json(&input.draft)).bind(public_key)
        .fetch_one(db).await?)
}
pub(super) async fn update(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    input: &SaveForm,
) -> Result<(), Error> {
    sqlx::query("UPDATE forms.forms SET name=$3,description=$4,confirmation_message=$5,draft=$6,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(input.name.trim()).bind(&input.description).bind(&input.confirmation_message).bind(Json(&input.draft))
        .execute(db).await?;
    Ok(())
}
pub(super) async fn publish(
    db: &mut PgConnection,
    ws: Uuid,
    form: &Form,
    member: Uuid,
) -> Result<(), Error> {
    let version: Uuid = sqlx::query_scalar("INSERT INTO forms.versions(workspace_id,form_id,number,name,definition,published_by_membership_id) SELECT $1,$2,COALESCE(max(number),0)+1,$3,$4,$5 FROM forms.versions WHERE workspace_id=$1 AND form_id=$2 RETURNING id")
        .bind(ws).bind(form.id).bind(&form.name).bind(Json(&form.draft)).bind(member)
        .fetch_one(&mut *db).await?;
    sqlx::query("UPDATE forms.forms SET published_version_id=$3,accepting=true,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(form.id).bind(version).execute(db).await?;
    Ok(())
}
pub(super) async fn set_accepting(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    accepting: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE forms.forms SET accepting=$3,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(accepting).execute(db).await?;
    Ok(())
}
/// Archiving closes the form; restoring leaves it closed until reopened.
pub(super) async fn archive(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE forms.forms SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) END,accepting=accepting AND NOT $3,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
pub(super) async fn can_read_submissions(
    db: &mut PgConnection,
    ws: Uuid,
    member: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM suite.memberships m JOIN suite.users u ON u.id=m.user_id
     JOIN suite.membership_roles mr ON (mr.workspace_id,mr.membership_id)=(m.workspace_id,m.id)
     JOIN suite.roles r ON (r.workspace_id,r.id)=(m.workspace_id,mr.role_id)
     JOIN suite.role_permissions rp ON (rp.workspace_id,rp.role_id)=(m.workspace_id,r.id)
     WHERE m.workspace_id=$1 AND m.id=$2 AND m.state='active' AND u.state='active' AND r.archived_at IS NULL
     AND rp.permission_key='forms.submissions.read' AND rp.record_scope='workspace')")
        .bind(ws).bind(member).fetch_one(db).await?)
}
pub(super) async fn set_notifications(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    members: &[Uuid],
) -> Result<(), Error> {
    sqlx::query("DELETE FROM forms.notification_recipients WHERE workspace_id=$1 AND form_id=$2")
        .bind(ws)
        .bind(id)
        .execute(&mut *db)
        .await?;
    sqlx::query("INSERT INTO forms.notification_recipients(workspace_id,form_id,membership_id) SELECT $1,$2,unnest($3::uuid[])")
        .bind(ws).bind(id).bind(members).execute(&mut *db).await?;
    sqlx::query(
        "UPDATE forms.forms SET version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2",
    )
    .bind(ws)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}
fn decode_version(row: PgRow) -> Result<FormVersion, Error> {
    Ok(FormVersion {
        id: row.try_get("id")?,
        form_id: row.try_get("form_id")?,
        number: row.try_get("number")?,
        name: row.try_get("name")?,
        definition: row.try_get::<Json<Definition>, _>("definition")?.0,
        published_at: row.try_get("published_at")?,
    })
}
const VERSION: &str = "SELECT id,form_id,number,name,definition,published_at::text AS published_at FROM forms.versions";
pub(super) async fn versions(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
) -> Result<Vec<FormVersion>, Error> {
    let sql = format!("{VERSION} WHERE workspace_id=$1 AND form_id=$2 ORDER BY number DESC");
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(form)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode_version)
        .collect()
}
/// Newest first, matching the unpaged history route.
pub(super) async fn version_page(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<FormVersion>, Error> {
    let sql = format!(
        "{VERSION} WHERE workspace_id=$1 AND form_id=$2 AND ($3::uuid IS NULL OR number<(SELECT number FROM forms.versions WHERE workspace_id=$1 AND form_id=$2 AND id=$3)) ORDER BY number DESC LIMIT $4"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(form)
        .bind(after)
        .bind(limit)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode_version)
        .collect()
}
pub(crate) async fn version(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
    id: Uuid,
) -> Result<FormVersion, Error> {
    let sql = format!("{VERSION} WHERE workspace_id=$1 AND form_id=$2 AND id=$3");
    decode_version(
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(form)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?,
    )
}
const SUBMISSION: &str = "SELECT s.id,s.form_id,s.version_id,v.number,s.answers,s.submitted_at::text AS submitted_at FROM forms.submissions s JOIN forms.versions v ON v.workspace_id=s.workspace_id AND v.id=s.version_id";
fn decode_submission(row: PgRow) -> Result<Submission, Error> {
    Ok(Submission {
        id: row.try_get("id")?,
        form_id: row.try_get("form_id")?,
        version_id: row.try_get("version_id")?,
        version_number: row.try_get("number")?,
        answers: row
            .try_get::<Json<serde_json::Map<String, serde_json::Value>>, _>("answers")?
            .0,
        submitted_at: row.try_get("submitted_at")?,
        recipe_run: None,
    })
}
pub(super) async fn submissions(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<Submission>, Error> {
    // Newest first, so reviewers see recent responses without paging.
    let sql = format!(
        "{SUBMISSION} WHERE s.workspace_id=$1 AND s.form_id=$2 AND ($3::uuid IS NULL OR (s.submitted_at,s.id)<(SELECT submitted_at,id FROM forms.submissions WHERE workspace_id=$1 AND id=$3)) ORDER BY s.submitted_at DESC,s.id DESC LIMIT $4"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(form)
        .bind(after)
        .bind(limit)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode_submission)
        .collect()
}
pub(crate) async fn submission(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
    id: Uuid,
) -> Result<Submission, Error> {
    let sql = format!("{SUBMISSION} WHERE s.workspace_id=$1 AND s.form_id=$2 AND s.id=$3");
    decode_submission(
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(form)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?,
    )
}
pub(super) async fn answered(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
    key: &str,
) -> Result<i64, Error> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM forms.submissions WHERE workspace_id=$1 AND form_id=$2 AND answers ? $3")
        .bind(ws).bind(form).bind(key).fetch_one(db).await?)
}
pub(super) async fn chosen(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
    key: &str,
    value: serde_json::Value,
) -> Result<i64, Error> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM forms.submissions WHERE workspace_id=$1 AND form_id=$2 AND (answers->$3=$4 OR (jsonb_typeof(answers->$3)='array' AND answers->$3 @> jsonb_build_array($4)))")
        .bind(ws).bind(form).bind(key).bind(Json(value)).fetch_one(db).await?)
}
/// Resolve a public key; the shared locks conflict only with closing,
/// archiving and application disablement.
pub(super) async fn public(
    db: &mut PgConnection,
    key: &str,
    lock: bool,
) -> Result<(Uuid, Uuid, String), Error> {
    let sql = format!(
        "SELECT f.workspace_id,f.id,f.confirmation_message FROM forms.forms f JOIN suite.workspaces w ON w.id=f.workspace_id WHERE f.public_key=$1 AND w.state='active' {}",
        if lock { "FOR SHARE OF f,w" } else { "" }
    );
    sqlx::query_as(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(key)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)
}
pub(super) async fn accepting_version(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
) -> Result<FormVersion, Error> {
    let (published, accepting) = sqlx::query_as::<_, (Option<Uuid>, bool)>(
        "SELECT published_version_id,accepting AND archived_at IS NULL FROM forms.forms WHERE workspace_id=$1 AND id=$2",
    )
    .bind(ws)
    .bind(form)
    .fetch_one(&mut *db)
    .await?;
    // A form that was never published does not exist publicly.
    let id = published.ok_or(Error::NotFound)?;
    if !accepting {
        return Err(Error::Conflict("This form is not accepting responses"));
    }
    version(db, ws, form, id).await
}
pub(super) async fn public_description(db: &mut PgConnection, form: Uuid) -> Result<String, Error> {
    Ok(
        sqlx::query_scalar("SELECT description FROM forms.forms WHERE id=$1")
            .bind(form)
            .fetch_one(db)
            .await?,
    )
}
pub(super) async fn submission_by_key(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
    key: &str,
) -> Result<Option<(Uuid, serde_json::Value)>, Error> {
    // Serialize concurrent retries of one submission key.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("form-submission:{ws}:{form}:{key}"))
        .execute(&mut *db)
        .await?;
    Ok(sqlx::query_as::<_, (Uuid, Json<serde_json::Value>)>("SELECT version_id,answers FROM forms.submissions WHERE workspace_id=$1 AND form_id=$2 AND submission_key=$3")
        .bind(ws).bind(form).bind(key).fetch_optional(db).await?.map(|(version, answers)| (version, answers.0)))
}
pub(super) async fn insert_submission(
    db: &mut PgConnection,
    ws: Uuid,
    version: &FormVersion,
    answers: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO forms.submissions(workspace_id,form_id,version_id,answers,source,submission_key) VALUES($1,$2,$3,$4,'public',$5) RETURNING id")
        .bind(ws).bind(version.form_id).bind(version.id).bind(Json(answers)).bind(key)
        .fetch_one(db).await?)
}
/// Configured recipients that still have an active, linked sign-in identity.
pub(super) async fn recipients(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
) -> Result<Vec<Recipient>, Error> {
    Ok(sqlx::query_as::<_, (Uuid, Uuid, String, String)>("SELECT DISTINCT ON (m.id) u.id,m.id,i.issuer,i.subject FROM forms.notification_recipients r
     JOIN suite.memberships m ON (m.workspace_id,m.id)=(r.workspace_id,r.membership_id) JOIN suite.users u ON u.id=m.user_id
     JOIN suite.auth_identities i ON i.user_id=u.id AND i.disabled_at IS NULL
     WHERE r.workspace_id=$1 AND r.form_id=$2 AND m.state='active' AND u.state='active' ORDER BY m.id,i.linked_at")
        .bind(ws).bind(form).fetch_all(db).await?
        .into_iter()
        .map(|(user, membership, issuer, subject)| Recipient { user, membership, issuer, subject })
        .collect())
}
