// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    forms::{
        self, ArchiveForm, Form, FormPage, FormSummaryReport, FormVersion, ListForms, PublicForm,
        PublicSubmission, PublishForm, Receipt, SaveForm, SetAccepting, SetNotifications,
        Submission, SubmissionPage, SubmissionQuery,
        recipes::{
            self, ConfigureRecipe, Recipe, RetryRun, RunPage, RunQuery, RunSummary, SetRecipeActive,
        },
    },
    identity::Identity,
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

#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/forms",operation_id="listForms",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListForms),responses((status=200,body=FormPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<ListForms>,
) -> Result<Json<FormPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(forms::list(&state.db, &actor, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/forms",operation_id="createForm",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveForm,responses((status=201,body=Form),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveForm>,
) -> Result<(StatusCode, Json<Form>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(forms::save(&state.db, &actor, None, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}",operation_id="getForm",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path)),responses((status=200,body=Form),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Form>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(forms::get(&state.db, &actor, id).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}",operation_id="updateForm",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveForm,responses((status=200,body=Form),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveForm>,
) -> Result<Json<Form>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        forms::save(&state.db, &actor, Some(id), key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/publish",operation_id="publishForm",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=PublishForm,responses((status=200,body=Form),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn publish(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<PublishForm>,
) -> Result<Json<Form>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        forms::publish(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/accepting",operation_id="setFormAccepting",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SetAccepting,responses((status=200,body=Form),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn accepting(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SetAccepting>,
) -> Result<Json<Form>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        forms::set_accepting(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/archive",operation_id="archiveForm",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveForm,responses((status=200,body=Form),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveForm>,
) -> Result<Json<Form>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        forms::archive(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/notifications",operation_id="setFormNotifications",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SetNotifications,responses((status=200,body=Form),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn notifications(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SetNotifications>,
) -> Result<Json<Form>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        forms::set_notifications(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/versions",operation_id="listFormVersions",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path)),responses((status=200,body=Vec<FormVersion>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn versions(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<FormVersion>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(forms::versions(&state.db, &actor, id).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/submissions",operation_id="listFormSubmissions",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),SubmissionQuery),responses((status=200,body=SubmissionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn submissions(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<SubmissionQuery>,
) -> Result<Json<SubmissionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        forms::submissions(&state.db, &actor, id, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/submissions/{submission_id}",operation_id="getFormSubmission",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),("submission_id"=Uuid,Path)),responses((status=200,body=Submission),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn submission(
    State(state): State<AppState>,
    Path((workspace, id, submission)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Submission>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        forms::submission(&state.db, &actor, id, submission).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/summary",operation_id="getFormSummary",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path)),responses((status=200,body=FormSummaryReport),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn summary(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<FormSummaryReport>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(forms::summary(&state.db, &actor, id).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe",operation_id="getFormRecipe",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path)),responses((status=200,body=Recipe),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn recipe(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Recipe>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(recipes::get(&state.db, &actor, id).await?))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe",operation_id="configureFormRecipe",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ConfigureRecipe,responses((status=200,body=Recipe),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn configure(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ConfigureRecipe>,
) -> Result<Json<Recipe>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        recipes::configure(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe/activation",operation_id="setFormRecipeActive",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SetRecipeActive,responses((status=200,body=Recipe),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn activation(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SetRecipeActive>,
) -> Result<Json<Recipe>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        recipes::set_active(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe/runs",operation_id="listFormRecipeRuns",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),RunQuery),responses((status=200,body=RunPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn runs(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<RunQuery>,
) -> Result<Json<RunPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(recipes::runs(&state.db, &actor, id, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe/runs/{run_id}/retry",operation_id="retryFormRecipeRun",tag="forms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("form_id"=Uuid,Path),("run_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RetryRun,responses((status=200,body=RunSummary),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn retry(
    State(state): State<AppState>,
    Path((workspace, id, run)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<RetryRun>,
) -> Result<Json<RunSummary>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        recipes::retry(&state.db, &actor, id, run, key(&headers)?, input).await?,
    ))
}

/// Public reads and submissions are throttled per client before any work.
fn admit(
    state: &AppState,
    headers: &HeaderMap,
    bucket: &'static str,
    limit: u32,
) -> Result<(), Failure> {
    let client = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .map(str::trim)
        .unwrap_or("direct");
    if state.public.admit(bucket, client, limit) {
        Ok(())
    } else {
        Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError {
                code: "throttled".into(),
                message: "Too many requests; try again in a minute".into(),
            }),
        ))
    }
}
#[utoipa::path(get,path="/api/v1/public/forms/{public_key}",operation_id="getPublicForm",tag="forms",params(("public_key"=String,Path)),responses((status=200,body=PublicForm),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn public_form(
    State(state): State<AppState>,
    Path(key): Path<String>,
    headers: HeaderMap,
) -> Result<Json<PublicForm>, Failure> {
    admit(&state, &headers, "forms.read", 120)?;
    Ok(Json(forms::public_form(&state.db, &key).await?))
}
#[utoipa::path(post,path="/api/v1/public/forms/{public_key}/submissions",operation_id="submitPublicForm",tag="forms",params(("public_key"=String,Path)),request_body=PublicSubmission,responses((status=201,body=Receipt),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn public_submit(
    State(state): State<AppState>,
    Path(key): Path<String>,
    headers: HeaderMap,
    Json(input): Json<PublicSubmission>,
) -> Result<(StatusCode, Json<Receipt>), Failure> {
    admit(&state, &headers, "forms.submit", 20)?;
    Ok((
        StatusCode::CREATED,
        Json(forms::public_submit(&state.db, &key, input).await?),
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get, update))
        .routes(routes!(publish))
        .routes(routes!(accepting))
        .routes(routes!(archive))
        .routes(routes!(notifications))
        .routes(routes!(versions))
        .routes(routes!(submissions))
        .routes(routes!(submission))
        .routes(routes!(summary))
        .routes(routes!(recipe, configure))
        .routes(routes!(activation))
        .routes(routes!(runs))
        .routes(routes!(retry))
        .routes(routes!(public_form))
        .routes(routes!(public_submit))
}
