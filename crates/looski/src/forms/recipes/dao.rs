// SPDX-License-Identifier: AGPL-3.0-only
use super::{Mapping, Recipe, RecipeRevision, RunContext, RunSummary};
use crate::{
    access::Access,
    application::{ActorContext, OperationError as Error},
};
use sqlx::{PgConnection, Row, postgres::PgRow, types::Json};
use uuid::Uuid;

pub(super) struct Authority {
    pub user: Uuid,
    pub membership: Uuid,
    pub issuer: String,
    pub subject: String,
}
pub(super) struct RunRow {
    pub id: Uuid,
    pub form_id: Uuid,
    pub submission_id: Uuid,
    pub state: String,
    pub intake_id: Option<Uuid>,
    pub mapping: Mapping,
}

pub(super) async fn get(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
    lock: bool,
) -> Result<Recipe, Error> {
    let sql = format!(
        "SELECT r.id,r.form_id,r.active,r.actor_membership_id,r.version,r.created_at::text AS created_at,r.updated_at::text AS updated_at,
         v.id AS revision_id,v.number,v.mapping,v.created_at::text AS revised_at
         FROM forms.crm_recipes r LEFT JOIN forms.crm_recipe_revisions v ON (v.workspace_id,v.id)=(r.workspace_id,r.revision_id)
         WHERE r.workspace_id=$1 AND r.form_id=$2 {}",
        if lock { "FOR UPDATE OF r" } else { "" }
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(form)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)?;
    let revision: Option<Uuid> = row.try_get("revision_id")?;
    Ok(Recipe {
        id: row.try_get("id")?,
        form_id: row.try_get("form_id")?,
        active: row.try_get("active")?,
        revision: match revision {
            Some(id) => Some(RecipeRevision {
                id,
                number: row.try_get("number")?,
                mapping: row.try_get::<Json<Mapping>, _>("mapping")?.0,
                created_at: row.try_get("revised_at")?,
            }),
            None => None,
        },
        activated_by_membership_id: row.try_get("actor_membership_id")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
pub(super) async fn insert(db: &mut PgConnection, ws: Uuid, form: Uuid) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar(
        "INSERT INTO forms.crm_recipes(workspace_id,form_id) VALUES($1,$2) RETURNING id",
    )
    .bind(ws)
    .bind(form)
    .fetch_one(db)
    .await?)
}
pub(super) async fn revise(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    mapping: &Mapping,
    member: Uuid,
) -> Result<(), Error> {
    let revision: Uuid = sqlx::query_scalar("INSERT INTO forms.crm_recipe_revisions(workspace_id,recipe_id,number,mapping,created_by_membership_id) SELECT $1,$2,COALESCE(max(number),0)+1,$3,$4 FROM forms.crm_recipe_revisions WHERE workspace_id=$1 AND recipe_id=$2 RETURNING id")
        .bind(ws).bind(recipe).bind(Json(mapping)).bind(member).fetch_one(&mut *db).await?;
    sqlx::query("UPDATE forms.crm_recipes SET revision_id=$3,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(recipe).bind(revision).execute(db).await?;
    Ok(())
}
pub(super) async fn activate(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    recipe: Uuid,
    write: &Access,
) -> Result<(), Error> {
    sqlx::query("UPDATE forms.crm_recipes SET active=true,actor_user_id=$3,actor_membership_id=$4,actor_issuer=$5,actor_subject=$6,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(actor.workspace).bind(recipe).bind(write.user_id).bind(write.membership_id).bind(actor.identity.issuer()).bind(actor.identity.subject())
        .execute(db).await?;
    Ok(())
}
pub(super) async fn deactivate(db: &mut PgConnection, ws: Uuid, recipe: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE forms.crm_recipes SET active=false,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(recipe).execute(db).await?;
    Ok(())
}
pub(super) async fn authority(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
) -> Result<Authority, Error> {
    let row = sqlx::query_as::<_, (Option<Uuid>, Option<Uuid>, Option<String>, Option<String>)>("SELECT actor_user_id,actor_membership_id,actor_issuer,actor_subject FROM forms.crm_recipes WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(recipe).fetch_one(db).await?;
    match row {
        (Some(user), Some(membership), Some(issuer), Some(subject)) => Ok(Authority {
            user,
            membership,
            issuer,
            subject,
        }),
        _ => Err(Error::Conflict("Activate the recipe before retrying runs")),
    }
}
/// The active recipe and authority for new submissions to a form.
pub(super) async fn active(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
) -> Result<Option<(Uuid, Uuid, Authority)>, Error> {
    Ok(sqlx::query_as::<_, (Uuid, Uuid, Uuid, Uuid, String, String)>("SELECT id,revision_id,actor_user_id,actor_membership_id,actor_issuer,actor_subject FROM forms.crm_recipes WHERE workspace_id=$1 AND form_id=$2 AND active FOR SHARE")
        .bind(ws).bind(form).fetch_optional(db).await?
        .map(|(id, revision, user, membership, issuer, subject)| (id, revision, Authority { user, membership, issuer, subject })))
}
pub(super) async fn insert_run(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    revision: Uuid,
    submission: Uuid,
    job: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO forms.crm_recipe_runs(workspace_id,recipe_id,revision_id,submission_id,job_id) VALUES($1,$2,$3,$4,$5)")
        .bind(ws).bind(recipe).bind(revision).bind(submission).bind(job).execute(db).await?;
    Ok(())
}
const RUN: &str = "SELECT r.id,r.submission_id,r.state,v.number,r.job_id,j.state AS job_state,j.error_code,r.intake_id,r.contact_id,r.opportunity_id,r.version,r.created_at::text AS created_at,r.updated_at::text AS updated_at
 FROM forms.crm_recipe_runs r JOIN forms.crm_recipe_revisions v ON (v.workspace_id,v.id)=(r.workspace_id,r.revision_id)
 JOIN suite.jobs j ON (j.workspace_id,j.id)=(r.workspace_id,r.job_id)";
fn decode(row: PgRow) -> Result<RunSummary, Error> {
    Ok(RunSummary {
        id: row.try_get("id")?,
        submission_id: row.try_get("submission_id")?,
        state: row.try_get("state")?,
        revision_number: row.try_get("number")?,
        job_id: row.try_get("job_id")?,
        job_state: row.try_get("job_state")?,
        error_code: row.try_get("error_code")?,
        intake_id: row.try_get("intake_id")?,
        contact_id: row.try_get("contact_id")?,
        opportunity_id: row.try_get("opportunity_id")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
pub(super) async fn run(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    id: Uuid,
) -> Result<RunSummary, Error> {
    let sql = format!("{RUN} WHERE r.workspace_id=$1 AND r.recipe_id=$2 AND r.id=$3");
    decode(
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(recipe)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?,
    )
}
pub(super) async fn lock_run(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    id: Uuid,
) -> Result<RunSummary, Error> {
    sqlx::query("SELECT id FROM forms.crm_recipe_runs WHERE workspace_id=$1 AND recipe_id=$2 AND id=$3 FOR UPDATE")
        .bind(ws).bind(recipe).bind(id).fetch_optional(&mut *db).await?.ok_or(Error::NotFound)?;
    run(db, ws, recipe, id).await
}
pub(super) async fn runs(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<RunSummary>, Error> {
    let sql = format!(
        "{RUN} WHERE r.workspace_id=$1 AND r.recipe_id=$2 AND ($3::uuid IS NULL OR (r.created_at,r.id)<(SELECT created_at,id FROM forms.crm_recipe_runs WHERE workspace_id=$1 AND id=$3)) ORDER BY r.created_at DESC,r.id DESC LIMIT $4"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(recipe)
        .bind(after)
        .bind(limit)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode)
        .collect()
}
pub(super) async fn for_submissions(
    db: &mut PgConnection,
    ws: Uuid,
    submissions: &[Uuid],
) -> Result<Vec<RunSummary>, Error> {
    let sql = format!("{RUN} WHERE r.workspace_id=$1 AND r.submission_id=ANY($2)");
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(submissions)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode)
        .collect()
}
pub(super) async fn requeue(
    db: &mut PgConnection,
    ws: Uuid,
    run: Uuid,
    job: Uuid,
) -> Result<(), Error> {
    sqlx::query("UPDATE forms.crm_recipe_runs SET job_id=$3,state='queued',version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(run).bind(job).execute(db).await?;
    Ok(())
}
pub(super) async fn touch(db: &mut PgConnection, ws: Uuid, run: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE forms.crm_recipe_runs SET version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(run).execute(db).await?;
    Ok(())
}
pub(super) async fn context(
    db: &mut PgConnection,
    ws: Uuid,
    job: Uuid,
) -> Result<Option<RunContext>, Error> {
    Ok(sqlx::query_as::<_, (Uuid, Uuid, Uuid, Uuid)>("SELECT r.id,r.recipe_id,c.form_id,r.submission_id FROM forms.crm_recipe_runs r JOIN forms.crm_recipes c ON (c.workspace_id,c.id)=(r.workspace_id,r.recipe_id) WHERE r.workspace_id=$1 AND r.job_id=$2")
        .bind(ws).bind(job).fetch_optional(db).await?
        .map(|(run_id, recipe_id, form_id, submission_id)| RunContext { run_id, recipe_id, form_id, submission_id }))
}
pub(super) async fn run_for_job(
    db: &mut PgConnection,
    ws: Uuid,
    job: Uuid,
) -> Result<RunRow, Error> {
    let row = sqlx::query("SELECT r.id,c.form_id,r.submission_id,r.state,r.intake_id,v.mapping FROM forms.crm_recipe_runs r
     JOIN forms.crm_recipes c ON (c.workspace_id,c.id)=(r.workspace_id,r.recipe_id)
     JOIN forms.crm_recipe_revisions v ON (v.workspace_id,v.id)=(r.workspace_id,r.revision_id)
     WHERE r.workspace_id=$1 AND r.job_id=$2 FOR UPDATE OF r")
        .bind(ws).bind(job).fetch_optional(db).await?.ok_or(Error::NotFound)?;
    Ok(RunRow {
        id: row.try_get("id")?,
        form_id: row.try_get("form_id")?,
        submission_id: row.try_get("submission_id")?,
        state: row.try_get("state")?,
        intake_id: row.try_get("intake_id")?,
        mapping: row.try_get::<Json<Mapping>, _>("mapping")?.0,
    })
}
pub(super) async fn set_intake(
    db: &mut PgConnection,
    ws: Uuid,
    run: Uuid,
    intake: Uuid,
) -> Result<(), Error> {
    sqlx::query("UPDATE forms.crm_recipe_runs SET intake_id=$3,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(run).bind(intake).execute(db).await?;
    Ok(())
}
pub(super) async fn finish(
    db: &mut PgConnection,
    ws: Uuid,
    run: Uuid,
    state: &str,
    contact: Option<Uuid>,
    opportunity: Option<Uuid>,
) -> Result<(), Error> {
    sqlx::query("UPDATE forms.crm_recipe_runs SET state=$3,contact_id=COALESCE($4,contact_id),opportunity_id=$5,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(run).bind(state).bind(contact).bind(opportunity).execute(db).await?;
    Ok(())
}
