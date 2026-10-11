// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    identity::Identity,
    knowledge::{self, *},
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

#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/collections",operation_id="listKnowledgeCollections",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListKnowledgeCollections),responses((status=200,body=KnowledgeCollectionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list_collections(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<ListKnowledgeCollections>,
) -> Result<Json<KnowledgeCollectionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::list_collections(&state.db, &actor, query).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/knowledge/collections",operation_id="createKnowledgeCollection",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveKnowledgeCollection,responses((status=201,body=KnowledgeCollection),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_collection(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveKnowledgeCollection>,
) -> Result<(StatusCode, Json<KnowledgeCollection>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(knowledge::save_collection(&state.db, &actor, None, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}",operation_id="getKnowledgeCollection",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("collection_id"=Uuid,Path)),responses((status=200,body=KnowledgeCollection),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn get_collection(
    State(state): State<AppState>,
    Path((workspace, collection_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<KnowledgeCollection>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::get_collection(&state.db, &actor, collection_id).await?,
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}",operation_id="updateKnowledgeCollection",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("collection_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveKnowledgeCollection,responses((status=200,body=KnowledgeCollection),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_collection(
    State(state): State<AppState>,
    Path((workspace, collection_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveKnowledgeCollection>,
) -> Result<Json<KnowledgeCollection>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::save_collection(
            &state.db,
            &actor,
            Some(collection_id),
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}/archive",operation_id="archiveKnowledgeCollection",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("collection_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveKnowledgeRecord,responses((status=200,body=KnowledgeCollection),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive_collection(
    State(state): State<AppState>,
    Path((workspace, collection_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveKnowledgeRecord>,
) -> Result<Json<KnowledgeCollection>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::archive_collection(&state.db, &actor, collection_id, key(&headers)?, input)
            .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}/members",operation_id="listKnowledgeCollectionMembers",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("collection_id"=Uuid,Path)),responses((status=200,body=Vec<KnowledgeMember>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list_members(
    State(state): State<AppState>,
    Path((workspace, collection_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<KnowledgeMember>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::members(&state.db, &actor, collection_id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}/members",operation_id="addKnowledgeCollectionMember",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("collection_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=AddKnowledgeMember,responses((status=201,body=KnowledgeMember),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn add_member(
    State(state): State<AppState>,
    Path((workspace, collection_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<AddKnowledgeMember>,
) -> Result<(StatusCode, Json<KnowledgeMember>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(knowledge::add_member(&state.db, &actor, collection_id, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}/members/{membership_id}/remove",operation_id="removeKnowledgeCollectionMember",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("collection_id"=Uuid,Path),("membership_id"=Uuid,Path),("Idempotency-Key"=String,Header)),responses((status=200,body=KnowledgeMemberRemoved),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn remove_member(
    State(state): State<AppState>,
    Path((workspace, collection_id, membership_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<KnowledgeMemberRemoved>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::remove_member(
            &state.db,
            &actor,
            collection_id,
            membership_id,
            key(&headers)?,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/directory",operation_id="listKnowledgeDirectory",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),responses((status=200,body=Vec<KnowledgeMember>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn directory(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<KnowledgeMember>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(knowledge::directory(&state.db, &actor).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/documents",operation_id="listKnowledgeDocuments",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListKnowledgeDocuments),responses((status=200,body=KnowledgeDocumentPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list_documents(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<ListKnowledgeDocuments>,
) -> Result<Json<KnowledgeDocumentPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::list_documents(&state.db, &actor, query).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/knowledge/documents",operation_id="createKnowledgeDocument",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=CreateKnowledgeDocument,responses((status=201,body=KnowledgeDocument),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_document(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<CreateKnowledgeDocument>,
) -> Result<(StatusCode, Json<KnowledgeDocument>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(knowledge::create_document(&state.db, &actor, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}",operation_id="getKnowledgeDocument",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("document_id"=Uuid,Path)),responses((status=200,body=KnowledgeDocument),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn get_document(
    State(state): State<AppState>,
    Path((workspace, document_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<KnowledgeDocument>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::get_document(&state.db, &actor, document_id).await?,
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}",operation_id="updateKnowledgeDocument",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("document_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveKnowledgeDocument,responses((status=200,body=KnowledgeDocument),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_document(
    State(state): State<AppState>,
    Path((workspace, document_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveKnowledgeDocument>,
) -> Result<Json<KnowledgeDocument>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::update_document(&state.db, &actor, document_id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}/archive",operation_id="archiveKnowledgeDocument",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("document_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveKnowledgeRecord,responses((status=200,body=KnowledgeDocument),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive_document(
    State(state): State<AppState>,
    Path((workspace, document_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveKnowledgeRecord>,
) -> Result<Json<KnowledgeDocument>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::archive_document(&state.db, &actor, document_id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}/move",operation_id="moveKnowledgeDocument",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("document_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=MoveKnowledgeDocument,responses((status=200,body=KnowledgeDocument),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn move_document(
    State(state): State<AppState>,
    Path((workspace, document_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<MoveKnowledgeDocument>,
) -> Result<Json<KnowledgeDocument>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::move_document(&state.db, &actor, document_id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}/revisions",operation_id="listKnowledgeRevisions",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("document_id"=Uuid,Path),KnowledgePageQuery),responses((status=200,body=KnowledgeRevisionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list_revisions(
    State(state): State<AppState>,
    Path((workspace, document_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<KnowledgePageQuery>,
) -> Result<Json<KnowledgeRevisionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::revisions(&state.db, &actor, document_id, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}/revisions/{revision_id}",operation_id="getKnowledgeRevision",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("document_id"=Uuid,Path),("revision_id"=Uuid,Path)),responses((status=200,body=KnowledgeRevision),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn get_revision(
    State(state): State<AppState>,
    Path((workspace, document_id, revision_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<KnowledgeRevision>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::revision(&state.db, &actor, document_id, revision_id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}/revisions/{revision_id}/restore",operation_id="restoreKnowledgeRevision",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("document_id"=Uuid,Path),("revision_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RestoreKnowledgeRevision,responses((status=200,body=KnowledgeDocument),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn restore_revision(
    State(state): State<AppState>,
    Path((workspace, document_id, revision_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<RestoreKnowledgeRevision>,
) -> Result<Json<KnowledgeDocument>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        knowledge::restore_revision(
            &state.db,
            &actor,
            document_id,
            revision_id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/search",operation_id="searchKnowledge",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),KnowledgeSearchQuery),responses((status=200,body=KnowledgeSearchPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn search(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<KnowledgeSearchQuery>,
) -> Result<Json<KnowledgeSearchPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(knowledge::search(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/suggest",operation_id="suggestKnowledgeDocuments",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),KnowledgeSuggestQuery),responses((status=200,body=KnowledgeSuggestions),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn suggest(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<KnowledgeSuggestQuery>,
) -> Result<Json<KnowledgeSuggestions>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(knowledge::suggest(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/knowledge/activity",operation_id="listKnowledgeActivity",tag="knowledge",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),KnowledgePageQuery),responses((status=200,body=KnowledgeActivityPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn activity(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<KnowledgePageQuery>,
) -> Result<Json<KnowledgeActivityPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(knowledge::activity(&state.db, &actor, query).await?))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_collections, create_collection))
        .routes(routes!(get_collection, update_collection))
        .routes(routes!(archive_collection))
        .routes(routes!(list_members, add_member))
        .routes(routes!(remove_member))
        .routes(routes!(directory))
        .routes(routes!(list_documents, create_document))
        .routes(routes!(get_document, update_document))
        .routes(routes!(archive_document))
        .routes(routes!(move_document))
        .routes(routes!(list_revisions))
        .routes(routes!(get_revision))
        .routes(routes!(restore_revision))
        .routes(routes!(search))
        .routes(routes!(suggest))
        .routes(routes!(activity))
}
