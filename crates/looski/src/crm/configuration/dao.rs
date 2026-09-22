// SPDX-License-Identifier: AGPL-3.0-only
use super::{Definition, DefinitionQuery, Kind, SaveDefinition};
use crate::application::OperationError as Error;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
fn table(kind: Kind) -> &'static str {
    match kind {
        Kind::Pipeline => "crm.opportunity_pipelines",
        Kind::Stage => "crm.opportunity_stages",
        Kind::Category => "crm.deal_categories",
        Kind::Status => "crm.deal_statuses",
    }
}
fn parent(kind: Kind) -> &'static str {
    match kind {
        Kind::Stage => "pipeline_id",
        Kind::Status => "category_id",
        _ => "NULL::uuid",
    }
}
fn projection(kind: Kind) -> String {
    format!(
        "jsonb_build_object('id',id,'name',name,'parent_id',{},'outcome',{},'position',position,'archived',archived_at IS NOT NULL,'version',version)",
        parent(kind),
        if matches!(kind, Kind::Stage) {
            "outcome"
        } else {
            "NULL::text"
        }
    )
}
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    kind: Kind,
    id: Uuid,
) -> Result<Definition, Error> {
    let sql = format!(
        "SELECT {} FROM {} WHERE workspace_id=$1 AND id=$2",
        projection(kind),
        table(kind)
    );
    Ok(
        sqlx::query_scalar::<_, Json<Definition>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    workspace: Uuid,
    kind: Kind,
    query: &DefinitionQuery,
    limit: i64,
) -> Result<Vec<Definition>, Error> {
    if let Some(after) = query.after {
        get(db, workspace, kind, after).await?;
    }
    let sql = format!(
        "SELECT {} FROM {} WHERE workspace_id=$1 AND ($2::uuid IS NULL OR (position,id)>(SELECT position,id FROM {} WHERE workspace_id=$1 AND id=$2)) AND ($3::uuid IS NULL OR {}=$3) AND (archived_at IS NOT NULL)=$4 ORDER BY position,id LIMIT $5",
        projection(kind),
        table(kind),
        table(kind),
        parent(kind)
    );
    Ok(
        sqlx::query_scalar::<_, Json<Definition>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(query.after)
            .bind(query.parent_id)
            .bind(query.archived.unwrap_or(false))
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn save(
    db: &mut PgConnection,
    workspace: Uuid,
    kind: Kind,
    id: Option<Uuid>,
    input: &SaveDefinition,
) -> Result<Uuid, Error> {
    if let Some(id) = id {
        let sql = format!(
            "UPDATE {} SET name=$3,position=$4,archived_at=CASE WHEN COALESCE($5,archived_at IS NOT NULL) THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2",
            table(kind)
        );
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .bind(input.name.trim())
            .bind(input.position.unwrap_or(0))
            .bind(input.archived)
            .execute(db)
            .await?;
        Ok(id)
    } else {
        let sql = format!(
            "INSERT INTO {}(workspace_id,name,position,archived_at{}) VALUES($1,$2,$3,CASE WHEN COALESCE($4,false) THEN now() ELSE NULL END{}) RETURNING id",
            table(kind),
            match kind {
                Kind::Stage => ",pipeline_id,outcome",
                Kind::Status => ",category_id",
                _ => "",
            },
            match kind {
                Kind::Stage => ",$5,$6",
                Kind::Status => ",$5",
                _ => "",
            }
        );
        let q = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(input.name.trim())
            .bind(input.position.unwrap_or(0))
            .bind(input.archived);
        let q = if kind.parent().is_some() {
            q.bind(input.parent_id)
        } else {
            q
        };
        let q = if matches!(kind, Kind::Stage) {
            q.bind(input.outcome.map(|o| o.key()))
        } else {
            q
        };
        Ok(q.fetch_one(db).await?)
    }
}
