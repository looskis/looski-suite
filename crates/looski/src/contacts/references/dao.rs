// SPDX-License-Identifier: AGPL-3.0-only
use super::{Kind, Reference};
use crate::{access::Access, application::OperationError as Error};
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
pub(super) async fn visible(
    db: &mut PgConnection,
    workspace: Uuid,
    kind: Kind,
    a: &Access,
    ids: &[Uuid],
) -> Result<Vec<Reference>, Error> {
    let sql = match kind {
        Kind::Contact => {
            "SELECT jsonb_build_object('reference_id',c.id,'canonical_id',s.id,'name',s.display_name,'archived',s.archived_at IS NOT NULL) FROM contacts.contacts c JOIN contacts.contacts s ON s.workspace_id=c.workspace_id AND s.id=COALESCE(c.merged_into_id,c.id) WHERE c.workspace_id=$1 AND c.id=ANY($2) AND ($3 OR (c.owner_membership_id=$4 AND s.owner_membership_id=$4))"
        }
        Kind::Organization => {
            "SELECT jsonb_build_object('reference_id',id,'canonical_id',id,'name',name,'archived',archived_at IS NOT NULL) FROM contacts.organizations WHERE workspace_id=$1 AND id=ANY($2) AND ($3 OR owner_membership_id=$4)"
        }
    };
    Ok(sqlx::query_scalar::<_, Json<Reference>>(sql)
        .bind(workspace)
        .bind(ids)
        .bind(a.workspace_wide)
        .bind(a.membership_id)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|r| r.0)
        .collect())
}
pub(super) async fn lock(
    db: &mut PgConnection,
    workspace: Uuid,
    kind: Kind,
    ids: &[Uuid],
) -> Result<(), Error> {
    let sql = match kind {
        Kind::Contact => {
            "SELECT id FROM contacts.contacts WHERE workspace_id=$1 AND id=ANY($2) ORDER BY id FOR SHARE"
        }
        Kind::Organization => {
            "SELECT id FROM contacts.organizations WHERE workspace_id=$1 AND id=ANY($2) ORDER BY id FOR SHARE"
        }
    };
    sqlx::query(sql)
        .bind(workspace)
        .bind(ids)
        .fetch_all(db)
        .await?;
    Ok(())
}
