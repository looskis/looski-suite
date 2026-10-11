// SPDX-License-Identifier: AGPL-3.0-only
use super::{Column, Comment, ListProjects, Project, ProjectMember, SaveColumn, SaveProject};
use crate::{access::Access, application::OperationError as Error, tasks::Status};
use sqlx::{PgConnection, Row, types::Json};
use std::collections::HashMap;
use uuid::Uuid;

const PROJECT: &str = "SELECT jsonb_build_object('id',p.id,'name',p.name,'description',p.description,'status',p.status,'due_on',p.due_on::text,'archived',p.archived_at IS NOT NULL,'is_member',false,'source_deal_id',(SELECT c.deal_id FROM projects.deal_conversions c WHERE c.workspace_id=p.workspace_id AND c.project_id=p.id),'version',p.version,'created_at',p.created_at::text,'updated_at',p.updated_at::text) FROM projects.projects p";
const COLUMN: &str = "SELECT jsonb_build_object('id',id,'name',name,'position',position,'task_status',task_status,'version',version) FROM projects.columns";
const COMMENT: &str = "SELECT jsonb_build_object('id',c.id,'project_id',c.project_id,'card_id',c.card_id,'author_membership_id',c.author_membership_id,'author_name',u.display_name,'body',c.body,'edited',c.edited_at IS NOT NULL,'removed',c.removed_at IS NOT NULL,'version',c.version,'created_at',c.created_at::text,'updated_at',c.updated_at::text),k.task_id FROM projects.comments c JOIN projects.cards k ON (k.workspace_id,k.id)=(c.workspace_id,c.card_id) JOIN suite.memberships m ON (m.workspace_id,m.id)=(c.workspace_id,c.author_membership_id) JOIN suite.users u ON u.id=m.user_id";

