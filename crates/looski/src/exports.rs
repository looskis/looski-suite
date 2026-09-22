// SPDX-License-Identifier: AGPL-3.0-only
//! Export composes public business reads; it never bypasses module-owned policies/DAOs.
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    contacts, crm, tags, tasks,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Collection {
    CoordinationRules,
    ContactOrganizations,
    ContactTags,
    OrganizationTags,
    OpportunityTags,
    DealTags,
    Contacts,
    Organizations,
    Tags,
    Opportunities,
    Deals,
    Tasks,
    Files,
    TaskTypes,
    TaskTemplates,
    TaskRevisions,
    TaskHistory,
    ActivityTypes,
    ActivityHistory,
    CoordinationTemplates,
    CoordinationRevisions,
    DealCoordination,
    Pipelines,
    Stages,
    Categories,
    Statuses,
    OpportunityContacts,
    OpportunityOrganizations,
    DealContacts,
    DealOrganizations,
    OpportunityTasks,
    OpportunityHistory,
    DealTasks,
    DealHistory,
    ContactActivities,
    OpportunityActivities,
    DealActivities,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportRequest {
    pub collection: Collection,
    pub parent_id: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
    pub archived: Option<bool>,
    pub ended: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CollectionSpec {
    pub collection: Collection,
    pub parent_required: bool,
    pub supports_archive_selection: bool,
    pub supports_ended_selection: bool,
    pub read_permission: String,
    pub dependent_read_permissions: Vec<String>,
}
impl Collection {
    fn spec(self) -> CollectionSpec {
        let (parent, archived, ended, read) = match self {
            Self::CoordinationRules => (false, true, false, "crm.configuration.read"),
            Self::ContactOrganizations => (true, false, true, "contacts.read"),
            Self::ContactTags => (true, false, false, "contacts.read"),
            Self::OrganizationTags => (true, false, false, "contacts.organizations.read"),
            Self::OpportunityTags => (true, false, false, "crm.opportunities.read"),
            Self::DealTags => (true, false, false, "crm.deals.read"),
            Self::Contacts => (false, true, false, "contacts.read"),
            Self::Organizations => (false, true, false, "contacts.organizations.read"),
            Self::Tags => (false, true, false, "workspace.tags.read"),
            Self::Opportunities => (false, true, false, "crm.opportunities.read"),
            Self::Deals => (false, true, false, "crm.deals.read"),
            Self::Files => (false, true, false, "files.read"),
            Self::Tasks => (false, true, false, "tasks.read"),
            Self::TaskTypes => (false, true, false, "tasks.configuration.read"),
            Self::TaskTemplates => (false, true, false, "tasks.configuration.read"),
            Self::TaskRevisions => (true, false, false, "tasks.configuration.read"),
            Self::TaskHistory => (true, false, false, "tasks.read"),
            Self::ActivityTypes => (false, true, false, "crm.configuration.read"),
            Self::ActivityHistory => (true, false, false, "crm.activities.read"),
            Self::CoordinationTemplates => (false, true, false, "crm.configuration.read"),
            Self::CoordinationRevisions => (true, false, false, "crm.configuration.read"),
            Self::DealCoordination => (true, false, false, "crm.deals.read"),
            Self::Pipelines => (false, true, false, "crm.configuration.read"),
            Self::Stages => (false, true, false, "crm.configuration.read"),
            Self::Categories => (false, true, false, "crm.configuration.read"),
            Self::Statuses => (false, true, false, "crm.configuration.read"),
            Self::OpportunityContacts => (true, false, true, "crm.opportunities.read"),
            Self::OpportunityOrganizations => (true, false, true, "crm.opportunities.read"),
            Self::DealContacts => (true, false, true, "crm.deals.read"),
            Self::DealOrganizations => (true, false, true, "crm.deals.read"),
            Self::OpportunityTasks => (true, false, true, "crm.opportunities.read"),
            Self::OpportunityHistory => (true, false, false, "crm.opportunities.read"),
            Self::DealTasks => (true, false, true, "crm.deals.read"),
            Self::DealHistory => (true, false, false, "crm.deals.read"),
            Self::ContactActivities => (true, true, false, "crm.activities.read"),
            Self::OpportunityActivities => (true, true, false, "crm.activities.read"),
            Self::DealActivities => (true, true, false, "crm.activities.read"),
        };
        CollectionSpec {
            collection: self,
            parent_required: parent,
            supports_archive_selection: archived,
            supports_ended_selection: ended,
            read_permission: read.into(),
            dependent_read_permissions: self.dependencies().iter().map(|s| (*s).into()).collect(),
        }
    }
    fn dependencies(self) -> &'static [&'static str] {
        match self {
            Self::ContactOrganizations => &["contacts.organizations.read"],
            Self::ContactTags | Self::OrganizationTags | Self::OpportunityTags | Self::DealTags => {
                &["workspace.tags.read"]
            }
            Self::OpportunityContacts | Self::DealContacts | Self::ContactActivities => {
                &["contacts.read"]
            }
            Self::OpportunityOrganizations | Self::DealOrganizations => {
                &["contacts.organizations.read"]
            }
            Self::OpportunityTasks | Self::DealTasks | Self::DealCoordination => &["tasks.read"],
            Self::OpportunityActivities => &["crm.opportunities.read"],
            Self::DealActivities => &["crm.deals.read"],
            Self::CoordinationRules | Self::CoordinationTemplates | Self::CoordinationRevisions => {
                &["tasks.configuration.read"]
            }
            _ => &[],
        }
    }
    fn crm(self) -> bool {
        !matches!(
            self,
            Self::ContactOrganizations
                | Self::ContactTags
                | Self::OrganizationTags
                | Self::Contacts
                | Self::Organizations
                | Self::Tags
                | Self::Files
                | Self::Tasks
                | Self::TaskTypes
                | Self::TaskTemplates
                | Self::TaskRevisions
                | Self::TaskHistory
        )
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Manifest {
    pub format: String,
    pub format_version: u32,
    pub application_version: String,
    pub workspace_id: Uuid,
    pub selection: ExportRequest,
    pub consistency: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ExportPage {
    pub manifest: Manifest,
    pub items: Vec<serde_json::Value>,
    pub next: Option<ExportRequest>,
}
async fn authorize(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    collection: Option<Collection>,
) -> Result<(), Error> {
    let mut db = pool.acquire().await?;
    let grant = access::require(
        &mut db,
        actor.identity,
        actor.workspace,
        "workspace.data.export",
    )
    .await?;
    if !grant.workspace_wide {
        return Err(Error::Forbidden);
    }
    if let Some(collection) = collection {
        access::require(
            &mut db,
            actor.identity,
            actor.workspace,
            &collection.spec().read_permission,
        )
        .await?;
        for action in collection.dependencies() {
            access::require(&mut db, actor.identity, actor.workspace, action).await?;
        }
        if collection.crm() {
            crate::modules::require_crm(&mut db, actor).await?;
        }
    }
    Ok(())
}
pub async fn catalog(
    pool: &PgPool,
    actor: &ActorContext<'_>,
) -> Result<Vec<CollectionSpec>, Error> {
    authorize(pool, actor, None).await?;
    Ok(vec![
        Collection::CoordinationRules.spec(),
        Collection::ContactOrganizations.spec(),
        Collection::ContactTags.spec(),
        Collection::OrganizationTags.spec(),
        Collection::OpportunityTags.spec(),
        Collection::DealTags.spec(),
        Collection::Contacts.spec(),
        Collection::Organizations.spec(),
        Collection::Tags.spec(),
        Collection::Opportunities.spec(),
        Collection::Deals.spec(),
        Collection::Files.spec(),
        Collection::Tasks.spec(),
        Collection::TaskTypes.spec(),
        Collection::TaskTemplates.spec(),
        Collection::TaskRevisions.spec(),
        Collection::TaskHistory.spec(),
        Collection::ActivityTypes.spec(),
        Collection::ActivityHistory.spec(),
        Collection::CoordinationTemplates.spec(),
        Collection::CoordinationRevisions.spec(),
        Collection::DealCoordination.spec(),
        Collection::Pipelines.spec(),
        Collection::Stages.spec(),
        Collection::Categories.spec(),
        Collection::Statuses.spec(),
        Collection::OpportunityContacts.spec(),
        Collection::OpportunityOrganizations.spec(),
        Collection::DealContacts.spec(),
        Collection::DealOrganizations.spec(),
        Collection::OpportunityTasks.spec(),
        Collection::OpportunityHistory.spec(),
        Collection::DealTasks.spec(),
        Collection::DealHistory.spec(),
        Collection::ContactActivities.spec(),
        Collection::OpportunityActivities.spec(),
        Collection::DealActivities.spec(),
    ])
}
fn query<T: DeserializeOwned>(input: &ExportRequest) -> Result<T, Error> {
    serde_json::from_value(serde_json::json!({"limit":input.limit,"after":input.after,"archived":input.archived,"ended":input.ended}))
        .map_err(|_|Error::Invalid("Invalid export selection"))
}
fn value<T: Serialize>(value: T) -> serde_json::Value {
    serde_json::to_value(value).expect("business response serializes")
}
#[derive(Deserialize)]
struct SourcePage {
    items: Vec<serde_json::Value>,
    next_cursor: Option<Uuid>,
}
pub async fn page(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    mut input: ExportRequest,
) -> Result<ExportPage, Error> {
    input.limit = Some(input.limit.unwrap_or(25));
    if !(1..=25).contains(&input.limit.unwrap()) {
        return Err(Error::Invalid("Export page limit must be 1–25"));
    }
    let spec = input.collection.spec();
    if spec.parent_required != input.parent_id.is_some()
        || (!spec.supports_archive_selection && input.archived.is_some())
        || (!spec.supports_ended_selection && input.ended.is_some())
    {
        return Err(Error::Invalid(
            "Selection does not match this export collection",
        ));
    }
    authorize(pool, actor, Some(input.collection)).await?;
    let parent = input.parent_id.unwrap_or(Uuid::nil());
    let result = match input.collection {
        Collection::CoordinationRules => {
            value(crm::status_jobs::list(pool, actor, query(&input)?).await?)
        }
        Collection::ContactOrganizations => value(
            contacts::organizations::relationship_page(pool, actor, parent, query(&input)?).await?,
        ),
        Collection::ContactTags => array_page(
            contacts::tagging::list(pool, actor, contacts::tagging::Target::Contact, parent)
                .await?,
            &input,
        )?,
        Collection::OrganizationTags => array_page(
            contacts::tagging::list(pool, actor, contacts::tagging::Target::Organization, parent)
                .await?,
            &input,
        )?,
        Collection::OpportunityTags => array_page(
            crm::tagging::list(pool, actor, crm::tagging::Target::Opportunity, parent).await?,
            &input,
        )?,
        Collection::DealTags => array_page(
            crm::tagging::list(pool, actor, crm::tagging::Target::Deal, parent).await?,
            &input,
        )?,
        Collection::Contacts => value(contacts::list(pool, actor, query(&input)?).await?),
        Collection::Organizations => {
            value(contacts::organizations::list(pool, actor, query(&input)?).await?)
        }
        Collection::Tags => value(tags::list(pool, actor, query(&input)?).await?),
        Collection::Opportunities => value(crm::opportunities(pool, actor, query(&input)?).await?),
        Collection::Deals => value(crm::deals(pool, actor, query(&input)?).await?),
        Collection::Files => value(crate::files::list(pool, actor, query(&input)?).await?),
        Collection::Tasks => value(tasks::list(pool, actor, query(&input)?).await?),
        Collection::TaskTypes => {
            value(tasks::configuration::types(pool, actor, query(&input)?).await?)
        }
        Collection::TaskTemplates => {
            value(tasks::configuration::templates(pool, actor, query(&input)?).await?)
        }
        Collection::TaskRevisions => {
            value(tasks::configuration::revisions(pool, actor, parent, query(&input)?).await?)
        }
        Collection::TaskHistory => {
            value(tasks::history(pool, actor, parent, query(&input)?).await?)
        }
        Collection::ActivityTypes => {
            value(crm::activities::types::list(pool, actor, query(&input)?).await?)
        }
        Collection::ActivityHistory => {
            value(crm::activities::history(pool, actor, parent, query(&input)?).await?)
        }
        Collection::CoordinationTemplates => {
            value(crm::coordination::templates(pool, actor, query(&input)?).await?)
        }
        Collection::CoordinationRevisions => {
            value(crm::coordination::revisions(pool, actor, parent, query(&input)?).await?)
        }
        Collection::DealCoordination => {
            value(crm::coordination::export_instances(pool, actor, parent, query(&input)?).await?)
        }
        Collection::Pipelines => value(
            crm::configuration::list(
                pool,
                actor,
                crm::configuration::Kind::Pipeline,
                query(&input)?,
            )
            .await?,
        ),
        Collection::Stages => value(
            crm::configuration::list(pool, actor, crm::configuration::Kind::Stage, query(&input)?)
                .await?,
        ),
        Collection::Categories => value(
            crm::configuration::list(
                pool,
                actor,
                crm::configuration::Kind::Category,
                query(&input)?,
            )
            .await?,
        ),
        Collection::Statuses => value(
            crm::configuration::list(
                pool,
                actor,
                crm::configuration::Kind::Status,
                query(&input)?,
            )
            .await?,
        ),
        Collection::OpportunityContacts => value(
            crm::participants::list(
                pool,
                actor,
                crm::participants::Target::OpportunityContact,
                parent,
                query(&input)?,
            )
            .await?,
        ),
        Collection::OpportunityOrganizations => value(
            crm::participants::list(
                pool,
                actor,
                crm::participants::Target::OpportunityOrganization,
                parent,
                query(&input)?,
            )
            .await?,
        ),
        Collection::DealContacts => value(
            crm::participants::list(
                pool,
                actor,
                crm::participants::Target::DealContact,
                parent,
                query(&input)?,
            )
            .await?,
        ),
        Collection::DealOrganizations => value(
            crm::participants::list(
                pool,
                actor,
                crm::participants::Target::DealOrganization,
                parent,
                query(&input)?,
            )
            .await?,
        ),
        Collection::OpportunityTasks => value(
            crm::task_links::list(
                pool,
                actor,
                crm::task_links::Target::Opportunity,
                parent,
                query(&input)?,
            )
            .await?,
        ),
        Collection::OpportunityHistory => {
            value(crm::history(pool, actor, parent, false, query(&input)?).await?)
        }
        Collection::DealTasks => value(
            crm::task_links::list(
                pool,
                actor,
                crm::task_links::Target::Deal,
                parent,
                query(&input)?,
            )
            .await?,
        ),
        Collection::DealHistory => {
            value(crm::history(pool, actor, parent, true, query(&input)?).await?)
        }
        Collection::ContactActivities => value(
            crm::activities::list(
                pool,
                actor,
                crate::crm::activities::ActivityQuery {
                    anchor_kind: crm::activities::AnchorKind::Contact,
                    anchor_id: parent,
                    limit: input.limit,
                    after: input.after,
                    archived: input.archived,
                    type_id: None,
                },
            )
            .await?,
        ),
        Collection::OpportunityActivities => value(
            crm::activities::list(
                pool,
                actor,
                crate::crm::activities::ActivityQuery {
                    anchor_kind: crm::activities::AnchorKind::Opportunity,
                    anchor_id: parent,
                    limit: input.limit,
                    after: input.after,
                    archived: input.archived,
                    type_id: None,
                },
            )
            .await?,
        ),
        Collection::DealActivities => value(
            crm::activities::list(
                pool,
                actor,
                crate::crm::activities::ActivityQuery {
                    anchor_kind: crm::activities::AnchorKind::Deal,
                    anchor_id: parent,
                    limit: input.limit,
                    after: input.after,
                    archived: input.archived,
                    type_id: None,
                },
            )
            .await?,
        ),
    };
    let source: SourcePage = serde_json::from_value(result).expect("public page contract");
    // Current authority is checked again after the owning read; there is no cached grant in a cursor.
    authorize(pool, actor, Some(input.collection)).await?;
    let next = source.next_cursor.map(|after| ExportRequest {
        after: Some(after),
        ..input.clone()
    });
    Ok(ExportPage {
        manifest: Manifest {
            format: "looski-business-json".into(),
            format_version: 1,
            application_version: env!("CARGO_PKG_VERSION").into(),
            workspace_id: actor.workspace,
            selection: input,
            consistency: "live_authorized_pages".into(),
        },
        items: source.items,
        next,
    })
}

// Tag sets are bounded to 50 by their owning operations; preserve page limits and stable IDs.
fn array_page(items: Vec<tags::Tag>, input: &ExportRequest) -> Result<serde_json::Value, Error> {
    let mut items = items;
    items.sort_by_key(|r| r.id);
    if let Some(after) = input.after {
        if !items.iter().any(|r| r.id == after) {
            return Err(Error::NotFound);
        }
        items.retain(|r| r.id > after);
    }
    let limit = input.limit.unwrap_or(25) as usize;
    let more = items.len() > limit;
    items.truncate(limit);
    let next_cursor = if more {
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(serde_json::json!({"items":items,"next_cursor":next_cursor}))
}
