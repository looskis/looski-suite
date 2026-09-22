// SPDX-License-Identifier: AGPL-3.0-only
use crate::api::SystemResponse;

#[derive(Debug, serde::Serialize)]
pub struct ApiFailure {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
}
impl std::fmt::Display for ApiFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).map_err(|_| std::fmt::Error)?
        )
    }
}
impl std::error::Error for ApiFailure {}

/// CLI and MCP use the HTTP boundary, never database credentials.
#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    base_url: reqwest::Url,
    token_file: Option<std::path::PathBuf>,
    source_interface: &'static str,
}

impl ApiClient {
    pub async fn crm_coordination_templates(
        &self,
        workspace: uuid::Uuid,
        query: &crate::crm::coordination::CoordinationQuery,
    ) -> anyhow::Result<crate::crm::coordination::CoordinationTemplatePage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/coordination-templates"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_coordination_template(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::coordination::CoordinationTemplate> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/coordination-templates/{id}"),
        )?)
        .await
    }
    pub async fn crm_create_coordination_template(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::crm::coordination::SaveCoordinationTemplate,
    ) -> anyhow::Result<crate::crm::coordination::CoordinationTemplate> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/coordination-templates"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_coordination_template(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::coordination::SaveCoordinationTemplate,
    ) -> anyhow::Result<crate::crm::coordination::CoordinationTemplate> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/coordination-templates/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_coordination_revisions(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::coordination::CoordinationPageQuery,
    ) -> anyhow::Result<crate::crm::coordination::CoordinationRevisionPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!(
                    "/api/v1/workspaces/{workspace}/crm/coordination-templates/{id}/revisions"
                ),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_deal_coordination(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::coordination::CoordinationPageQuery,
    ) -> anyhow::Result<crate::crm::coordination::CoordinationInstancePage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/coordination"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_deal_coordination_instance(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        instance_id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::coordination::CoordinationInstance> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/coordination/{instance_id}"),
        )?)
        .await
    }
    pub async fn crm_apply_coordination(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::coordination::ApplyCoordination,
    ) -> anyhow::Result<crate::crm::coordination::CoordinationInstance> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/coordination"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }

    pub async fn tasks_types(
        &self,
        workspace: uuid::Uuid,
        query: &crate::tasks::configuration::TaskConfigurationQuery,
    ) -> anyhow::Result<crate::tasks::configuration::TaskTypePage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/tasks/types"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn tasks_type(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::tasks::configuration::TaskType> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/tasks/types/{id}"),
        )?)
        .await
    }
    pub async fn tasks_create_type(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::tasks::configuration::SaveTaskType,
    ) -> anyhow::Result<crate::tasks::configuration::TaskType> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/tasks/types"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn tasks_edit_type(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::tasks::configuration::SaveTaskType,
    ) -> anyhow::Result<crate::tasks::configuration::TaskType> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/tasks/types/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn tasks_templates(
        &self,
        workspace: uuid::Uuid,
        query: &crate::tasks::configuration::TaskConfigurationQuery,
    ) -> anyhow::Result<crate::tasks::configuration::TaskTemplatePage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/tasks/templates"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn tasks_template(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::tasks::configuration::TaskTemplate> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/tasks/templates/{id}"),
        )?)
        .await
    }
    pub async fn tasks_create_template(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::tasks::configuration::SaveTaskTemplate,
    ) -> anyhow::Result<crate::tasks::configuration::TaskTemplate> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/tasks/templates"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn tasks_edit_template(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::tasks::configuration::SaveTaskTemplate,
    ) -> anyhow::Result<crate::tasks::configuration::TaskTemplate> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/tasks/templates/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn tasks_template_revisions(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::tasks::configuration::TaskRevisionQuery,
    ) -> anyhow::Result<crate::tasks::configuration::TaskRevisionPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/tasks/templates/{id}/revisions"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn tasks_instantiate_template(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::tasks::configuration::InstantiateTaskTemplate,
    ) -> anyhow::Result<crate::tasks::Task> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/tasks/templates/{id}/instantiate"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }

    pub async fn crm_activity_types(
        &self,
        workspace: uuid::Uuid,
        query: &crate::crm::activities::types::ActivityTypeQuery,
    ) -> anyhow::Result<crate::crm::activities::types::ActivityTypePage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/activity-types"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_activity_type(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::activities::types::ActivityType> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/activity-types/{id}"),
        )?)
        .await
    }
    pub async fn crm_create_activity_type(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::crm::activities::types::SaveActivityType,
    ) -> anyhow::Result<crate::crm::activities::types::ActivityType> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/activity-types"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_activity_type(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::activities::types::SaveActivityType,
    ) -> anyhow::Result<crate::crm::activities::types::ActivityType> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/activity-types/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_activities(
        &self,
        workspace: uuid::Uuid,
        query: &crate::crm::activities::ActivityQuery,
    ) -> anyhow::Result<crate::crm::activities::ActivityPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/activities"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_activity(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::activities::Activity> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/activities/{id}"),
        )?)
        .await
    }
    pub async fn crm_create_activity(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::crm::activities::CreateActivity,
    ) -> anyhow::Result<crate::crm::activities::Activity> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/activities"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_activity(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::activities::EditActivity,
    ) -> anyhow::Result<crate::crm::activities::Activity> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/activities/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_archive_activity(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::activities::ArchiveActivity,
    ) -> anyhow::Result<crate::crm::activities::Activity> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/activities/{id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_activity_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::activities::ActivityHistoryQuery,
    ) -> anyhow::Result<crate::crm::activities::ActivityHistoryPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/activities/{id}/history"),
            )?
            .query(query),
        )
        .await
    }

    pub async fn crm_opportunity_tasks(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::task_links::TaskLinkQuery,
    ) -> anyhow::Result<crate::crm::task_links::TaskLinkPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/tasks"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_link_opportunity_task(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::task_links::LinkTask,
    ) -> anyhow::Result<crate::crm::task_links::TaskLink> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/tasks"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_create_opportunity_followup(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::task_links::CreateFollowup,
    ) -> anyhow::Result<crate::crm::task_links::TaskLink> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/followups"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_end_opportunity_task(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        link_id: uuid::Uuid,
        key: &str,
        input: &crate::crm::task_links::EndTaskLink,
    ) -> anyhow::Result<crate::crm::task_links::TaskLink> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/tasks/{link_id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_opportunity_task_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        link_id: uuid::Uuid,
        query: &crate::crm::HistoryQuery,
    ) -> anyhow::Result<crate::crm::task_links::TaskLinkHistory> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!(
                    "/api/v1/workspaces/{workspace}/crm/opportunities/{id}/tasks/{link_id}/history"
                ),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_deal_tasks(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::task_links::TaskLinkQuery,
    ) -> anyhow::Result<crate::crm::task_links::TaskLinkPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/tasks"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_link_deal_task(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::task_links::LinkTask,
    ) -> anyhow::Result<crate::crm::task_links::TaskLink> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/tasks"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_create_deal_followup(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::task_links::CreateFollowup,
    ) -> anyhow::Result<crate::crm::task_links::TaskLink> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/followups"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_end_deal_task(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        link_id: uuid::Uuid,
        key: &str,
        input: &crate::crm::task_links::EndTaskLink,
    ) -> anyhow::Result<crate::crm::task_links::TaskLink> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/tasks/{link_id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_deal_task_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        link_id: uuid::Uuid,
        query: &crate::crm::HistoryQuery,
    ) -> anyhow::Result<crate::crm::task_links::TaskLinkHistory> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/tasks/{link_id}/history"),
            )?
            .query(query),
        )
        .await
    }

    pub async fn tasks_list(
        &self,
        workspace: uuid::Uuid,
        query: &crate::tasks::TaskQuery,
    ) -> anyhow::Result<crate::tasks::TaskPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/tasks"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn tasks_get(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::tasks::Task> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/tasks/{id}"),
        )?)
        .await
    }
    pub async fn tasks_create(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::tasks::TaskInput,
    ) -> anyhow::Result<crate::tasks::Task> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/tasks"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn tasks_edit(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::tasks::EditTask,
    ) -> anyhow::Result<crate::tasks::Task> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/tasks/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn tasks_archive(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::tasks::ArchiveTask,
    ) -> anyhow::Result<crate::tasks::Task> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/tasks/{id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn tasks_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::tasks::TaskHistoryQuery,
    ) -> anyhow::Result<crate::tasks::TaskHistoryPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/tasks/{id}/history"),
            )?
            .query(query),
        )
        .await
    }

    pub async fn crm_opportunity_contacts(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::participants::ParticipantQuery,
    ) -> anyhow::Result<crate::crm::participants::ParticipantPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/contacts"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_add_opportunity_contact(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::participants::NewParticipant,
    ) -> anyhow::Result<crate::crm::participants::Participant> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/contacts"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_opportunity_contact(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        key: &str,
        input: &crate::crm::participants::EditParticipant,
    ) -> anyhow::Result<crate::crm::participants::Participant> {
        self.business(self.authenticated(reqwest::Method::PATCH,&format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/contacts/{association_id}"))?.header("Idempotency-Key",key).json(input)).await
    }
    pub async fn crm_opportunity_contact_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        query: &crate::crm::HistoryQuery,
    ) -> anyhow::Result<crate::crm::participants::ParticipantHistory> {
        self.business(self.authenticated(reqwest::Method::GET,&format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/contacts/{association_id}/history"))?.query(query)).await
    }
    pub async fn crm_opportunity_organizations(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::participants::ParticipantQuery,
    ) -> anyhow::Result<crate::crm::participants::ParticipantPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/organizations"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_add_opportunity_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::participants::NewParticipant,
    ) -> anyhow::Result<crate::crm::participants::Participant> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/organizations"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_opportunity_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        key: &str,
        input: &crate::crm::participants::EditParticipant,
    ) -> anyhow::Result<crate::crm::participants::Participant> {
        self.business(self.authenticated(reqwest::Method::PATCH,&format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/organizations/{association_id}"))?.header("Idempotency-Key",key).json(input)).await
    }
    pub async fn crm_opportunity_organization_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        query: &crate::crm::HistoryQuery,
    ) -> anyhow::Result<crate::crm::participants::ParticipantHistory> {
        self.business(self.authenticated(reqwest::Method::GET,&format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/organizations/{association_id}/history"))?.query(query)).await
    }
    pub async fn crm_deal_contacts(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::participants::ParticipantQuery,
    ) -> anyhow::Result<crate::crm::participants::ParticipantPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/contacts"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_add_deal_contact(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::participants::NewParticipant,
    ) -> anyhow::Result<crate::crm::participants::Participant> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/contacts"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_deal_contact(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        key: &str,
        input: &crate::crm::participants::EditParticipant,
    ) -> anyhow::Result<crate::crm::participants::Participant> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/contacts/{association_id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_deal_contact_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        query: &crate::crm::HistoryQuery,
    ) -> anyhow::Result<crate::crm::participants::ParticipantHistory> {
        self.business(self.authenticated(reqwest::Method::GET,&format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/contacts/{association_id}/history"))?.query(query)).await
    }
    pub async fn crm_deal_organizations(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::participants::ParticipantQuery,
    ) -> anyhow::Result<crate::crm::participants::ParticipantPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/organizations"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_add_deal_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::participants::NewParticipant,
    ) -> anyhow::Result<crate::crm::participants::Participant> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/organizations"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_deal_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        key: &str,
        input: &crate::crm::participants::EditParticipant,
    ) -> anyhow::Result<crate::crm::participants::Participant> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!(
                    "/api/v1/workspaces/{workspace}/crm/deals/{id}/organizations/{association_id}"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_deal_organization_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        query: &crate::crm::HistoryQuery,
    ) -> anyhow::Result<crate::crm::participants::ParticipantHistory> {
        self.business(self.authenticated(reqwest::Method::GET,&format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/organizations/{association_id}/history"))?.query(query)).await
    }

    pub async fn crm_opportunity_tags(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<Vec<crate::tags::Tag>> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/tags"),
        )?)
        .await
    }
    pub async fn crm_set_opportunity_tags(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::tags::SetTags,
    ) -> anyhow::Result<Vec<crate::tags::Tag>> {
        self.business(
            self.authenticated(
                reqwest::Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/tags"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_deal_tags(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<Vec<crate::tags::Tag>> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/tags"),
        )?)
        .await
    }
    pub async fn crm_set_deal_tags(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::tags::SetTags,
    ) -> anyhow::Result<Vec<crate::tags::Tag>> {
        self.business(
            self.authenticated(
                reqwest::Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/tags"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }

    pub async fn crm_pipelines(
        &self,
        workspace: uuid::Uuid,
        query: &crate::crm::configuration::DefinitionQuery,
    ) -> anyhow::Result<crate::crm::configuration::DefinitionPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/pipelines"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_pipeline(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/pipelines/{id}"),
        )?)
        .await
    }
    pub async fn crm_create_pipeline(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::crm::configuration::SaveDefinition,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/pipelines"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_pipeline(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::configuration::SaveDefinition,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/pipelines/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_stages(
        &self,
        workspace: uuid::Uuid,
        query: &crate::crm::configuration::DefinitionQuery,
    ) -> anyhow::Result<crate::crm::configuration::DefinitionPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/stages"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_stage(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/stages/{id}"),
        )?)
        .await
    }
    pub async fn crm_create_stage(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::crm::configuration::SaveDefinition,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/stages"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_stage(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::configuration::SaveDefinition,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/stages/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_categories(
        &self,
        workspace: uuid::Uuid,
        query: &crate::crm::configuration::DefinitionQuery,
    ) -> anyhow::Result<crate::crm::configuration::DefinitionPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/categories"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_category(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/categories/{id}"),
        )?)
        .await
    }
    pub async fn crm_create_category(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::crm::configuration::SaveDefinition,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/categories"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_category(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::configuration::SaveDefinition,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/categories/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_statuses(
        &self,
        workspace: uuid::Uuid,
        query: &crate::crm::configuration::DefinitionQuery,
    ) -> anyhow::Result<crate::crm::configuration::DefinitionPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/statuses"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_status(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/statuses/{id}"),
        )?)
        .await
    }
    pub async fn crm_create_status(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::crm::configuration::SaveDefinition,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/statuses"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_status(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::configuration::SaveDefinition,
    ) -> anyhow::Result<crate::crm::configuration::Definition> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/statuses/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_opportunities(
        &self,
        workspace: uuid::Uuid,
        query: &crate::crm::OpportunityQuery,
    ) -> anyhow::Result<crate::crm::OpportunityPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_opportunity(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::Opportunity> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}"),
        )?)
        .await
    }
    pub async fn crm_create_opportunity(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::crm::OpportunityInput,
    ) -> anyhow::Result<crate::crm::Opportunity> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_opportunity(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::EditOpportunity,
    ) -> anyhow::Result<crate::crm::Opportunity> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_archive_opportunity(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::ArchiveRecord,
    ) -> anyhow::Result<crate::crm::Opportunity> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_opportunity_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::HistoryQuery,
    ) -> anyhow::Result<crate::crm::HistoryPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/opportunities/{id}/history"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_deals(
        &self,
        workspace: uuid::Uuid,
        query: &crate::crm::DealQuery,
    ) -> anyhow::Result<crate::crm::DealPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/deals"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_deal(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::crm::Deal> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}"),
        )?)
        .await
    }
    pub async fn crm_create_deal(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::crm::NewDeal,
    ) -> anyhow::Result<crate::crm::Deal> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/deals"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_edit_deal(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::EditDeal,
    ) -> anyhow::Result<crate::crm::Deal> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_archive_deal(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::crm::ArchiveRecord,
    ) -> anyhow::Result<crate::crm::Deal> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn crm_deal_history(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        query: &crate::crm::HistoryQuery,
    ) -> anyhow::Result<crate::crm::HistoryPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/crm/deals/{id}/history"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn crm_modules(
        &self,
        workspace: uuid::Uuid,
    ) -> anyhow::Result<Vec<crate::modules::Module>> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/modules"),
        )?)
        .await
    }
    pub async fn crm_set_crm(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::modules::SetModule,
    ) -> anyhow::Result<crate::modules::Module> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/modules/crm"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }

    pub async fn access_permissions(
        &self,
        workspace: uuid::Uuid,
    ) -> anyhow::Result<Vec<crate::workspace_access::Permission>> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/access/permissions"),
        )?)
        .await
    }
    pub async fn access_roles(
        &self,
        workspace: uuid::Uuid,
        query: &crate::workspace_access::AccessQuery,
    ) -> anyhow::Result<crate::workspace_access::RolePage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/access/roles"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn access_role(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::workspace_access::Role> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/access/roles/{id}"),
        )?)
        .await
    }
    pub async fn access_create_role(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::workspace_access::SaveRole,
    ) -> anyhow::Result<crate::workspace_access::Role> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/access/roles"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn access_edit_role(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::workspace_access::SaveRole,
    ) -> anyhow::Result<crate::workspace_access::Role> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/access/roles/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn access_members(
        &self,
        workspace: uuid::Uuid,
        query: &crate::workspace_access::AccessQuery,
    ) -> anyhow::Result<crate::workspace_access::MemberPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/access/members"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn access_member(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::workspace_access::Member> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/access/members/{id}"),
        )?)
        .await
    }
    pub async fn access_edit_member(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::workspace_access::EditMember,
    ) -> anyhow::Result<crate::workspace_access::Member> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/access/members/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }

    pub fn new(base_url: &str) -> anyhow::Result<Self> {
        let base_url = reqwest::Url::parse(base_url)?;
        anyhow::ensure!(
            matches!(base_url.scheme(), "http" | "https"),
            "API URL must use HTTP or HTTPS"
        );
        anyhow::ensure!(
            base_url.username().is_empty() && base_url.password().is_none(),
            "Do not put credentials in the API URL"
        );
        Ok(Self {
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(5))
                .build()?,
            base_url,
            token_file: None,
            source_interface: "cli",
        })
    }

    pub fn for_mcp(mut self) -> Self {
        self.source_interface = "mcp";
        self
    }
    pub fn with_token_file(mut self, file: std::path::PathBuf) -> Self {
        self.token_file = Some(file);
        self
    }

    fn authenticated(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        let host = self.base_url.host_str().unwrap_or_default();
        let loopback = host == "localhost"
            || host
                .trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_loopback());
        anyhow::ensure!(
            self.base_url.scheme() == "https" || loopback,
            "Authenticated API calls require HTTPS except on localhost/loopback"
        );
        let file = self.token_file.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Set LOOSKI_ACCESS_TOKEN_FILE to a private session access-token file")
        })?;
        let token = std::fs::read_to_string(file)?;
        Ok(self
            .http
            .request(method, self.base_url.join(path)?)
            .bearer_auth(token.trim())
            .header("x-looski-interface", self.source_interface))
    }

    pub async fn workspaces(&self) -> anyhow::Result<Vec<crate::workspaces::Workspace>> {
        Ok(self
            .authenticated(reqwest::Method::GET, "/api/v1/workspaces")?
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn contacts(
        &self,
        workspace: uuid::Uuid,
        query: &crate::contacts::ListContacts,
    ) -> anyhow::Result<crate::contacts::ContactPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/contacts"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn contact(
        &self,
        workspace: uuid::Uuid,
        contact: uuid::Uuid,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/contacts/{contact}"),
        )?)
        .await
    }
    pub async fn create_contact(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::contacts::ContactInput,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/contacts"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn edit_contact(
        &self,
        workspace: uuid::Uuid,
        contact: uuid::Uuid,
        key: &str,
        input: &crate::contacts::EditContact,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/contacts/{contact}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn archive_contact(
        &self,
        workspace: uuid::Uuid,
        contact: uuid::Uuid,
        key: &str,
        input: &crate::contacts::ArchiveContact,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/contacts/{contact}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn merge_contacts(
        &self,
        workspace: uuid::Uuid,
        survivor: uuid::Uuid,
        key: &str,
        input: &crate::contacts::MergeContacts,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/contacts/{survivor}/merge"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn organizations(
        &self,
        workspace: uuid::Uuid,
        query: &crate::contacts::ListContacts,
    ) -> anyhow::Result<crate::contacts::organizations::OrganizationPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/organizations"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::contacts::organizations::Organization> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/organizations/{id}"),
        )?)
        .await
    }
    pub async fn create_organization(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::contacts::organizations::OrganizationInput,
    ) -> anyhow::Result<crate::contacts::organizations::Organization> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/organizations"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn edit_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::contacts::organizations::EditOrganization,
    ) -> anyhow::Result<crate::contacts::organizations::Organization> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/organizations/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn archive_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::contacts::ArchiveContact,
    ) -> anyhow::Result<crate::contacts::organizations::Organization> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/organizations/{id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn contact_organizations(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<Vec<crate::contacts::organizations::Relationship>> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/contacts/{id}/organizations"),
        )?)
        .await
    }
    pub async fn set_contact_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        organization: uuid::Uuid,
        key: &str,
        input: &crate::contacts::organizations::RelationshipInput,
    ) -> anyhow::Result<crate::contacts::organizations::Relationship> {
        self.business(
            self.authenticated(
                reqwest::Method::PUT,
                &format!(
                    "/api/v1/workspaces/{workspace}/contacts/{id}/organizations/{organization}"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn tags(
        &self,
        workspace: uuid::Uuid,
        query: &crate::tags::ListTags,
    ) -> anyhow::Result<crate::tags::TagPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/tags"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn tag(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::tags::Tag> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/tags/{id}"),
        )?)
        .await
    }
    pub async fn create_tag(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::tags::SaveTag,
    ) -> anyhow::Result<crate::tags::Tag> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/tags"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn edit_tag(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::tags::SaveTag,
    ) -> anyhow::Result<crate::tags::Tag> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/tags/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn contacts_tags(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<Vec<crate::tags::Tag>> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/contacts/{id}/tags"),
        )?)
        .await
    }
    pub async fn set_contacts_tags(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::contacts::tagging::SetTags,
    ) -> anyhow::Result<Vec<crate::tags::Tag>> {
        self.business(
            self.authenticated(
                reqwest::Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/contacts/{id}/tags"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn organizations_tags(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<Vec<crate::tags::Tag>> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/organizations/{id}/tags"),
        )?)
        .await
    }
    pub async fn set_organizations_tags(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::contacts::tagging::SetTags,
    ) -> anyhow::Result<Vec<crate::tags::Tag>> {
        self.business(
            self.authenticated(
                reqwest::Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/organizations/{id}/tags"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn intakes(
        &self,
        workspace: uuid::Uuid,
        input: &crate::contacts::intake::ListIntakes,
    ) -> anyhow::Result<crate::contacts::intake::IntakePage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/contact-intakes"),
            )?
            .query(input),
        )
        .await
    }
    pub async fn intake(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::contacts::intake::Intake> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/contact-intakes/{id}"),
        )?)
        .await
    }
    pub async fn submit_intake(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::contacts::intake::SubmitIntake,
    ) -> anyhow::Result<crate::contacts::intake::Intake> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/contact-intakes"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn resolve_intake(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::contacts::intake::ResolveIntake,
    ) -> anyhow::Result<crate::contacts::intake::Intake> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/contact-intakes/{id}/resolve"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn intake_candidates(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<Vec<crate::contacts::Contact>> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/contact-intakes/{id}/candidates"),
        )?)
        .await
    }
    async fn business<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<T> {
        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let request_id = response
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let error = response.json::<crate::api::ApiError>().await?;
            return Err(ApiFailure {
                status,
                code: error.code,
                message: error.message,
                request_id,
            }
            .into());
        }
        Ok(response.json().await?)
    }

    pub async fn system(&self) -> anyhow::Result<SystemResponse> {
        Ok(self
            .http
            .get(self.base_url.join("/api/v1/system")?)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn authenticated_client_rejects_remote_http_before_reading_credentials() {
        let error = ApiClient::new("http://business.example")
            .unwrap()
            .with_token_file("/nonexistent/private-token".into())
            .workspaces()
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("require HTTPS"));
    }
}