pub(super) struct CardRow {
    pub id: Uuid,
    pub task_id: Uuid,
    pub column_id: Option<Uuid>,
    pub comment_count: i64,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
pub(super) struct TaskCard {
    pub id: Uuid,
    pub project_id: Uuid,
    pub project_name: String,
}
fn card_row(row: sqlx::postgres::PgRow) -> Result<CardRow, Error> {
    Ok(CardRow {
        id: row.try_get("id")?,
        task_id: row.try_get("task_id")?,
        column_id: row.try_get("column_id")?,
        comment_count: row.try_get("comment_count")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
const CARD: &str = "SELECT k.id,k.task_id,k.column_id,k.version,k.created_at::text AS created_at,k.updated_at::text AS updated_at,(SELECT count(*) FROM projects.comments c WHERE c.workspace_id=k.workspace_id AND c.card_id=k.id AND c.removed_at IS NULL) AS comment_count FROM projects.cards k";

pub(super) async fn get(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<Project, Error> {
    let sql = format!("{PROJECT} WHERE p.workspace_id=$1 AND p.id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<Project>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    ws: Uuid,
    a: &Access,
    q: &ListProjects,
    limit: i64,
) -> Result<Vec<Project>, Error> {
    let sql = format!(
        "{PROJECT} WHERE p.workspace_id=$1 AND ($2 OR EXISTS(SELECT 1 FROM projects.members m WHERE m.workspace_id=p.workspace_id AND m.project_id=p.id AND m.membership_id=$3)) AND ($4::uuid IS NULL OR (p.created_at,p.id)>(SELECT created_at,id FROM projects.projects WHERE workspace_id=$1 AND id=$4)) AND ($5::text IS NULL OR strpos(lower(p.name),lower($5))>0) AND (p.archived_at IS NOT NULL)=$6 AND ($7::text IS NULL OR p.status=$7) ORDER BY p.created_at,p.id LIMIT $8"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Project>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(a.workspace_wide)
            .bind(a.membership_id)
            .bind(q.after)
            .bind(&q.search)
            .bind(q.archived.unwrap_or(false))
            .bind(q.status.map(|s| s.key()))
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn insert(
    db: &mut PgConnection,
    ws: Uuid,
    input: &SaveProject,
    creator: Uuid,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO projects.projects(workspace_id,name,description,status,due_on,created_by_membership_id) VALUES($1,$2,$3,$4,$5::text::date,$6) RETURNING id")
        .bind(ws).bind(input.name.trim()).bind(&input.description).bind(input.status.key()).bind(&input.due_on).bind(creator).fetch_one(db).await?)
}
pub(super) async fn update(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    input: &SaveProject,
) -> Result<(), Error> {
    sqlx::query("UPDATE projects.projects SET name=$3,description=$4,status=$5,due_on=$6::text::date,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(input.name.trim()).bind(&input.description).bind(input.status.key()).bind(&input.due_on).execute(db).await?;
    Ok(())
}
pub(super) async fn archive(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE projects.projects SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
pub(super) async fn is_member(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    member: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects.members WHERE workspace_id=$1 AND project_id=$2 AND membership_id=$3)")
        .bind(ws).bind(project).bind(member).fetch_one(db).await?)
}
pub(super) async fn members(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
) -> Result<Vec<ProjectMember>, Error> {
    Ok(sqlx::query_scalar::<_, Json<ProjectMember>>("SELECT jsonb_build_object('id',m.id,'display_name',u.display_name,'active',m.state='active','added_at',p.added_at::text) FROM projects.members p JOIN suite.memberships m ON (m.workspace_id,m.id)=(p.workspace_id,p.membership_id) JOIN suite.users u ON u.id=m.user_id WHERE p.workspace_id=$1 AND p.project_id=$2 ORDER BY u.display_name,m.id")
        .bind(ws).bind(project).fetch_all(db).await?.into_iter().map(|r| r.0).collect())
}
pub(super) async fn directory(
    db: &mut PgConnection,
    ws: Uuid,
) -> Result<Vec<ProjectMember>, Error> {
    Ok(sqlx::query_scalar::<_, Json<ProjectMember>>("SELECT jsonb_build_object('id',m.id,'display_name',u.display_name,'active',true,'added_at',NULL) FROM suite.memberships m JOIN suite.users u ON u.id=m.user_id WHERE m.workspace_id=$1 AND m.state='active' ORDER BY u.display_name,m.id LIMIT 500")
        .bind(ws).fetch_all(db).await?.into_iter().map(|r| r.0).collect())
}
pub(super) async fn add_member(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    member: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO projects.members(workspace_id,project_id,membership_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
        .bind(ws).bind(project).bind(member).execute(db).await?;
    Ok(())
}
pub(super) async fn remove_member(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    member: Uuid,
) -> Result<(), Error> {
    sqlx::query(
        "DELETE FROM projects.members WHERE workspace_id=$1 AND project_id=$2 AND membership_id=$3",
    )
    .bind(ws)
    .bind(project)
    .bind(member)
    .execute(db)
    .await?;
    Ok(())
}
pub(super) async fn columns(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
) -> Result<Vec<Column>, Error> {
    let sql = format!("{COLUMN} WHERE workspace_id=$1 AND project_id=$2 ORDER BY position");
    Ok(
        sqlx::query_scalar::<_, Json<Column>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(project)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn insert_column(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    name: &str,
    position: i32,
    status: Option<Status>,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO projects.columns(workspace_id,project_id,name,position,task_status) VALUES($1,$2,$3,$4,$5) RETURNING id")
        .bind(ws).bind(project).bind(name).bind(position).bind(status.map(|s| s.key())).fetch_one(db).await?)
}
pub(super) async fn update_column(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    input: &SaveColumn,
) -> Result<(), Error> {
    sqlx::query("UPDATE projects.columns SET name=$3,task_status=$4,version=version+1 WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(input.name.trim()).bind(input.task_status.map(|s| s.key())).execute(db).await?;
    Ok(())
}
pub(super) async fn order_columns(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    order: &[Uuid],
) -> Result<(), Error> {
    sqlx::query("UPDATE projects.columns c SET position=o.n-1 FROM unnest($3::uuid[]) WITH ORDINALITY o(id,n) WHERE c.workspace_id=$1 AND c.project_id=$2 AND c.id=o.id AND c.position<>o.n-1")
        .bind(ws).bind(project).bind(order).execute(db).await?;
    Ok(())
}
pub(super) async fn delete_column(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("DELETE FROM projects.columns WHERE workspace_id=$1 AND id=$2")
        .bind(ws)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}
pub(super) async fn cards(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    ended: bool,
) -> Result<Vec<CardRow>, Error> {
    let sql = format!(
        "{CARD} LEFT JOIN projects.columns l ON (l.workspace_id,l.id)=(k.workspace_id,k.column_id) WHERE k.workspace_id=$1 AND k.project_id=$2 AND (k.removed_at IS NOT NULL)=$3 ORDER BY l.position,k.position,k.id"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(project)
        .bind(ended)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(card_row)
        .collect()
}
pub(super) async fn card(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    id: Uuid,
) -> Result<CardRow, Error> {
    let sql = format!("{CARD} WHERE k.workspace_id=$1 AND k.project_id=$2 AND k.id=$3");
    card_row(
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(project)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?,
    )
}
pub(super) async fn active_cards(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
) -> Result<i64, Error> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM projects.cards WHERE workspace_id=$1 AND project_id=$2 AND removed_at IS NULL")
        .bind(ws).bind(project).fetch_one(db).await?)
}
pub(super) async fn on_a_board(db: &mut PgConnection, ws: Uuid, task: Uuid) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects.cards WHERE workspace_id=$1 AND task_id=$2 AND removed_at IS NULL)")
        .bind(ws).bind(task).fetch_one(db).await?)
}
pub(super) async fn add_card(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    task: Uuid,
    column: Uuid,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO projects.cards(workspace_id,project_id,task_id,column_id,position) VALUES($1,$2,$3,$4,(SELECT count(*) FROM projects.cards WHERE workspace_id=$1 AND column_id=$4)) ON CONFLICT(workspace_id,project_id,task_id) DO UPDATE SET column_id=excluded.column_id,position=excluded.position,removed_at=NULL,version=projects.cards.version+1,updated_at=now() RETURNING id")
        .bind(ws).bind(project).bind(task).bind(column).fetch_one(db).await?)
}
pub(super) async fn column_order(
    db: &mut PgConnection,
    ws: Uuid,
    column: Uuid,
) -> Result<Vec<Uuid>, Error> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM projects.cards WHERE workspace_id=$1 AND column_id=$2 ORDER BY position",
    )
    .bind(ws)
    .bind(column)
    .fetch_all(db)
    .await?)
}
/// Put exactly these cards in this column, in this order.
pub(super) async fn place_cards(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    column: Uuid,
    order: &[Uuid],
) -> Result<(), Error> {
    sqlx::query("UPDATE projects.cards c SET column_id=$3,position=o.n-1 FROM unnest($4::uuid[]) WITH ORDINALITY o(id,n) WHERE c.workspace_id=$1 AND c.project_id=$2 AND c.id=o.id AND (c.column_id IS DISTINCT FROM $3 OR c.position IS DISTINCT FROM o.n-1)")
        .bind(ws).bind(project).bind(column).bind(order).execute(db).await?;
    Ok(())
}
pub(super) async fn compact_column(
    db: &mut PgConnection,
    ws: Uuid,
    column: Uuid,
) -> Result<(), Error> {
    sqlx::query("UPDATE projects.cards c SET position=o.n FROM (SELECT id,row_number() OVER (ORDER BY position)-1 AS n FROM projects.cards WHERE workspace_id=$1 AND column_id=$2) o WHERE c.workspace_id=$1 AND c.id=o.id AND c.position<>o.n")
        .bind(ws).bind(column).execute(db).await?;
    Ok(())
}
pub(super) async fn touch_card(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE projects.cards SET version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).execute(db).await?;
    Ok(())
}
pub(super) async fn remove_card(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE projects.cards SET column_id=NULL,position=NULL,removed_at=now(),version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).execute(db).await?;
    Ok(())
}
/// Active cards for these tasks in projects within the reader's scope.
pub(super) async fn cards_for_tasks(
    db: &mut PgConnection,
    ws: Uuid,
    a: &Access,
    tasks: &[Uuid],
) -> Result<HashMap<Uuid, TaskCard>, Error> {
    let rows = sqlx::query("SELECT k.id,k.task_id,p.id AS project_id,p.name FROM projects.cards k JOIN projects.projects p ON (p.workspace_id,p.id)=(k.workspace_id,k.project_id) WHERE k.workspace_id=$1 AND k.task_id=ANY($2) AND k.removed_at IS NULL AND p.archived_at IS NULL AND ($3 OR EXISTS(SELECT 1 FROM projects.members m WHERE m.workspace_id=p.workspace_id AND m.project_id=p.id AND m.membership_id=$4))")
        .bind(ws).bind(tasks).bind(a.workspace_wide).bind(a.membership_id).fetch_all(db).await?;
    let mut cards = HashMap::with_capacity(rows.len());
    for row in rows {
        cards.insert(
            row.try_get("task_id")?,
            TaskCard {
                id: row.try_get("id")?,
                project_id: row.try_get("project_id")?,
                project_name: row.try_get("name")?,
            },
        );
    }
    Ok(cards)
}
/// The installation's date and a week ahead, which deadlines are compared with.
pub(super) async fn today(db: &mut PgConnection) -> Result<(String, String), Error> {
    Ok(
        sqlx::query_as("SELECT current_date::text,(current_date+7)::text")
            .fetch_one(db)
            .await?,
    )
}
pub(super) async fn comment(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    id: Uuid,
) -> Result<(Comment, Uuid), Error> {
    let sql = format!("{COMMENT} WHERE c.workspace_id=$1 AND c.project_id=$2 AND c.id=$3");
    let row = sqlx::query_as::<_, (Json<Comment>, Uuid)>(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(project)
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)?;
    Ok((row.0.0, row.1))
}
pub(super) async fn comments(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    card: Option<Uuid>,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<(Comment, Uuid)>, Error> {
    let sql = format!(
        "{COMMENT} WHERE c.workspace_id=$1 AND c.project_id=$2 AND ($3::uuid IS NULL OR c.card_id=$3) AND ($4::uuid IS NULL OR (c.created_at,c.id)>(SELECT created_at,id FROM projects.comments WHERE workspace_id=$1 AND id=$4)) ORDER BY c.created_at,c.id LIMIT $5"
    );
    Ok(
        sqlx::query_as::<_, (Json<Comment>, Uuid)>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(project)
            .bind(card)
            .bind(after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|(comment, task)| (comment.0, task))
            .collect(),
    )
}
pub(super) async fn insert_comment(
    db: &mut PgConnection,
    ws: Uuid,
    project: Uuid,
    card: Uuid,
    author: Uuid,
    body: &str,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO projects.comments(workspace_id,project_id,card_id,author_membership_id,body) VALUES($1,$2,$3,$4,$5) RETURNING id")
        .bind(ws).bind(project).bind(card).bind(author).bind(body).fetch_one(db).await?)
}
pub(super) async fn edit_comment(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    body: &str,
) -> Result<(), Error> {
    sqlx::query("UPDATE projects.comments SET body=$3,edited_at=now(),version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(body).execute(db).await?;
    Ok(())
}
pub(super) async fn remove_comment(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE projects.comments SET body='',removed_at=now(),version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).execute(db).await?;
    Ok(())
}
