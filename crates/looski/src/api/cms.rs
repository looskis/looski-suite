// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    cms::{
        self, ArchiveContent, ContentEntry, ContentMedia, ContentRevision, ContentType,
        ContentTypePage, CreateEntry, EntryPage, ExpectedVersion, ListContentTypes, ListEntries,
        ListMedia, MediaContent, MediaPage, PublicIndex, PublicIndexQuery, PublicPage,
        RegisterMedia, RestoredEntry, RevisionPage, RevisionQuery, SaveContentType, SaveEntry,
        SaveMedia,
    },
    identity::Identity,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
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

#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/types",operation_id="listContentTypes",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListContentTypes),responses((status=200,body=ContentTypePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list_types(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<ListContentTypes>,
) -> Result<Json<ContentTypePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::list_types(&state.db, &actor, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/cms/types",operation_id="createContentType",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveContentType,responses((status=201,body=ContentType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_type(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveContentType>,
) -> Result<(StatusCode, Json<ContentType>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(cms::save_type(&state.db, &actor, None, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/types/{type_id}",operation_id="getContentType",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("type_id"=Uuid,Path)),responses((status=200,body=ContentType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn get_type(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<ContentType>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::get_type(&state.db, &actor, id).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/cms/types/{type_id}",operation_id="updateContentType",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("type_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveContentType,responses((status=200,body=ContentType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_type(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveContentType>,
) -> Result<Json<ContentType>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        cms::save_type(&state.db, &actor, Some(id), key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/cms/types/{type_id}/archive",operation_id="archiveContentType",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("type_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveContent,responses((status=200,body=ContentType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive_type(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveContent>,
) -> Result<Json<ContentType>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        cms::archive_type(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/entries",operation_id="listContentEntries",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListEntries),responses((status=200,body=EntryPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list_entries(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<ListEntries>,
) -> Result<Json<EntryPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::list_entries(&state.db, &actor, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/cms/entries",operation_id="createContentEntry",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=CreateEntry,responses((status=201,body=ContentEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_entry(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<CreateEntry>,
) -> Result<(StatusCode, Json<ContentEntry>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(cms::create_entry(&state.db, &actor, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}",operation_id="getContentEntry",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("entry_id"=Uuid,Path)),responses((status=200,body=ContentEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn get_entry(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<ContentEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::get_entry(&state.db, &actor, id).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}",operation_id="updateContentEntry",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("entry_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveEntry,responses((status=200,body=ContentEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_entry(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveEntry>,
) -> Result<Json<ContentEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        cms::update_entry(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/publish",operation_id="publishContentEntry",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("entry_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ExpectedVersion,responses((status=200,body=ContentEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn publish(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ExpectedVersion>,
) -> Result<Json<ContentEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        cms::publish(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/unpublish",operation_id="unpublishContentEntry",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("entry_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ExpectedVersion,responses((status=200,body=ContentEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn unpublish(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ExpectedVersion>,
) -> Result<Json<ContentEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        cms::unpublish(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/archive",operation_id="archiveContentEntry",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("entry_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveContent,responses((status=200,body=ContentEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive_entry(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveContent>,
) -> Result<Json<ContentEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        cms::archive_entry(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/preview",operation_id="previewContentEntry",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("entry_id"=Uuid,Path)),responses((status=200,body=PublicPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn preview(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<PublicPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::preview(&state.db, &actor, id).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/revisions",operation_id="listContentRevisions",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("entry_id"=Uuid,Path),RevisionQuery),responses((status=200,body=RevisionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn revisions(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<RevisionQuery>,
) -> Result<Json<RevisionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::revisions(&state.db, &actor, id, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/revisions/{revision_id}",operation_id="getContentRevision",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("entry_id"=Uuid,Path),("revision_id"=Uuid,Path)),responses((status=200,body=ContentRevision),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn revision(
    State(state): State<AppState>,
    Path((workspace, id, revision)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<ContentRevision>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::revision(&state.db, &actor, id, revision).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/revisions/{revision_id}/restore",operation_id="restoreContentRevision",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("entry_id"=Uuid,Path),("revision_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ExpectedVersion,responses((status=200,body=RestoredEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn restore(
    State(state): State<AppState>,
    Path((workspace, id, revision)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ExpectedVersion>,
) -> Result<Json<RestoredEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        cms::restore_revision(&state.db, &actor, id, revision, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/media",operation_id="listContentMedia",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListMedia),responses((status=200,body=MediaPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list_media(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<ListMedia>,
) -> Result<Json<MediaPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::list_media(&state.db, &actor, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/cms/media",operation_id="registerContentMedia",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RegisterMedia,responses((status=201,body=ContentMedia),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn register_media(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<RegisterMedia>,
) -> Result<(StatusCode, Json<ContentMedia>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok((
        StatusCode::CREATED,
        Json(cms::register_media(&state.db, &actor, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/media/{media_id}",operation_id="getContentMedia",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("media_id"=Uuid,Path)),responses((status=200,body=ContentMedia),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn get_media(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<ContentMedia>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::get_media(&state.db, &actor, id).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/cms/media/{media_id}",operation_id="updateContentMedia",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("media_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveMedia,responses((status=200,body=ContentMedia),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_media(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveMedia>,
) -> Result<Json<ContentMedia>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        cms::update_media(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/cms/media/{media_id}/archive",operation_id="archiveContentMedia",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("media_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveContent,responses((status=200,body=ContentMedia),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive_media(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveContent>,
) -> Result<Json<ContentMedia>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(
        cms::archive_media(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/cms/media/{media_id}/content",operation_id="getContentMediaBytes",tag="cms",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("media_id"=Uuid,Path)),responses((status=200,body=MediaContent),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn media_content(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<MediaContent>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = actor(&identity, workspace, request_id, &headers);
    Ok(Json(cms::media_content(&state.db, &actor, id).await?))
}
#[utoipa::path(get,path="/api/v1/public/cms/{public_key}",operation_id="getPublicContentIndex",tag="cms",params(("public_key"=String,Path),PublicIndexQuery),responses((status=200,body=PublicIndex),(status=404,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn public_index(
    State(state): State<AppState>,
    Path(public_key): Path<String>,
    headers: HeaderMap,
    Query(query): Query<PublicIndexQuery>,
) -> Result<Json<PublicIndex>, Failure> {
    admit(&state, &headers, "cms.read", 300)?;
    Ok(Json(
        cms::public_index(&state.db, &public_key, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/public/cms/{public_key}/entries/{slug}",operation_id="getPublicContentEntry",tag="cms",params(("public_key"=String,Path),("slug"=String,Path)),responses((status=200,body=PublicPage),(status=404,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn public_entry(
    State(state): State<AppState>,
    Path((public_key, slug)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<PublicPage>, Failure> {
    admit(&state, &headers, "cms.read", 300)?;
    Ok(Json(
        cms::public_entry(&state.db, &public_key, &slug).await?,
    ))
}
/// Bytes go out with the type fixed at registration, never sniffed, and sandboxed so
/// a file opened directly cannot act as a page on this origin.
#[utoipa::path(get,path="/api/v1/public/cms/{public_key}/media/{media_id}",operation_id="getPublicContentMedia",tag="cms",params(("public_key"=String,Path),("media_id"=Uuid,Path)),responses((status=200,content_type="application/octet-stream",body=Vec<u8>),(status=404,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn public_media(
    State(state): State<AppState>,
    Path((public_key, media)): Path<(String, Uuid)>,
    headers: HeaderMap,
) -> Result<Response, Failure> {
    admit(&state, &headers, "cms.media", 600)?;
    let content = cms::public_media(&state.db, &public_key, media).await?;
    let disposition = if content.media_type.starts_with("image/") {
        "inline"
    } else {
        "attachment"
    };
    let mut response = content.bytes.into_response();
    let target = response.headers_mut();
    target.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&content.media_type).expect("allow-listed media type"),
    );
    target.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static(disposition),
    );
    target.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    target.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; sandbox"),
    );
    Ok(response)
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_types, create_type))
        .routes(routes!(get_type, update_type))
        .routes(routes!(archive_type))
        .routes(routes!(list_entries, create_entry))
        .routes(routes!(get_entry, update_entry))
        .routes(routes!(publish))
        .routes(routes!(unpublish))
        .routes(routes!(archive_entry))
        .routes(routes!(preview))
        .routes(routes!(revisions))
        .routes(routes!(revision))
        .routes(routes!(restore))
        .routes(routes!(list_media, register_media))
        .routes(routes!(get_media, update_media))
        .routes(routes!(archive_media))
        .routes(routes!(media_content))
        .routes(routes!(public_index))
        .routes(routes!(public_entry))
        .routes(routes!(public_media))
}
