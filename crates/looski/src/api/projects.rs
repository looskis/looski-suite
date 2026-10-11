// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    identity::Identity,
    projects::{
        self, AddCard, AddMember, ArchiveProject, Board, Card, CardPage, CardQuery, Column,
        Comment, CommentPage, Delivery, DeliveryQuery, EditComment, ListProjects, MoveCard, MyWork,
        MyWorkQuery, NewComment, PageQuery, Project, ProjectMember, ProjectPage, RemoveCard,
        RemoveColumn, RemoveComment, SaveColumn, SaveProject, ScheduleCard,
        recipes::{
            self, DealConversion, DealConversionPage, DealConversionQuery, DealRecipe,
            DealRecipePage, RetryDealConversion, SaveDealRecipe, SetDealRecipeActive,
        },
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
fn key(headers: &HeaderMap) -> Result<&str, OperationError> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(OperationError::Invalid(
            "Idempotency-Key is required for mutations",
        ))
}
fn actor<'a>(
    identity: &'a Identity,
    workspace: Uuid,
    request_id: RequestId,
    headers: &HeaderMap,
) -> ActorContext<'a> {
    ActorContext {
        identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(headers),
    }
}

#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects",operation_id="listProjects",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListProjects),responses((status=200,body=ProjectPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list_projects(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<ListProjects>,
) -> Result<Json<ProjectPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(projects::list(&state.db, &actor, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects",operation_id="createProject",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveProject,responses((status=201,body=Project),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_project(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveProject>,
) -> Result<(StatusCode, Json<Project>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(projects::save(&state.db, &actor, None, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/my-work",operation_id="getMyProjectWork",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),MyWorkQuery),responses((status=200,body=MyWork),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn my_project_work(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<MyWorkQuery>,
) -> Result<Json<MyWork>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(projects::my_work(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/directory",operation_id="listProjectDirectory",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),responses((status=200,body=Vec<ProjectMember>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_directory(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<ProjectMember>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(projects::directory(&state.db, &actor).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/recipes",operation_id="listProjectRecipes",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),DealConversionQuery),responses((status=200,body=DealRecipePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_recipes(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<DealConversionQuery>,
) -> Result<Json<DealRecipePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(recipes::list(&state.db, &actor, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/recipes",operation_id="createProjectRecipe",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveDealRecipe,responses((status=201,body=DealRecipe),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_project_recipe(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveDealRecipe>,
) -> Result<(StatusCode, Json<DealRecipe>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(recipes::save(&state.db, &actor, None, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}",operation_id="getProjectRecipe",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("recipe_id"=Uuid,Path)),responses((status=200,body=DealRecipe),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_recipe(
    State(state): State<AppState>,
    Path((workspace, recipe_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<DealRecipe>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(recipes::get(&state.db, &actor, recipe_id).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}",operation_id="updateProjectRecipe",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("recipe_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveDealRecipe,responses((status=200,body=DealRecipe),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_project_recipe(
    State(state): State<AppState>,
    Path((workspace, recipe_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveDealRecipe>,
) -> Result<Json<DealRecipe>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        recipes::save(&state.db, &actor, Some(recipe_id), key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}/active",operation_id="setProjectRecipeActive",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("recipe_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SetDealRecipeActive,responses((status=200,body=DealRecipe),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn set_project_recipe_active(
    State(state): State<AppState>,
    Path((workspace, recipe_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SetDealRecipeActive>,
) -> Result<Json<DealRecipe>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        recipes::set_active(&state.db, &actor, recipe_id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}/runs",operation_id="listProjectRecipeRuns",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("recipe_id"=Uuid,Path),DealConversionQuery),responses((status=200,body=DealConversionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_recipe_runs(
    State(state): State<AppState>,
    Path((workspace, recipe_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<DealConversionQuery>,
) -> Result<Json<DealConversionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        recipes::runs(&state.db, &actor, recipe_id, query).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}/runs/{run_id}/retry",operation_id="retryProjectRecipeRun",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("recipe_id"=Uuid,Path),("run_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RetryDealConversion,responses((status=200,body=DealConversion),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn retry_project_recipe_run(
    State(state): State<AppState>,
    Path((workspace, recipe_id, run_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<RetryDealConversion>,
) -> Result<Json<DealConversion>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        recipes::retry(&state.db, &actor, recipe_id, run_id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}",operation_id="getProject",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path)),responses((status=200,body=Project),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Project>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(projects::get(&state.db, &actor, project_id).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}",operation_id="updateProject",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveProject,responses((status=200,body=Project),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_project(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveProject>,
) -> Result<Json<Project>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::save(&state.db, &actor, Some(project_id), key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/archive",operation_id="archiveProject",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveProject,responses((status=200,body=Project),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive_project(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveProject>,
) -> Result<Json<Project>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::archive(&state.db, &actor, project_id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/members",operation_id="listProjectMembers",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path)),responses((status=200,body=Vec<ProjectMember>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_members(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<ProjectMember>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::members(&state.db, &actor, project_id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/members",operation_id="addProjectMember",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=AddMember,responses((status=201,body=ProjectMember),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn add_project_member(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<AddMember>,
) -> Result<(StatusCode, Json<ProjectMember>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(projects::add_member(&state.db, &actor, project_id, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/members/{membership_id}/remove",operation_id="removeProjectMember",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("membership_id"=Uuid,Path),("Idempotency-Key"=String,Header)),responses((status=200,body=ProjectMember),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn remove_project_member(
    State(state): State<AppState>,
    Path((workspace, project_id, membership_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<ProjectMember>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::remove_member(&state.db, &actor, project_id, membership_id, key(&headers)?)
            .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/columns",operation_id="listProjectColumns",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path)),responses((status=200,body=Vec<Column>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_columns(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<Column>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::columns(&state.db, &actor, project_id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/columns",operation_id="createProjectColumn",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveColumn,responses((status=201,body=Column),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_project_column(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveColumn>,
) -> Result<(StatusCode, Json<Column>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(
            projects::save_column(&state.db, &actor, project_id, None, key(&headers)?, input)
                .await?,
        ),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/columns/{column_id}",operation_id="updateProjectColumn",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("column_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveColumn,responses((status=200,body=Column),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_project_column(
    State(state): State<AppState>,
    Path((workspace, project_id, column_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveColumn>,
) -> Result<Json<Column>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::save_column(
            &state.db,
            &actor,
            project_id,
            Some(column_id),
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/columns/{column_id}/remove",operation_id="removeProjectColumn",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("column_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RemoveColumn,responses((status=200,body=Column),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn remove_project_column(
    State(state): State<AppState>,
    Path((workspace, project_id, column_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<RemoveColumn>,
) -> Result<Json<Column>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::remove_column(
            &state.db,
            &actor,
            project_id,
            column_id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/board",operation_id="getProjectBoard",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path)),responses((status=200,body=Board),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_board(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Board>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(projects::board(&state.db, &actor, project_id).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/delivery",operation_id="getProjectDelivery",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),DeliveryQuery),responses((status=200,body=Delivery),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_delivery(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<DeliveryQuery>,
) -> Result<Json<Delivery>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::delivery(&state.db, &actor, project_id, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards",operation_id="listProjectCards",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),CardQuery),responses((status=200,body=CardPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_cards(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<CardQuery>,
) -> Result<Json<CardPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::cards(&state.db, &actor, project_id, query).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards",operation_id="addProjectCard",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=AddCard,responses((status=201,body=Card),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn add_project_card(
    State(state): State<AppState>,
    Path((workspace, project_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<AddCard>,
) -> Result<(StatusCode, Json<Card>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(projects::add_card(&state.db, &actor, project_id, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}",operation_id="getProjectCard",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("card_id"=Uuid,Path)),responses((status=200,body=Card),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_card(
    State(state): State<AppState>,
    Path((workspace, project_id, card_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Card>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::card(&state.db, &actor, project_id, card_id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/move",operation_id="moveProjectCard",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("card_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=MoveCard,responses((status=200,body=Card),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn move_project_card(
    State(state): State<AppState>,
    Path((workspace, project_id, card_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<MoveCard>,
) -> Result<Json<Card>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::move_card(
            &state.db,
            &actor,
            project_id,
            card_id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/schedule",operation_id="scheduleProjectCard",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("card_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ScheduleCard,responses((status=200,body=Card),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn schedule_project_card(
    State(state): State<AppState>,
    Path((workspace, project_id, card_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ScheduleCard>,
) -> Result<Json<Card>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::schedule_card(
            &state.db,
            &actor,
            project_id,
            card_id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/remove",operation_id="removeProjectCard",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("card_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RemoveCard,responses((status=200,body=Card),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn remove_project_card(
    State(state): State<AppState>,
    Path((workspace, project_id, card_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<RemoveCard>,
) -> Result<Json<Card>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::remove_card(
            &state.db,
            &actor,
            project_id,
            card_id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/comments",operation_id="listProjectComments",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("card_id"=Uuid,Path),PageQuery),responses((status=200,body=CommentPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn project_comments(
    State(state): State<AppState>,
    Path((workspace, project_id, card_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<PageQuery>,
) -> Result<Json<CommentPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::comments(&state.db, &actor, project_id, card_id, query).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/comments",operation_id="addProjectComment",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("card_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=NewComment,responses((status=201,body=Comment),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn add_project_comment(
    State(state): State<AppState>,
    Path((workspace, project_id, card_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<NewComment>,
) -> Result<(StatusCode, Json<Comment>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(
            projects::add_comment(
                &state.db,
                &actor,
                project_id,
                card_id,
                key(&headers)?,
                input,
            )
            .await?,
        ),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/comments/{comment_id}",operation_id="editProjectComment",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("comment_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=EditComment,responses((status=200,body=Comment),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit_project_comment(
    State(state): State<AppState>,
    Path((workspace, project_id, comment_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<EditComment>,
) -> Result<Json<Comment>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::edit_comment(
            &state.db,
            &actor,
            project_id,
            comment_id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/projects/{project_id}/comments/{comment_id}/remove",operation_id="removeProjectComment",tag="projects",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("project_id"=Uuid,Path),("comment_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RemoveComment,responses((status=200,body=Comment),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn remove_project_comment(
    State(state): State<AppState>,
    Path((workspace, project_id, comment_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<RemoveComment>,
) -> Result<Json<Comment>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        projects::remove_comment(
            &state.db,
            &actor,
            project_id,
            comment_id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_projects, create_project))
        .routes(routes!(my_project_work))
        .routes(routes!(project_directory))
        .routes(routes!(project_recipes, create_project_recipe))
        .routes(routes!(project_recipe, update_project_recipe))
        .routes(routes!(set_project_recipe_active))
        .routes(routes!(project_recipe_runs))
        .routes(routes!(retry_project_recipe_run))
        .routes(routes!(project, update_project))
        .routes(routes!(archive_project))
        .routes(routes!(project_members, add_project_member))
        .routes(routes!(remove_project_member))
        .routes(routes!(project_columns, create_project_column))
        .routes(routes!(update_project_column))
        .routes(routes!(remove_project_column))
        .routes(routes!(project_board))
        .routes(routes!(project_delivery))
        .routes(routes!(project_cards, add_project_card))
        .routes(routes!(project_card))
        .routes(routes!(move_project_card))
        .routes(routes!(schedule_project_card))
        .routes(routes!(remove_project_card))
        .routes(routes!(project_comments, add_project_comment))
        .routes(routes!(edit_project_comment))
        .routes(routes!(remove_project_comment))
}
