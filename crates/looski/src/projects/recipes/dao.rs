// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    DealConversion, DealConversionContext, DealRecipe, DealRecipeConfiguration, DealRecipeRevision,
};
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
    pub deal_id: Uuid,
    pub state: String,
    pub configuration: DealRecipeConfiguration,
}
const RECIPE: &str = "SELECT r.id,r.name,r.active,r.actor_membership_id,r.version,r.created_at::text AS created_at,r.updated_at::text AS updated_at,v.id AS revision_id,v.number,v.configuration,v.created_at::text AS revised_at FROM projects.deal_recipes r LEFT JOIN projects.deal_recipe_revisions v ON (v.workspace_id,v.id)=(r.workspace_id,r.revision_id)";
const RUN: &str = "SELECT jsonb_build_object('id',c.id,'recipe_id',c.recipe_id,'state',c.state,'revision_number',v.number,'deal_id',c.deal_id,'project_id',c.project_id,'job_id',c.job_id,'job_state',j.state,'error_code',j.error_code,'version',c.version,'created_at',c.created_at::text,'updated_at',c.updated_at::text) FROM projects.deal_conversions c JOIN projects.deal_recipe_revisions v ON (v.workspace_id,v.id)=(c.workspace_id,c.revision_id) JOIN suite.jobs j ON (j.workspace_id,j.id)=(c.workspace_id,c.job_id)";

