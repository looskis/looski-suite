// SPDX-License-Identifier: AGPL-3.0-only
//! An explicit removable copy of a small, versioned set of CRM defaults.
mod dao;
use super::{activities::types as activity_types, configuration as cfg, coordination, status_jobs};
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    modules, reliability,
    tasks::configuration as task_cfg,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct StarterPackage {
    pub key: String,
    pub name: String,
    pub version: i32,
    pub description: String,
}
fn package() -> StarterPackage {
    StarterPackage{key:"general-crm".into(),name:"General CRM".into(),version:1,description:"Editable sales stages, deal statuses, activity types and one status-triggered follow-up checklist.".into()}
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct StarterSummary {
    pub id: Uuid,
    pub package_version: i32,
    pub version: i64,
    pub installed_at: String,
    pub removed: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct StarterRecord {
    pub kind: String,
    pub id: Uuid,
    pub name: String,
    pub archived: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RemovalDecision {
    pub kind: String,
    pub id: Uuid,
    pub outcome: String,
    pub reason: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct StarterInstallation {
    #[serde(flatten)]
    pub summary: StarterSummary,
    pub records: Vec<StarterRecord>,
    pub removal: Option<Vec<RemovalDecision>>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct StarterPage {
    pub package: StarterPackage,
    pub items: Vec<StarterSummary>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct StarterQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallStarter {
    pub package_version: i32,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveStarter {
    pub expected_version: i64,
}
async fn require(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    write: bool,
) -> Result<access::Access, Error> {
    let a = super::authorize(
        db,
        actor,
        if write {
            "crm.configuration.manage"
        } else {
            "crm.configuration.read"
        },
    )
    .await?;
    if !a.workspace_wide {
        return Err(Error::Forbidden);
    }
    let read = super::authorize(db, actor, "crm.configuration.read").await?;
    if !read.workspace_wide {
        return Err(Error::Forbidden);
    }
    task_cfg::require_read(db, actor).await?;
    if write {
        let tasks = access::require(
            db,
            actor.identity,
            actor.workspace,
            "tasks.configuration.manage",
        )
        .await?;
        if !tasks.workspace_wide {
            return Err(Error::Forbidden);
        }
    }
    Ok(a)
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    q: StarterQuery,
) -> Result<StarterPage, Error> {
    let n = q.limit.unwrap_or(25);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Package page limit must be 1–100"));
    }
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, false).await?;
    let mut items = dao::list(&mut tx, actor.workspace, &q, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|i| i.id)
    } else {
        None
    };
    Ok(StarterPage {
        package: package(),
        items,
        next_cursor,
    })
}
pub async fn get(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<StarterInstallation, Error> {
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, false).await?;
    project(&mut tx, actor, id).await
}
fn record(kind: &str, id: Uuid, name: String, archived: bool, version: i64) -> StarterRecord {
    StarterRecord {
        kind: kind.into(),
        id,
        name,
        archived,
        version,
    }
}
async fn project(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<StarterInstallation, Error> {
    let stored = dao::get(db, actor.workspace, id).await?;
    let mut records = Vec::new();
    for (kind, id) in [
        (cfg::Kind::Pipeline, stored.refs.pipeline),
        (cfg::Kind::Category, stored.refs.category),
    ]
    .into_iter()
    .chain(stored.refs.stages.iter().map(|i| (cfg::Kind::Stage, *i)))
    .chain(stored.refs.statuses.iter().map(|i| (cfg::Kind::Status, *i)))
    {
        let d = cfg::get_in_transaction(db, actor, kind, id).await?;
        records.push(record(kind.key(), d.id, d.name, d.archived, d.version));
    }
    for id in [stored.refs.note, stored.refs.interaction] {
        let d = activity_types::get_in_transaction(db, actor, id).await?;
        records.push(record(
            "activity-types",
            d.id,
            d.name,
            d.archived,
            d.version,
        ));
    }
    let d = task_cfg::task_type_in_transaction(db, actor, stored.refs.task_type).await?;
    records.push(record("task-types", d.id, d.name, d.archived, d.version));
    let d = task_cfg::template_in_transaction(db, actor, stored.refs.task_template).await?;
    records.push(record(
        "task-templates",
        d.id,
        d.name,
        d.archived,
        d.version,
    ));
    let d = coordination::template_in_transaction(db, actor, stored.refs.coordination).await?;
    records.push(record(
        "coordination-templates",
        d.id,
        d.name,
        d.archived,
        d.version,
    ));
    let d = status_jobs::get_in_transaction(db, actor, stored.refs.rule).await?;
    records.push(record(
        "coordination-rules",
        d.id,
        d.name,
        d.archived,
        d.version,
    ));
    Ok(StarterInstallation {
        summary: stored.summary,
        records,
        removal: stored.removal,
    })
}
fn seed(
    name: &str,
    parent_id: Option<Uuid>,
    position: i32,
    outcome: Option<cfg::Outcome>,
) -> cfg::SaveDefinition {
    cfg::SaveDefinition {
        name: name.into(),
        parent_id,
        outcome,
        position: Some(position),
        archived: Some(false),
        expected_version: None,
    }
}
async fn definition(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    installation: Uuid,
    kind: cfg::Kind,
    input: cfg::SaveDefinition,
) -> Result<Uuid, Error> {
    let position = input.position.unwrap_or(0);
    Ok(cfg::save_in_transaction(
        db,
        actor,
        kind,
        None,
        &format!("starter:{installation}:{}:{position}", kind.key()),
        input,
    )
    .await?
    .id)
}
pub async fn install(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: InstallStarter,
) -> Result<StarterInstallation, Error> {
    if input.package_version != 1 {
        return Err(Error::Invalid("Only starter version 1 is available"));
    }
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, true).await?;
    modules::lock_workspace(&mut tx, actor.workspace).await?;
    let a = require(&mut tx, actor, true).await?;
    let request = serde_json::to_value(&input).expect("package serializes");
    let operation = "crm.starter.install";
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &a, operation, key, &request).await?
    {
        return Ok(saved);
    }
    if dao::active(&mut tx, actor.workspace).await? {
        return Err(Error::Conflict(
            "This package is already installed; manage its copied definitions or remove it first",
        ));
    }
    let id = Uuid::new_v4();
    let pipeline = definition(
        &mut tx,
        actor,
        id,
        cfg::Kind::Pipeline,
        seed("Sales", None, 0, None),
    )
    .await?;
    let mut stages = Vec::new();
    for (position, (name, outcome)) in [
        ("New", cfg::Outcome::Open),
        ("Qualified", cfg::Outcome::Open),
        ("Proposal", cfg::Outcome::Open),
        ("Won", cfg::Outcome::Won),
        ("Lost", cfg::Outcome::Lost),
    ]
    .into_iter()
    .enumerate()
    {
        stages.push(
            definition(
                &mut tx,
                actor,
                id,
                cfg::Kind::Stage,
                seed(name, Some(pipeline), position as i32, Some(outcome)),
            )
            .await?,
        );
    }
    let category = definition(
        &mut tx,
        actor,
        id,
        cfg::Kind::Category,
        seed("General", None, 0, None),
    )
    .await?;
    let mut statuses = Vec::new();
    for (position, name) in ["Planned", "Active", "Completed"].into_iter().enumerate() {
        statuses.push(
            definition(
                &mut tx,
                actor,
                id,
                cfg::Kind::Status,
                seed(name, Some(category), position as i32, None),
            )
            .await?,
        );
    }
    let mut activity_ids = Vec::new();
    for (position, (name, behavior)) in [
        ("Note", activity_types::ActivityBehavior::Note),
        ("Interaction", activity_types::ActivityBehavior::Interaction),
    ]
    .into_iter()
    .enumerate()
    {
        activity_ids.push(
            activity_types::save_in_transaction(
                &mut tx,
                actor,
                None,
                &format!("starter:{id}:activity:{position}"),
                activity_types::SaveActivityType {
                    name: name.into(),
                    behavior,
                    position: Some(position as i32),
                    archived: Some(false),
                    expected_version: None,
                },
            )
            .await?
            .id,
        );
    }
    let task_type = task_cfg::save_type_in_transaction(
        &mut tx,
        actor,
        None,
        &format!("starter:{id}:task-type"),
        task_cfg::SaveTaskType {
            name: "Follow-up".into(),
            position: Some(0),
            archived: Some(false),
            expected_version: None,
        },
    )
    .await?;
    let task_template = task_cfg::save_template_in_transaction(
        &mut tx,
        actor,
        None,
        &format!("starter:{id}:task-template"),
        task_cfg::SaveTaskTemplate {
            name: "Review requirements".into(),
            archived: Some(false),
            expected_version: None,
            fields: task_cfg::TemplateFields {
                title: "Review requirements".into(),
                notes: "Confirm the agreed scope and next steps.".into(),
                type_id: Some(task_type.id),
                assignment: task_cfg::AssignmentRule::ContextOwner,
                fixed_membership_id: None,
                due_offset_days: 0,
            },
        },
    )
    .await?;
    let coordination = coordination::save_in_transaction(
        &mut tx,
        actor,
        None,
        &format!("starter:{id}:coordination"),
        coordination::SaveCoordinationTemplate {
            name: "Start delivery".into(),
            items: vec![coordination::CoordinationStepInput {
                label: "Review requirements".into(),
                task_template_revision_id: task_template.revision.id,
            }],
            archived: Some(false),
            expected_version: None,
        },
    )
    .await?;
    let rule = status_jobs::save_in_transaction(
        &mut tx,
        actor,
        None,
        &format!("starter:{id}:rule"),
        status_jobs::SaveCoordinationRule {
            name: "Start delivery when active".into(),
            status_id: statuses[1],
            revision_id: coordination.revision.id,
            archived: Some(false),
            expected_version: None,
        },
    )
    .await?;
    let audit = reliability::audit(
        &mut tx,
        actor,
        &a,
        "crm.starter.installed",
        id,
        serde_json::json!({"package_version":1}),
    )
    .await?;
    let refs = dao::References {
        pipeline,
        category,
        note: activity_ids[0],
        interaction: activity_ids[1],
        task_type: task_type.id,
        task_template: task_template.id,
        coordination: coordination.id,
        rule: rule.id,
        stages,
        statuses,
    };
    dao::insert(&mut tx, actor.workspace, id, &refs, audit).await?;
    let result = project(&mut tx, actor, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &a,
        operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}

fn decision(
    kind: &str,
    id: Uuid,
    archived: bool,
    version: i64,
    protected: bool,
) -> RemovalDecision {
    let (outcome, reason) = if archived {
        ("already_archived", None)
    } else if version != 1 {
        ("retained", Some("customized"))
    } else if protected {
        ("retained", Some("active_configuration_reference"))
    } else {
        ("archived", None)
    };
    RemovalDecision {
        kind: kind.into(),
        id,
        outcome: outcome.into(),
        reason: reason.map(str::to_owned),
    }
}
async fn retire_definition(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    installation: Uuid,
    kind: cfg::Kind,
    id: Uuid,
) -> Result<RemovalDecision, Error> {
    let d = cfg::get_in_transaction(db, actor, kind, id).await?;
    let protected = if matches!(kind, cfg::Kind::Status) {
        dao::active_rule_for_status(db, actor.workspace, id).await?
    } else {
        dao::active_children(db, actor.workspace, kind, id).await?
    };
    let result = decision(kind.key(), id, d.archived, d.version, protected);
    if result.outcome == "archived" {
        cfg::save_in_transaction(
            db,
            actor,
            kind,
            Some(id),
            &format!("starter:{installation}:retire:{id}"),
            cfg::SaveDefinition {
                name: d.name,
                parent_id: d.parent_id,
                outcome: d.outcome,
                position: Some(d.position),
                archived: Some(true),
                expected_version: Some(d.version),
            },
        )
        .await?;
    }
    Ok(result)
}
pub async fn remove(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: RemoveStarter,
) -> Result<StarterInstallation, Error> {
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, true).await?;
    modules::lock_workspace(&mut tx, actor.workspace).await?;
    let a = require(&mut tx, actor, true).await?;
    let operation = format!("crm.starter.remove:{id}");
    let request = serde_json::to_value(&input).expect("package serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &a, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let stored = dao::get(&mut tx, actor.workspace, id).await?;
    if stored.summary.version != input.expected_version || stored.summary.removed {
        return Err(Error::Conflict(
            "Package changed or is already removed; reload before removing",
        ));
    }
    let mut decisions = Vec::new();
    // Retire dependants first. Customized active configuration protects its inputs.
    let r = status_jobs::get_in_transaction(&mut tx, actor, stored.refs.rule).await?;
    let d = decision("coordination-rules", r.id, r.archived, r.version, false);
    if d.outcome == "archived" {
        status_jobs::save_in_transaction(
            &mut tx,
            actor,
            Some(r.id),
            &format!("starter:{id}:retire:{}", r.id),
            status_jobs::SaveCoordinationRule {
                name: r.name,
                status_id: r.status_id,
                revision_id: r.revision_id,
                archived: Some(true),
                expected_version: Some(r.version),
            },
        )
        .await?;
    }
    decisions.push(d);
    let r = coordination::template_in_transaction(&mut tx, actor, stored.refs.coordination).await?;
    let d = decision(
        "coordination-templates",
        r.id,
        r.archived,
        r.version,
        dao::active_rule_for_template(&mut tx, actor.workspace, r.id).await?,
    );
    if d.outcome == "archived" {
        coordination::save_in_transaction(
            &mut tx,
            actor,
            Some(r.id),
            &format!("starter:{id}:retire:{}", r.id),
            coordination::SaveCoordinationTemplate {
                name: r.name,
                items: r.revision.items.into_iter().map(|i| i.fields).collect(),
                archived: Some(true),
                expected_version: Some(r.version),
            },
        )
        .await?;
    }
    decisions.push(d);
    let r = task_cfg::template_in_transaction(&mut tx, actor, stored.refs.task_template).await?;
    let d = decision(
        "task-templates",
        r.id,
        r.archived,
        r.version,
        dao::active_checklist_for_task_revision(&mut tx, actor.workspace, r.revision.id).await?,
    );
    if d.outcome == "archived" {
        task_cfg::save_template_in_transaction(
            &mut tx,
            actor,
            Some(r.id),
            &format!("starter:{id}:retire:{}", r.id),
            task_cfg::SaveTaskTemplate {
                name: r.name,
                fields: r.revision.fields,
                archived: Some(true),
                expected_version: Some(r.version),
            },
        )
        .await?;
    }
    decisions.push(d);
    let r = task_cfg::task_type_in_transaction(&mut tx, actor, stored.refs.task_type).await?;
    let d = decision(
        "task-types",
        r.id,
        r.archived,
        r.version,
        task_cfg::active_templates_for_type(&mut tx, actor, r.id).await?,
    );
    if d.outcome == "archived" {
        task_cfg::save_type_in_transaction(
            &mut tx,
            actor,
            Some(r.id),
            &format!("starter:{id}:retire:{}", r.id),
            task_cfg::SaveTaskType {
                name: r.name,
                position: Some(r.position),
                archived: Some(true),
                expected_version: Some(r.version),
            },
        )
        .await?;
    }
    decisions.push(d);
    for activity_id in [stored.refs.note, stored.refs.interaction] {
        let r = activity_types::get_in_transaction(&mut tx, actor, activity_id).await?;
        let d = decision("activity-types", r.id, r.archived, r.version, false);
        if d.outcome == "archived" {
            activity_types::save_in_transaction(
                &mut tx,
                actor,
                Some(r.id),
                &format!("starter:{id}:retire:{}", r.id),
                activity_types::SaveActivityType {
                    name: r.name,
                    behavior: r.behavior,
                    position: Some(r.position),
                    archived: Some(true),
                    expected_version: Some(r.version),
                },
            )
            .await?;
        }
        decisions.push(d);
    }
    for stage in &stored.refs.stages {
        decisions.push(retire_definition(&mut tx, actor, id, cfg::Kind::Stage, *stage).await?);
    }
    for status in &stored.refs.statuses {
        decisions.push(retire_definition(&mut tx, actor, id, cfg::Kind::Status, *status).await?);
    }
    decisions.push(
        retire_definition(
            &mut tx,
            actor,
            id,
            cfg::Kind::Pipeline,
            stored.refs.pipeline,
        )
        .await?,
    );
    decisions.push(
        retire_definition(
            &mut tx,
            actor,
            id,
            cfg::Kind::Category,
            stored.refs.category,
        )
        .await?,
    );
    let audit = reliability::audit(
        &mut tx,
        actor,
        &a,
        "crm.starter.removed",
        id,
        serde_json::json!({"decisions":decisions}),
    )
    .await?;
    dao::finish_removal(&mut tx, actor.workspace, id, audit, &decisions).await?;
    let result = project(&mut tx, actor, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &a,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
