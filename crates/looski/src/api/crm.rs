// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::ActorContext,
    crm::{
        self, ArchiveRecord, Deal, DealPage, DealQuery, EditDeal, EditOpportunity, HistoryPage,
        HistoryQuery, NewDeal, Opportunity, OpportunityInput, OpportunityPage, OpportunityQuery,
        configuration,
    },
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
type Failure = (StatusCode, Json<ApiError>);
fn key(headers: &HeaderMap) -> Result<&str, crate::application::OperationError> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(crate::application::OperationError::Invalid(
            "Idempotency-Key is required for mutations",
        ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/pipelines",operation_id="crm_pipelines",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),configuration::DefinitionQuery),responses((status=200,body=configuration::DefinitionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn pipelines(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<configuration::DefinitionQuery>,
) -> Result<Json<configuration::DefinitionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::list(&state.db, &actor, configuration::Kind::Pipeline, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/pipelines/{id}",operation_id="crm_pipeline",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn pipeline(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<configuration::Definition>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::get(&state.db, &actor, configuration::Kind::Pipeline, id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/pipelines",operation_id="crm_create_pipeline",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=configuration::SaveDefinition,responses((status=201,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_pipeline(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<configuration::SaveDefinition>,
) -> Result<(StatusCode, Json<configuration::Definition>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(
            configuration::save(
                &state.db,
                &actor,
                configuration::Kind::Pipeline,
                None,
                key(&headers)?,
                input,
            )
            .await?,
        ),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/pipelines/{id}",operation_id="crm_edit_pipeline",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=configuration::SaveDefinition,responses((status=200,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit_pipeline(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<configuration::SaveDefinition>,
) -> Result<Json<configuration::Definition>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::save(
            &state.db,
            &actor,
            configuration::Kind::Pipeline,
            Some(id),
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/stages",operation_id="crm_stages",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),configuration::DefinitionQuery),responses((status=200,body=configuration::DefinitionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn stages(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<configuration::DefinitionQuery>,
) -> Result<Json<configuration::DefinitionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::list(&state.db, &actor, configuration::Kind::Stage, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/stages/{id}",operation_id="crm_stage",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn stage(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<configuration::Definition>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::get(&state.db, &actor, configuration::Kind::Stage, id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/stages",operation_id="crm_create_stage",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=configuration::SaveDefinition,responses((status=201,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_stage(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<configuration::SaveDefinition>,
) -> Result<(StatusCode, Json<configuration::Definition>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(
            configuration::save(
                &state.db,
                &actor,
                configuration::Kind::Stage,
                None,
                key(&headers)?,
                input,
            )
            .await?,
        ),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/stages/{id}",operation_id="crm_edit_stage",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=configuration::SaveDefinition,responses((status=200,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit_stage(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<configuration::SaveDefinition>,
) -> Result<Json<configuration::Definition>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::save(
            &state.db,
            &actor,
            configuration::Kind::Stage,
            Some(id),
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/categories",operation_id="crm_categories",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),configuration::DefinitionQuery),responses((status=200,body=configuration::DefinitionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn categories(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<configuration::DefinitionQuery>,
) -> Result<Json<configuration::DefinitionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::list(&state.db, &actor, configuration::Kind::Category, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/categories/{id}",operation_id="crm_category",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn category(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<configuration::Definition>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::get(&state.db, &actor, configuration::Kind::Category, id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/categories",operation_id="crm_create_category",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=configuration::SaveDefinition,responses((status=201,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_category(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<configuration::SaveDefinition>,
) -> Result<(StatusCode, Json<configuration::Definition>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(
            configuration::save(
                &state.db,
                &actor,
                configuration::Kind::Category,
                None,
                key(&headers)?,
                input,
            )
            .await?,
        ),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/categories/{id}",operation_id="crm_edit_category",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=configuration::SaveDefinition,responses((status=200,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit_category(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<configuration::SaveDefinition>,
) -> Result<Json<configuration::Definition>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::save(
            &state.db,
            &actor,
            configuration::Kind::Category,
            Some(id),
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/statuses",operation_id="crm_statuses",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),configuration::DefinitionQuery),responses((status=200,body=configuration::DefinitionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn statuses(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<configuration::DefinitionQuery>,
) -> Result<Json<configuration::DefinitionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::list(&state.db, &actor, configuration::Kind::Status, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/statuses/{id}",operation_id="crm_status",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn status(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<configuration::Definition>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::get(&state.db, &actor, configuration::Kind::Status, id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/statuses",operation_id="crm_create_status",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=configuration::SaveDefinition,responses((status=201,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_status(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<configuration::SaveDefinition>,
) -> Result<(StatusCode, Json<configuration::Definition>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(
            configuration::save(
                &state.db,
                &actor,
                configuration::Kind::Status,
                None,
                key(&headers)?,
                input,
            )
            .await?,
        ),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/statuses/{id}",operation_id="crm_edit_status",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=configuration::SaveDefinition,responses((status=200,body=configuration::Definition),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit_status(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<configuration::SaveDefinition>,
) -> Result<Json<configuration::Definition>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        configuration::save(
            &state.db,
            &actor,
            configuration::Kind::Status,
            Some(id),
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/opportunities",operation_id="crm_opportunities",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),OpportunityQuery),responses((status=200,body=OpportunityPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn opportunities(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<OpportunityQuery>,
) -> Result<Json<OpportunityPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(crm::opportunities(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}",operation_id="crm_opportunity",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=Opportunity),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn opportunity(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Opportunity>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(crm::opportunity(&state.db, &actor, id).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/opportunities",operation_id="crm_create_opportunity",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=OpportunityInput,responses((status=201,body=Opportunity),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_opportunity(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<OpportunityInput>,
) -> Result<(StatusCode, Json<Opportunity>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(crm::save_opportunity(&state.db, &actor, None, None, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}",operation_id="crm_edit_opportunity",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=EditOpportunity,responses((status=200,body=Opportunity),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit_opportunity(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<EditOpportunity>,
) -> Result<Json<Opportunity>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::save_opportunity(
            &state.db,
            &actor,
            Some(id),
            Some(input.expected_version),
            key(&headers)?,
            input.input,
        )
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/archive",operation_id="crm_archive_opportunity",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveRecord,responses((status=200,body=Opportunity),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive_opportunity(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveRecord>,
) -> Result<Json<Opportunity>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::archive_opportunity(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/history",operation_id="crm_opportunity_history",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),HistoryQuery),responses((status=200,body=HistoryPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn opportunity_history(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<HistoryPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::history(&state.db, &actor, id, false, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/deals",operation_id="crm_deals",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),DealQuery),responses((status=200,body=DealPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn deals(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<DealQuery>,
) -> Result<Json<DealPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(crm::deals(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}",operation_id="crm_deal",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=Deal),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn deal(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Deal>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(crm::deal(&state.db, &actor, id).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/deals",operation_id="crm_create_deal",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=NewDeal,responses((status=201,body=Deal),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_deal(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<NewDeal>,
) -> Result<(StatusCode, Json<Deal>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(
            crm::save_deal(
                &state.db,
                &actor,
                None,
                None,
                input.originating_opportunity_id,
                key(&headers)?,
                input.input,
            )
            .await?,
        ),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}",operation_id="crm_edit_deal",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=EditDeal,responses((status=200,body=Deal),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit_deal(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<EditDeal>,
) -> Result<Json<Deal>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::save_deal(
            &state.db,
            &actor,
            Some(id),
            Some(input.expected_version),
            None,
            key(&headers)?,
            input.input,
        )
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/archive",operation_id="crm_archive_deal",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveRecord,responses((status=200,body=Deal),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive_deal(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveRecord>,
) -> Result<Json<Deal>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::archive_deal(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/history",operation_id="crm_deal_history",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),HistoryQuery),responses((status=200,body=HistoryPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn deal_history(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<HistoryPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::history(&state.db, &actor, id, true, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/modules",operation_id="crm_modules",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),responses((status=200,body=Vec<crate::modules::Module>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn modules(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<crate::modules::Module>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(crate::modules::list(&state.db, &actor).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/modules/crm",operation_id="crm_set_crm",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=crate::modules::SetModule,responses((status=200,body=crate::modules::Module),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn set_crm(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<crate::modules::SetModule>,
) -> Result<Json<crate::modules::Module>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crate::modules::set(&state.db, &actor, "crm", key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tags",operation_id="crm_opportunity_tags",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=Vec<crate::tags::Tag>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn opportunity_tags(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<crate::tags::Tag>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::tagging::list(&state.db, &actor, crm::tagging::Target::Opportunity, id).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tags",operation_id="crm_set_opportunity_tags",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=crate::tags::SetTags,responses((status=200,body=Vec<crate::tags::Tag>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn set_opportunity_tags(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<crate::tags::SetTags>,
) -> Result<Json<Vec<crate::tags::Tag>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::tagging::set(
            &state.db,
            &actor,
            crm::tagging::Target::Opportunity,
            id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tags",operation_id="crm_deal_tags",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=Vec<crate::tags::Tag>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn deal_tags(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<crate::tags::Tag>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::tagging::list(&state.db, &actor, crm::tagging::Target::Deal, id).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tags",operation_id="crm_set_deal_tags",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=crate::tags::SetTags,responses((status=200,body=Vec<crate::tags::Tag>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn set_deal_tags(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<crate::tags::SetTags>,
) -> Result<Json<Vec<crate::tags::Tag>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        crm::tagging::set(
            &state.db,
            &actor,
            crm::tagging::Target::Deal,
            id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}

pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(opportunity_tags, set_opportunity_tags))
        .routes(routes!(deal_tags, set_deal_tags))
        .routes(routes!(pipelines, create_pipeline))
        .routes(routes!(pipeline, edit_pipeline))
        .routes(routes!(stages, create_stage))
        .routes(routes!(stage, edit_stage))
        .routes(routes!(categories, create_category))
        .routes(routes!(category, edit_category))
        .routes(routes!(statuses, create_status))
        .routes(routes!(status, edit_status))
        .routes(routes!(opportunities, create_opportunity))
        .routes(routes!(opportunity, edit_opportunity))
        .routes(routes!(archive_opportunity))
        .routes(routes!(opportunity_history))
        .routes(routes!(deals, create_deal))
        .routes(routes!(deal, edit_deal))
        .routes(routes!(archive_deal))
        .routes(routes!(deal_history))
        .routes(routes!(modules))
        .routes(routes!(set_crm))
}
