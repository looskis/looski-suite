// SPDX-License-Identifier: AGPL-3.0-only
use crate::client::ApiClient;
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};

use rmcp::handler::server::wrapper::Parameters;
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize, JsonSchema)]
struct ContactListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: crate::contacts::ListContacts,
}
#[derive(Deserialize, JsonSchema)]
struct ContactGetArgs {
    workspace_id: Uuid,
    contact_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
struct ContactCreateArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    contact: crate::contacts::ContactInput,
}
#[derive(Deserialize, JsonSchema)]
struct ContactEditArgs {
    workspace_id: Uuid,
    contact_id: Uuid,
    idempotency_key: String,
    edit: crate::contacts::EditContact,
}
#[derive(Deserialize, JsonSchema)]
struct ContactArchiveArgs {
    workspace_id: Uuid,
    contact_id: Uuid,
    idempotency_key: String,
    change: crate::contacts::ArchiveContact,
}
#[derive(Deserialize, JsonSchema)]
struct ContactMergeArgs {
    workspace_id: Uuid,
    survivor_id: Uuid,
    idempotency_key: String,
    merge: crate::contacts::MergeContacts,
}
#[derive(Deserialize, JsonSchema)]
struct OrganizationGetArgs {
    workspace_id: Uuid,
    organization_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
struct OrganizationCreateArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    organization: crate::contacts::organizations::OrganizationInput,
}
#[derive(Deserialize, JsonSchema)]
struct OrganizationEditArgs {
    workspace_id: Uuid,
    organization_id: Uuid,
    idempotency_key: String,
    edit: crate::contacts::organizations::EditOrganization,
}
#[derive(Deserialize, JsonSchema)]
struct OrganizationArchiveArgs {
    workspace_id: Uuid,
    organization_id: Uuid,
    idempotency_key: String,
    change: crate::contacts::ArchiveContact,
}
#[derive(Deserialize, JsonSchema)]
struct RelationshipArgs {
    workspace_id: Uuid,
    contact_id: Uuid,
    organization_id: Uuid,
    idempotency_key: String,
    relationship: crate::contacts::organizations::RelationshipInput,
}
#[derive(Deserialize, JsonSchema)]
struct TagListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: crate::tags::ListTags,
}
#[derive(Deserialize, JsonSchema)]
struct TagGetArgs {
    workspace_id: Uuid,
    tag_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
struct TagCreateArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    tag: crate::tags::SaveTag,
}
#[derive(Deserialize, JsonSchema)]
struct TagEditArgs {
    workspace_id: Uuid,
    tag_id: Uuid,
    idempotency_key: String,
    tag: crate::tags::SaveTag,
}
#[derive(Deserialize, JsonSchema)]
struct ContactTagsArgs {
    workspace_id: Uuid,
    contact_id: Uuid,
    idempotency_key: String,
    change: crate::contacts::tagging::SetTags,
}
#[derive(Deserialize, JsonSchema)]
struct OrganizationTagsArgs {
    workspace_id: Uuid,
    organization_id: Uuid,
    idempotency_key: String,
    change: crate::contacts::tagging::SetTags,
}
#[derive(Deserialize, JsonSchema)]
struct IntakeListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: crate::contacts::intake::ListIntakes,
}
#[derive(Deserialize, JsonSchema)]
struct IntakeGetArgs {
    workspace_id: Uuid,
    intake_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
struct IntakeSubmitArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    intake: crate::contacts::intake::SubmitIntake,
}
#[derive(Deserialize, JsonSchema)]
struct IntakeResolveArgs {
    workspace_id: Uuid,
    intake_id: Uuid,
    idempotency_key: String,
    review: crate::contacts::intake::ResolveIntake,
}
fn business_result<T: serde::Serialize>(
    result: anyhow::Result<T>,
) -> Result<CallToolResult, ErrorData> {
    match result {
        Ok(value) => Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string(&value)
                .map_err(|_| ErrorData::internal_error("Could not encode result", None))?,
        )])),
        Err(error) => {
            let mut result = CallToolResult::error(vec![ContentBlock::text(error.to_string())]);
            if let Some(failure) = error.downcast_ref::<crate::client::ApiFailure>() {
                result.structured_content =
                    Some(serde_json::to_value(failure).map_err(|_| {
                        ErrorData::internal_error("Could not encode failure", None)
                    })?);
            }
            Ok(result)
        }
    }
}