fn recipe(row: PgRow) -> Result<DealRecipe, Error> {
    let revision: Option<Uuid> = row.try_get("revision_id")?;
    Ok(DealRecipe {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        active: row.try_get("active")?,
        revision: match revision {
            Some(id) => Some(DealRecipeRevision {
                id,
                number: row.try_get("number")?,
                configuration: row
                    .try_get::<Json<DealRecipeConfiguration>, _>("configuration")?
                    .0,
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
pub(super) async fn get(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    lock: bool,
) -> Result<DealRecipe, Error> {
    let sql = format!(
        "{RECIPE} WHERE r.workspace_id=$1 AND r.id=$2 {}",
        if lock { "FOR UPDATE OF r" } else { "" }
    );
    recipe(
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
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<DealRecipe>, Error> {
    let sql = format!(
        "{RECIPE} WHERE r.workspace_id=$1 AND ($2::uuid IS NULL OR (r.created_at,r.id)>(SELECT created_at,id FROM projects.deal_recipes WHERE workspace_id=$1 AND id=$2)) ORDER BY r.created_at,r.id LIMIT $3"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(after)
        .bind(limit)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(recipe)
        .collect()
}
pub(super) async fn insert(db: &mut PgConnection, ws: Uuid, name: &str) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar(
        "INSERT INTO projects.deal_recipes(workspace_id,name) VALUES($1,$2) RETURNING id",
    )
    .bind(ws)
    .bind(name)
    .fetch_one(db)
    .await?)
}
/// Saving appends an immutable revision; runs keep the revision they started from.
pub(super) async fn revise(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    name: &str,
    configuration: &DealRecipeConfiguration,
    author: Uuid,
) -> Result<(), Error> {
    let revision: Uuid = sqlx::query_scalar("INSERT INTO projects.deal_recipe_revisions(workspace_id,recipe_id,number,configuration,created_by_membership_id) VALUES($1,$2,(SELECT COALESCE(max(number),0)+1 FROM projects.deal_recipe_revisions WHERE workspace_id=$1 AND recipe_id=$2),$3,$4) RETURNING id")
        .bind(ws).bind(recipe).bind(Json(configuration)).bind(author).fetch_one(&mut *db).await?;
    sqlx::query("UPDATE projects.deal_recipes SET name=$3,revision_id=$4,status_id=$5,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(recipe).bind(name).bind(revision).bind(configuration.status_id).execute(db).await?;
    Ok(())
}
pub(super) async fn conflicting(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    status: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects.deal_recipes WHERE workspace_id=$1 AND status_id=$3 AND active AND id<>$2)")
        .bind(ws).bind(recipe).bind(status).fetch_one(db).await?)
}
pub(super) async fn activate(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    recipe: Uuid,
    write: &Access,
) -> Result<(), Error> {
    sqlx::query("UPDATE projects.deal_recipes SET active=true,actor_user_id=$3,actor_membership_id=$4,actor_issuer=$5,actor_subject=$6,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(actor.workspace).bind(recipe).bind(write.user_id).bind(write.membership_id).bind(actor.identity.issuer()).bind(actor.identity.subject())
        .execute(db).await?;
    Ok(())
}
pub(super) async fn deactivate(db: &mut PgConnection, ws: Uuid, recipe: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE projects.deal_recipes SET active=false,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(recipe).execute(db).await?;
    Ok(())
}
pub(super) async fn active_for_status(
    db: &mut PgConnection,
    ws: Uuid,
    status: Uuid,
) -> Result<Option<(Uuid, Uuid, Authority)>, Error> {
    let row = sqlx::query("SELECT id,revision_id,actor_user_id,actor_membership_id,actor_issuer,actor_subject FROM projects.deal_recipes WHERE workspace_id=$1 AND status_id=$2 AND active")
        .bind(ws).bind(status).fetch_optional(db).await?;
    let Some(row) = row else { return Ok(None) };
    Ok(Some((
        row.try_get("id")?,
        row.try_get("revision_id")?,
        Authority {
            user: row.try_get("actor_user_id")?,
            membership: row.try_get("actor_membership_id")?,
            issuer: row.try_get("actor_issuer")?,
            subject: row.try_get("actor_subject")?,
        },
    )))
}
pub(super) async fn converted(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    deal: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects.deal_conversions WHERE workspace_id=$1 AND recipe_id=$2 AND deal_id=$3)")
        .bind(ws).bind(recipe).bind(deal).fetch_one(db).await?)
}
pub(super) async fn insert_run(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    revision: Uuid,
    deal: Uuid,
    job: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO projects.deal_conversions(workspace_id,recipe_id,revision_id,deal_id,job_id) VALUES($1,$2,$3,$4,$5)")
        .bind(ws).bind(recipe).bind(revision).bind(deal).bind(job).execute(db).await?;
    Ok(())
}
pub(super) async fn run(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    id: Uuid,
) -> Result<DealConversion, Error> {
    let sql = format!("{RUN} WHERE c.workspace_id=$1 AND c.recipe_id=$2 AND c.id=$3");
    Ok(
        sqlx::query_scalar::<_, Json<DealConversion>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(recipe)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn runs(
    db: &mut PgConnection,
    ws: Uuid,
    recipe: Uuid,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<DealConversion>, Error> {
    let sql = format!(
        "{RUN} WHERE c.workspace_id=$1 AND c.recipe_id=$2 AND ($3::uuid IS NULL OR (c.created_at,c.id)>(SELECT created_at,id FROM projects.deal_conversions WHERE workspace_id=$1 AND id=$3)) ORDER BY c.created_at,c.id LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<DealConversion>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(recipe)
            .bind(after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn touch(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE projects.deal_conversions SET version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).execute(db).await?;
    Ok(())
}
pub(super) async fn run_for_job(
    db: &mut PgConnection,
    ws: Uuid,
    job: Uuid,
) -> Result<RunRow, Error> {
    let row = sqlx::query("SELECT c.id,c.deal_id,c.state,v.configuration FROM projects.deal_conversions c JOIN projects.deal_recipe_revisions v ON (v.workspace_id,v.id)=(c.workspace_id,c.revision_id) WHERE c.workspace_id=$1 AND c.job_id=$2 FOR UPDATE OF c")
        .bind(ws).bind(job).fetch_optional(db).await?.ok_or(Error::NotFound)?;
    Ok(RunRow {
        id: row.try_get("id")?,
        deal_id: row.try_get("deal_id")?,
        state: row.try_get("state")?,
        configuration: row
            .try_get::<Json<DealRecipeConfiguration>, _>("configuration")?
            .0,
    })
}
pub(super) async fn complete(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    project: Uuid,
) -> Result<(), Error> {
    sqlx::query("UPDATE projects.deal_conversions SET state='completed',project_id=$3,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(project).execute(db).await?;
    Ok(())
}
pub(super) async fn context(
    db: &mut PgConnection,
    ws: Uuid,
    job: Uuid,
) -> Result<Option<DealConversionContext>, Error> {
    let row = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT id,recipe_id FROM projects.deal_conversions WHERE workspace_id=$1 AND job_id=$2",
    )
    .bind(ws)
    .bind(job)
    .fetch_optional(db)
    .await?;
    Ok(row.map(|(run_id, recipe_id)| DealConversionContext { run_id, recipe_id }))
}