#[derive(Clone)]
pub struct McpServer {
    client: ApiClient,
    tool_router: ToolRouter<Self>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AccessPermissionsArgs {
    workspace_id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AccessRolesArgs {
    workspace_id: Uuid,
    #[serde(default)]
    query: crate::workspace_access::AccessQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AccessRoleArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AccessCreateRoleArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: crate::workspace_access::SaveRole,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AccessEditRoleArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::workspace_access::SaveRole,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AccessMembersArgs {
    workspace_id: Uuid,
    #[serde(default)]
    query: crate::workspace_access::AccessQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AccessMemberArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AccessEditMemberArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::workspace_access::EditMember,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmPipelinesArgs {
    workspace_id: Uuid,
    #[serde(default)]
    query: crate::crm::configuration::DefinitionQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmPipelineArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmCreatePipelineArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: crate::crm::configuration::SaveDefinition,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditPipelineArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::configuration::SaveDefinition,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmStagesArgs {
    workspace_id: Uuid,
    #[serde(default)]
    query: crate::crm::configuration::DefinitionQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmStageArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmCreateStageArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: crate::crm::configuration::SaveDefinition,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditStageArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::configuration::SaveDefinition,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmCategoriesArgs {
    workspace_id: Uuid,
    #[serde(default)]
    query: crate::crm::configuration::DefinitionQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmCategoryArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmCreateCategoryArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: crate::crm::configuration::SaveDefinition,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditCategoryArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::configuration::SaveDefinition,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmStatusesArgs {
    workspace_id: Uuid,
    #[serde(default)]
    query: crate::crm::configuration::DefinitionQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmStatusArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmCreateStatusArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: crate::crm::configuration::SaveDefinition,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditStatusArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::configuration::SaveDefinition,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmOpportunitiesArgs {
    workspace_id: Uuid,
    #[serde(default)]
    query: crate::crm::OpportunityQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmOpportunityArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmCreateOpportunityArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: crate::crm::OpportunityInput,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditOpportunityArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::EditOpportunity,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmArchiveOpportunityArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::ArchiveRecord,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmOpportunityHistoryArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(default)]
    query: crate::crm::HistoryQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmDealsArgs {
    workspace_id: Uuid,
    #[serde(default)]
    query: crate::crm::DealQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmDealArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmCreateDealArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: crate::crm::NewDeal,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditDealArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::EditDeal,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmArchiveDealArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::ArchiveRecord,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmDealHistoryArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(default)]
    query: crate::crm::HistoryQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmModulesArgs {
    workspace_id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmSetCrmArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: crate::modules::SetModule,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmOpportunityTagsArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmSetOpportunityTagsArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::tags::SetTags,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmDealTagsArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmSetDealTagsArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::tags::SetTags,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmOpportunityContactsArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(default)]
    query: crate::crm::participants::ParticipantQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmAddOpportunityContactArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::participants::NewParticipant,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditOpportunityContactArgs {
    workspace_id: Uuid,
    id: Uuid,
    association_id: Uuid,
    idempotency_key: String,
    change: crate::crm::participants::EditParticipant,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmOpportunityContactHistoryArgs {
    workspace_id: Uuid,
    id: Uuid,
    association_id: Uuid,
    #[serde(default)]
    query: crate::crm::HistoryQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmOpportunityOrganizationsArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(default)]
    query: crate::crm::participants::ParticipantQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmAddOpportunityOrganizationArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::participants::NewParticipant,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditOpportunityOrganizationArgs {
    workspace_id: Uuid,
    id: Uuid,
    association_id: Uuid,
    idempotency_key: String,
    change: crate::crm::participants::EditParticipant,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmOpportunityOrganizationHistoryArgs {
    workspace_id: Uuid,
    id: Uuid,
    association_id: Uuid,
    #[serde(default)]
    query: crate::crm::HistoryQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmDealContactsArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(default)]
    query: crate::crm::participants::ParticipantQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmAddDealContactArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::participants::NewParticipant,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditDealContactArgs {
    workspace_id: Uuid,
    id: Uuid,
    association_id: Uuid,
    idempotency_key: String,
    change: crate::crm::participants::EditParticipant,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmDealContactHistoryArgs {
    workspace_id: Uuid,
    id: Uuid,
    association_id: Uuid,
    #[serde(default)]
    query: crate::crm::HistoryQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmDealOrganizationsArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(default)]
    query: crate::crm::participants::ParticipantQuery,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmAddDealOrganizationArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::crm::participants::NewParticipant,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmEditDealOrganizationArgs {
    workspace_id: Uuid,
    id: Uuid,
    association_id: Uuid,
    idempotency_key: String,
    change: crate::crm::participants::EditParticipant,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CrmDealOrganizationHistoryArgs {
    workspace_id: Uuid,
    id: Uuid,
    association_id: Uuid,
    #[serde(default)]
    query: crate::crm::HistoryQuery,
}

#[derive(Deserialize, JsonSchema)]
struct TasksListArgs {
    workspace_id: Uuid,
    #[serde(default)]
    query: crate::tasks::TaskQuery,
}
#[derive(Deserialize, JsonSchema)]
struct TasksGetArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
struct TasksCreateArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: crate::tasks::TaskInput,
}
#[derive(Deserialize, JsonSchema)]
struct TasksEditArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::tasks::EditTask,
}
#[derive(Deserialize, JsonSchema)]
struct TasksArchiveArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    change: crate::tasks::ArchiveTask,
}
#[derive(Deserialize, JsonSchema)]
struct TasksHistoryArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(default)]
    query: crate::tasks::TaskHistoryQuery,
}

#[tool_router]
impl McpServer {
    #[tool(
        description = "Tasks list. Requires current task permissions; linked records never grant access."
    )]
    async fn tasks_list(
        &self,
        Parameters(input): Parameters<TasksListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .tasks_list(input.workspace_id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Tasks get. Requires current task permissions; linked records never grant access."
    )]
    async fn tasks_get(
        &self,
        Parameters(input): Parameters<TasksGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.tasks_get(input.workspace_id, input.id).await)
    }
    #[tool(
        description = "Tasks create. Requires current task permissions; linked records never grant access."
    )]
    async fn tasks_create(
        &self,
        Parameters(input): Parameters<TasksCreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .tasks_create(input.workspace_id, &input.idempotency_key, &input.change)
                .await,
        )
    }
    #[tool(
        description = "Tasks edit. Requires current task permissions; linked records never grant access."
    )]
    async fn tasks_edit(
        &self,
        Parameters(input): Parameters<TasksEditArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .tasks_edit(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Tasks archive. Requires current task permissions; linked records never grant access."
    )]
    async fn tasks_archive(
        &self,
        Parameters(input): Parameters<TasksArchiveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .tasks_archive(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Tasks history. Requires current task permissions; linked records never grant access."
    )]
    async fn tasks_history(
        &self,
        Parameters(input): Parameters<TasksHistoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .tasks_history(input.workspace_id, input.id, &input.query)
                .await,
        )
    }

    #[tool(
        description = "Opportunity contacts with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_opportunity_contacts(
        &self,
        Parameters(input): Parameters<CrmOpportunityContactsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_opportunity_contacts(input.workspace_id, input.id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Add opportunity contact with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_add_opportunity_contact(
        &self,
        Parameters(input): Parameters<CrmAddOpportunityContactArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_add_opportunity_contact(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Edit opportunity contact with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_edit_opportunity_contact(
        &self,
        Parameters(input): Parameters<CrmEditOpportunityContactArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_opportunity_contact(
                    input.workspace_id,
                    input.id,
                    input.association_id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Opportunity contact history with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_opportunity_contact_history(
        &self,
        Parameters(input): Parameters<CrmOpportunityContactHistoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_opportunity_contact_history(
                    input.workspace_id,
                    input.id,
                    input.association_id,
                    &input.query,
                )
                .await,
        )
    }
    #[tool(
        description = "Opportunity organizations with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_opportunity_organizations(
        &self,
        Parameters(input): Parameters<CrmOpportunityOrganizationsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_opportunity_organizations(input.workspace_id, input.id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Add opportunity organization with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_add_opportunity_organization(
        &self,
        Parameters(input): Parameters<CrmAddOpportunityOrganizationArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_add_opportunity_organization(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Edit opportunity organization with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_edit_opportunity_organization(
        &self,
        Parameters(input): Parameters<CrmEditOpportunityOrganizationArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_opportunity_organization(
                    input.workspace_id,
                    input.id,
                    input.association_id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Opportunity organization history with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_opportunity_organization_history(
        &self,
        Parameters(input): Parameters<CrmOpportunityOrganizationHistoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_opportunity_organization_history(
                    input.workspace_id,
                    input.id,
                    input.association_id,
                    &input.query,
                )
                .await,
        )
    }
    #[tool(
        description = "Deal contacts with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_deal_contacts(
        &self,
        Parameters(input): Parameters<CrmDealContactsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_deal_contacts(input.workspace_id, input.id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Add deal contact with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_add_deal_contact(
        &self,
        Parameters(input): Parameters<CrmAddDealContactArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_add_deal_contact(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Edit deal contact with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_edit_deal_contact(
        &self,
        Parameters(input): Parameters<CrmEditDealContactArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_deal_contact(
                    input.workspace_id,
                    input.id,
                    input.association_id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Deal contact history with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_deal_contact_history(
        &self,
        Parameters(input): Parameters<CrmDealContactHistoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_deal_contact_history(
                    input.workspace_id,
                    input.id,
                    input.association_id,
                    &input.query,
                )
                .await,
        )
    }
    #[tool(
        description = "Deal organizations with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_deal_organizations(
        &self,
        Parameters(input): Parameters<CrmDealOrganizationsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_deal_organizations(input.workspace_id, input.id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Add deal organization with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_add_deal_organization(
        &self,
        Parameters(input): Parameters<CrmAddDealOrganizationArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_add_deal_organization(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Edit deal organization with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_edit_deal_organization(
        &self,
        Parameters(input): Parameters<CrmEditDealOrganizationArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_deal_organization(
                    input.workspace_id,
                    input.id,
                    input.association_id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Deal organization history with independent CRM and Contacts visibility. Mutations require the parent record version and an idempotency key."
    )]
    async fn crm_deal_organization_history(
        &self,
        Parameters(input): Parameters<CrmDealOrganizationHistoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_deal_organization_history(
                    input.workspace_id,
                    input.id,
                    input.association_id,
                    &input.query,
                )
                .await,
        )
    }

    #[tool(
        description = "Read tags on a permitted opportunity. CRM must be enabled; mutations use expected_version and an idempotency key."
    )]
    async fn crm_opportunity_tags(
        &self,
        Parameters(input): Parameters<CrmOpportunityTagsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_opportunity_tags(input.workspace_id, input.id)
                .await,
        )
    }
    #[tool(
        description = "Set tags on a permitted opportunity. CRM must be enabled; mutations use expected_version and an idempotency key."
    )]
    async fn crm_set_opportunity_tags(
        &self,
        Parameters(input): Parameters<CrmSetOpportunityTagsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_set_opportunity_tags(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Read tags on a permitted deal. CRM must be enabled; mutations use expected_version and an idempotency key."
    )]
    async fn crm_deal_tags(
        &self,
        Parameters(input): Parameters<CrmDealTagsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_deal_tags(input.workspace_id, input.id)
                .await,
        )
    }
    #[tool(
        description = "Set tags on a permitted deal. CRM must be enabled; mutations use expected_version and an idempotency key."
    )]
    async fn crm_set_deal_tags(
        &self,
        Parameters(input): Parameters<CrmSetDealTagsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_set_deal_tags(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }

    #[tool(
        description = "Pipelines in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_pipelines(
        &self,
        Parameters(input): Parameters<CrmPipelinesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_pipelines(input.workspace_id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Pipeline in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_pipeline(
        &self,
        Parameters(input): Parameters<CrmPipelineArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.crm_pipeline(input.workspace_id, input.id).await)
    }
    #[tool(
        description = "Create pipeline in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_create_pipeline(
        &self,
        Parameters(input): Parameters<CrmCreatePipelineArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_create_pipeline(input.workspace_id, &input.idempotency_key, &input.change)
                .await,
        )
    }
    #[tool(
        description = "Edit pipeline in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_edit_pipeline(
        &self,
        Parameters(input): Parameters<CrmEditPipelineArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_pipeline(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Stages in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_stages(
        &self,
        Parameters(input): Parameters<CrmStagesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_stages(input.workspace_id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Stage in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_stage(
        &self,
        Parameters(input): Parameters<CrmStageArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.crm_stage(input.workspace_id, input.id).await)
    }
    #[tool(
        description = "Create stage in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_create_stage(
        &self,
        Parameters(input): Parameters<CrmCreateStageArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_create_stage(input.workspace_id, &input.idempotency_key, &input.change)
                .await,
        )
    }
    #[tool(
        description = "Edit stage in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_edit_stage(
        &self,
        Parameters(input): Parameters<CrmEditStageArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_stage(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Categories in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_categories(
        &self,
        Parameters(input): Parameters<CrmCategoriesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_categories(input.workspace_id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Category in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_category(
        &self,
        Parameters(input): Parameters<CrmCategoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.crm_category(input.workspace_id, input.id).await)
    }
    #[tool(
        description = "Create category in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_create_category(
        &self,
        Parameters(input): Parameters<CrmCreateCategoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_create_category(input.workspace_id, &input.idempotency_key, &input.change)
                .await,
        )
    }
    #[tool(
        description = "Edit category in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_edit_category(
        &self,
        Parameters(input): Parameters<CrmEditCategoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_category(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Statuses in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_statuses(
        &self,
        Parameters(input): Parameters<CrmStatusesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_statuses(input.workspace_id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Status in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_status(
        &self,
        Parameters(input): Parameters<CrmStatusArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.crm_status(input.workspace_id, input.id).await)
    }
    #[tool(
        description = "Create status in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_create_status(
        &self,
        Parameters(input): Parameters<CrmCreateStatusArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_create_status(input.workspace_id, &input.idempotency_key, &input.change)
                .await,
        )
    }
    #[tool(
        description = "Edit status in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_edit_status(
        &self,
        Parameters(input): Parameters<CrmEditStatusArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_status(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Opportunities in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_opportunities(
        &self,
        Parameters(input): Parameters<CrmOpportunitiesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_opportunities(input.workspace_id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Opportunity in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_opportunity(
        &self,
        Parameters(input): Parameters<CrmOpportunityArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_opportunity(input.workspace_id, input.id)
                .await,
        )
    }
    #[tool(
        description = "Create opportunity in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_create_opportunity(
        &self,
        Parameters(input): Parameters<CrmCreateOpportunityArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_create_opportunity(input.workspace_id, &input.idempotency_key, &input.change)
                .await,
        )
    }
    #[tool(
        description = "Edit opportunity in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_edit_opportunity(
        &self,
        Parameters(input): Parameters<CrmEditOpportunityArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_opportunity(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Archive opportunity in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_archive_opportunity(
        &self,
        Parameters(input): Parameters<CrmArchiveOpportunityArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_archive_opportunity(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Opportunity history in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_opportunity_history(
        &self,
        Parameters(input): Parameters<CrmOpportunityHistoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_opportunity_history(input.workspace_id, input.id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Deals in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_deals(
        &self,
        Parameters(input): Parameters<CrmDealsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_deals(input.workspace_id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Deal in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_deal(
        &self,
        Parameters(input): Parameters<CrmDealArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.crm_deal(input.workspace_id, input.id).await)
    }
    #[tool(
        description = "Create deal in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_create_deal(
        &self,
        Parameters(input): Parameters<CrmCreateDealArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_create_deal(input.workspace_id, &input.idempotency_key, &input.change)
                .await,
        )
    }
    #[tool(
        description = "Edit deal in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_edit_deal(
        &self,
        Parameters(input): Parameters<CrmEditDealArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_edit_deal(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Archive deal in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_archive_deal(
        &self,
        Parameters(input): Parameters<CrmArchiveDealArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_archive_deal(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Deal history in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_deal_history(
        &self,
        Parameters(input): Parameters<CrmDealHistoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_deal_history(input.workspace_id, input.id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Modules in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_modules(
        &self,
        Parameters(input): Parameters<CrmModulesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.crm_modules(input.workspace_id).await)
    }
    #[tool(
        description = "Set crm in the permitted workspace. CRM records require an enabled module and current record scope; mutations require an idempotency key."
    )]
    async fn crm_set_crm(
        &self,
        Parameters(input): Parameters<CrmSetCrmArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .crm_set_crm(input.workspace_id, &input.idempotency_key, &input.change)
                .await,
        )
    }

    #[tool(
        description = "List workspace permissions. Mutations require current administrative authority and preserve a final active administrator."
    )]
    async fn list_workspace_permissions(
        &self,
        Parameters(input): Parameters<AccessPermissionsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.access_permissions(input.workspace_id).await)
    }
    #[tool(
        description = "List workspace roles. Mutations require current administrative authority and preserve a final active administrator."
    )]
    async fn list_workspace_roles(
        &self,
        Parameters(input): Parameters<AccessRolesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .access_roles(input.workspace_id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Get workspace role. Mutations require current administrative authority and preserve a final active administrator."
    )]
    async fn get_workspace_role(
        &self,
        Parameters(input): Parameters<AccessRoleArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.access_role(input.workspace_id, input.id).await)
    }
    #[tool(
        description = "Create workspace role. Mutations require current administrative authority and preserve a final active administrator."
    )]
    async fn create_workspace_role(
        &self,
        Parameters(input): Parameters<AccessCreateRoleArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .access_create_role(input.workspace_id, &input.idempotency_key, &input.change)
                .await,
        )
    }
    #[tool(
        description = "Update workspace role. Mutations require current administrative authority and preserve a final active administrator."
    )]
    async fn update_workspace_role(
        &self,
        Parameters(input): Parameters<AccessEditRoleArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .access_edit_role(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(
        description = "List workspace members. Mutations require current administrative authority and preserve a final active administrator."
    )]
    async fn list_workspace_members(
        &self,
        Parameters(input): Parameters<AccessMembersArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .access_members(input.workspace_id, &input.query)
                .await,
        )
    }
    #[tool(
        description = "Get workspace member. Mutations require current administrative authority and preserve a final active administrator."
    )]
    async fn get_workspace_member(
        &self,
        Parameters(input): Parameters<AccessMemberArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .access_member(input.workspace_id, input.id)
                .await,
        )
    }
    #[tool(
        description = "Update workspace member. Mutations require current administrative authority and preserve a final active administrator."
    )]
    async fn update_workspace_member(
        &self,
        Parameters(input): Parameters<AccessEditMemberArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .access_edit_member(
                    input.workspace_id,
                    input.id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }

    #[tool(description = "List scoped intake evidence by state; pending items require review.")]
    async fn list_contact_intakes(
        &self,
        Parameters(input): Parameters<IntakeListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.intakes(input.workspace_id, &input.query).await)
    }
    #[tool(description = "Read permitted original intake evidence and resolution state.")]
    async fn get_contact_intake(
        &self,
        Parameters(input): Parameters<IntakeGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .intake(input.workspace_id, input.intake_id)
                .await,
        )
    }
    #[tool(
        description = "Resolve identifiers without overwriting trusted fields; preserve ambiguous input for review."
    )]
    async fn submit_contact_intake(
        &self,
        Parameters(input): Parameters<IntakeSubmitArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .submit_intake(input.workspace_id, &input.idempotency_key, &input.intake)
                .await,
        )
    }
    #[tool(
        description = "Explicitly link a permitted person, create a reviewed person, or dismiss a pending intake; requires expected version."
    )]
    async fn resolve_contact_intake(
        &self,
        Parameters(input): Parameters<IntakeResolveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .resolve_intake(
                    input.workspace_id,
                    input.intake_id,
                    &input.idempotency_key,
                    &input.review,
                )
                .await,
        )
    }
    #[tool(
        description = "List up to 50 currently permitted candidate contacts for an intake; no hidden candidate counts."
    )]
    async fn list_intake_candidates(
        &self,
        Parameters(input): Parameters<IntakeGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .intake_candidates(input.workspace_id, input.intake_id)
                .await,
        )
    }

    #[tool(description = "List tags within the authorized workspace.")]
    async fn list_tags(
        &self,
        Parameters(input): Parameters<TagListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.tags(input.workspace_id, &input.query).await)
    }
    #[tool(description = "Get tag within the authorized workspace.")]
    async fn get_tag(
        &self,
        Parameters(input): Parameters<TagGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.tag(input.workspace_id, input.tag_id).await)
    }
    #[tool(description = "Create tag within the authorized workspace.")]
    async fn create_tag(
        &self,
        Parameters(input): Parameters<TagCreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_tag(input.workspace_id, &input.idempotency_key, &input.tag)
                .await,
        )
    }
    #[tool(
        description = "Rename, color, archive or restore a tag; requires its expected version. Stable assignments are preserved."
    )]
    async fn update_tag(
        &self,
        Parameters(input): Parameters<TagEditArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .edit_tag(
                    input.workspace_id,
                    input.tag_id,
                    &input.idempotency_key,
                    &input.tag,
                )
                .await,
        )
    }
    #[tool(description = "List contact tags within the authorized workspace.")]
    async fn list_contact_tags(
        &self,
        Parameters(input): Parameters<ContactGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .contacts_tags(input.workspace_id, input.contact_id)
                .await,
        )
    }
    #[tool(
        description = "Replace a contact tag set using its expected version and a retry key; tags do not grant access."
    )]
    async fn set_contact_tags(
        &self,
        Parameters(input): Parameters<ContactTagsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .set_contacts_tags(
                    input.workspace_id,
                    input.contact_id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }
    #[tool(description = "List organization tags within the authorized workspace.")]
    async fn list_organization_tags(
        &self,
        Parameters(input): Parameters<OrganizationGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .organizations_tags(input.workspace_id, input.organization_id)
                .await,
        )
    }
    #[tool(
        description = "Replace organization tags with expected version and a retry key; tags do not grant access."
    )]
    async fn set_organization_tags(
        &self,
        Parameters(input): Parameters<OrganizationTagsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .set_organizations_tags(
                    input.workspace_id,
                    input.organization_id,
                    &input.idempotency_key,
                    &input.change,
                )
                .await,
        )
    }

    #[tool(
        description = "List workspaces allowed by the configured first-party session and current suite permissions."
    )]
    async fn list_workspaces(&self) -> Result<CallToolResult, ErrorData> {
        match self.client.workspaces().await {
            Ok(workspaces) => Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string(&workspaces)
                    .map_err(|_| ErrorData::internal_error("Could not encode workspaces", None))?,
            )])),
            Err(_) => Ok(CallToolResult::error(vec![ContentBlock::text(
                "Workspace access failed. Check the API and the session in LOOSKI_ACCESS_TOKEN_FILE.",
            )])),
        }
    }
    #[tool(
        description = "List permitted workspace contacts with bounded cursor pagination and optional name search."
    )]
    async fn list_contacts(
        &self,
        Parameters(args): Parameters<ContactListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.contacts(args.workspace_id, &args.query).await)
    }
    #[tool(description = "Read a permitted contact by its stable ID.")]
    async fn get_contact(
        &self,
        Parameters(args): Parameters<ContactGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .contact(args.workspace_id, args.contact_id)
                .await,
        )
    }
    #[tool(
        description = "Create a contact. Supply a stable idempotency key for retries; matching identifiers require review. allow_shared requires explicit reviewed sharing and permission."
    )]
    async fn create_contact(
        &self,
        Parameters(args): Parameters<ContactCreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_contact(args.workspace_id, &args.idempotency_key, &args.contact)
                .await,
        )
    }
    #[tool(
        description = "Replace editable contact fields using expected_version and an idempotency key. Omitted identifiers are removed from active association, with history retained."
    )]
    async fn update_contact(
        &self,
        Parameters(args): Parameters<ContactEditArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .edit_contact(
                    args.workspace_id,
                    args.contact_id,
                    &args.idempotency_key,
                    &args.edit,
                )
                .await,
        )
    }
    #[tool(
        description = "Archive or restore a contact with expected_version and an idempotency key. Does not permanently delete records."
    )]
    async fn archive_contact(
        &self,
        Parameters(args): Parameters<ContactArchiveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .archive_contact(
                    args.workspace_id,
                    args.contact_id,
                    &args.idempotency_key,
                    &args.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Merge confirmed duplicate contacts into an explicitly chosen survivor. Requires both record versions and resolved fields; preserves identifiers/history and old-ID aliases. Never use shared email alone as proof of identity."
    )]
    async fn merge_contacts(
        &self,
        Parameters(args): Parameters<ContactMergeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .merge_contacts(
                    args.workspace_id,
                    args.survivor_id,
                    &args.idempotency_key,
                    &args.merge,
                )
                .await,
        )
    }
    #[tool(
        description = "List permitted organizations with bounded cursor pagination and name search."
    )]
    async fn list_organizations(
        &self,
        Parameters(args): Parameters<ContactListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .organizations(args.workspace_id, &args.query)
                .await,
        )
    }
    #[tool(description = "Read a permitted organization by ID.")]
    async fn get_organization(
        &self,
        Parameters(args): Parameters<OrganizationGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .organization(args.workspace_id, args.organization_id)
                .await,
        )
    }
    #[tool(
        description = "Create an organization with an idempotency key. Name or website is not unique identity."
    )]
    async fn create_organization(
        &self,
        Parameters(args): Parameters<OrganizationCreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_organization(args.workspace_id, &args.idempotency_key, &args.organization)
                .await,
        )
    }
    #[tool(
        description = "Replace editable organization fields using expected_version and an idempotency key."
    )]
    async fn update_organization(
        &self,
        Parameters(args): Parameters<OrganizationEditArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .edit_organization(
                    args.workspace_id,
                    args.organization_id,
                    &args.idempotency_key,
                    &args.edit,
                )
                .await,
        )
    }
    #[tool(
        description = "Archive or restore an organization without deleting relationships or history."
    )]
    async fn archive_organization(
        &self,
        Parameters(args): Parameters<OrganizationArchiveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .archive_organization(
                    args.workspace_id,
                    args.organization_id,
                    &args.idempotency_key,
                    &args.change,
                )
                .await,
        )
    }
    #[tool(
        description = "List up to 100 current organization relationships for a permitted contact. Each organization must also be permitted."
    )]
    async fn list_contact_organizations(
        &self,
        Parameters(args): Parameters<ContactGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .contact_organizations(args.workspace_id, args.contact_id)
                .await,
        )
    }
    #[tool(
        description = "Create, replace, or end a contact organization relationship. Requires expected_contact_version and idempotency key; preserves history and at most one primary organization."
    )]
    async fn set_contact_organization(
        &self,
        Parameters(args): Parameters<RelationshipArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .set_contact_organization(
                    args.workspace_id,
                    args.contact_id,
                    args.organization_id,
                    &args.idempotency_key,
                    &args.relationship,
                )
                .await,
        )
    }
    pub fn new(client: ApiClient) -> Self {
        Self {
            client: client.for_mcp(),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Read public Looski installation version and scaffold status. No business data or login access."
    )]
    async fn system_status(&self) -> Result<CallToolResult, ErrorData> {
        match self.client.system().await {
            Ok(info) => Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string(&info)
                    .map_err(|_| ErrorData::internal_error("Could not encode status", None))?,
            )])),
            Err(_) => Ok(CallToolResult::error(vec![ContentBlock::text(
                "Looski API is unavailable",
            )])),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("looski-suite", env!("CARGO_PKG_VERSION")))
            .with_instructions("system_status is public. Business tools use LOOSKI_ACCESS_TOKEN_FILE and the same authorization as the HTTP API. No credentials are accepted as tool arguments.")
    }
}
