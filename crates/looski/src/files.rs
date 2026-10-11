// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded immutable file content, independently scoped from linked applications.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FileInput {
    pub filename: String,
    pub media_type: String,
    #[serde(default)]
    pub notes: String,
    pub owner_membership_id: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UploadFile {
    #[serde(flatten)]
    pub metadata: FileInput,
    pub content_base64: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditFile {
    pub expected_version: i64,
    #[serde(flatten)]
    pub metadata: FileInput,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveFile {
    pub expected_version: i64,
    pub archived: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FileRecord {
    pub id: Uuid,
    pub filename: String,
    pub media_type: String,
    pub notes: String,
    pub owner_membership_id: Uuid,
    pub byte_length: i64,
    pub sha256: String,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct FileQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub archived: Option<bool>,
    pub search: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FilePage {
    pub items: Vec<FileRecord>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FileContent {
    pub file: FileRecord,
    pub content_base64: String,
}
fn validate(input: &FileInput) -> Result<(), Error> {
    if input.filename.is_empty()
        || input.filename.chars().count() > 255
        || input.filename.trim() != input.filename
        || matches!(input.filename.as_str(), "." | "..")
        || input
            .filename
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
        || input.notes.chars().count() > 10000
    {
        return Err(Error::Invalid(
            "Use a filename of 1–255 printable characters without paths, and notes up to 10000 characters",
        ));
    }
    let Some((kind, subtype)) = input.media_type.split_once('/') else {
        return Err(Error::Invalid(
            "Media type must be type/subtype without parameters",
        ));
    };
    let token = |value: &str| {
        !value.is_empty()
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&b))
    };
    if input.media_type.len() > 127 || !token(kind) || !token(subtype) {
        return Err(Error::Invalid(
            "Media type must be type/subtype without parameters",
        ));
    }
    Ok(())
}
fn decode(value: &str) -> Result<Vec<u8>, Error> {
    if value.len() > MAX_BYTES.div_ceil(3) * 4 {
        return Err(Error::Invalid("File content exceeds 8 MiB"));
    }
    let bytes = STANDARD
        .decode(value)
        .map_err(|_| Error::Invalid("Content must be canonical base64"))?;
    if bytes.len() > MAX_BYTES {
        return Err(Error::Invalid("File content exceeds 8 MiB"));
    }
    Ok(bytes)
}
pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub fn verified_bytes(content: &FileContent) -> Result<Vec<u8>, Error> {
    let bytes = decode(&content.content_base64)?;
    if bytes.len() as i64 != content.file.byte_length || digest(&bytes) != content.file.sha256 {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}
/// Publish a verified private download without overwriting any destination.
pub fn write_download(path: &std::path::Path, content: &FileContent) -> anyhow::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let bytes = verified_bytes(content)
        .map_err(|_| anyhow::anyhow!("File content failed integrity verification"))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let temporary = parent.join(format!(".looski-download-{}", Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    let result = (|| -> anyhow::Result<()> {
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::hard_link(&temporary, path)?;
        std::fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    let cleanup = std::fs::remove_file(&temporary);
    result?;
    cleanup?;
    Ok(())
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    q: FileQuery,
) -> Result<FilePage, Error> {
    let limit = q.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) || q.search.as_ref().is_some_and(|s| s.len() > 200) {
        return Err(Error::Invalid(
            "Limit must be 1–100 and search at most 200 bytes",
        ));
    }
    let mut tx = pool.begin().await?;
    let read = access::require(&mut tx, actor.identity, actor.workspace, "files.read").await?;
    let mut items = dao::list(&mut tx, actor.workspace, &read, &q, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(FilePage { items, next_cursor })
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<FileRecord, Error> {
    let mut tx = pool.begin().await?;
    let read = access::require(&mut tx, actor.identity, actor.workspace, "files.read").await?;
    dao::get(&mut tx, actor.workspace, id, &read, false).await
}
pub async fn content(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<FileContent, Error> {
    let mut tx = pool.begin().await?;
    let read = access::require(&mut tx, actor.identity, actor.workspace, "files.read").await?;
    let file = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    let bytes = dao::content(&mut tx, actor.workspace, id).await?;
    if bytes.len() as i64 != file.byte_length || digest(&bytes) != file.sha256 {
        return Err(Error::Unavailable);
    }
    Ok(FileContent {
        file,
        content_base64: STANDARD.encode(bytes),
    })
}
/// A file the actor may read right now, for another module to adopt.
pub(crate) async fn readable(
    db: &mut sqlx::PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<FileRecord, Error> {
    let read = access::require(db, actor.identity, actor.workspace, "files.read").await?;
    dao::get(db, actor.workspace, id, &read, false).await
}
/// Verified bytes of an active file, with no actor. Callers own the decision that
/// these bytes may be released; CMS media is the only one and goes through its own checks.
pub(crate) async fn active_bytes(
    db: &mut sqlx::PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Vec<u8>, Error> {
    let (byte_length, sha256) = dao::active_digest(db, workspace, id).await?;
    let bytes = dao::content(db, workspace, id).await?;
    if bytes.len() as i64 != byte_length || digest(&bytes) != sha256 {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}
async fn mutation(
    db: &mut sqlx::PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<(access::Access, access::Access), Error> {
    access::require(db, actor.identity, actor.workspace, action).await?;
    crate::modules::lock_workspace(db, actor.workspace).await?;
    Ok((
        access::require(db, actor.identity, actor.workspace, action).await?,
        access::require(db, actor.identity, actor.workspace, "files.read").await?,
    ))
}
pub async fn upload(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: UploadFile,
) -> Result<FileRecord, Error> {
    validate(&input.metadata)?;
    let bytes = decode(&input.content_base64)?;
    let sha256 = digest(&bytes);
    let mut tx = pool.begin().await?;
    let (write, read) = mutation(&mut tx, actor, "files.create").await?;
    let owner = input
        .metadata
        .owner_membership_id
        .unwrap_or(write.membership_id);
    if !write.permits_owner(owner) || !read.permits_owner(owner) {
        return Err(Error::Forbidden);
    }
    access::require_active_member(&mut tx, actor.workspace, owner).await?;
    // Do not duplicate uploaded bytes in replay/audit storage.
    let request =
        serde_json::json!({"metadata":input.metadata,"byte_length":bytes.len(),"sha256":sha256});
    if let Some(saved) = reliability::replay::<FileRecord>(
        &mut tx,
        actor.workspace,
        &write,
        "files.upload",
        key,
        &request,
    )
    .await?
    {
        dao::get(&mut tx, actor.workspace, saved.id, &write, false).await?;
        dao::get(&mut tx, actor.workspace, saved.id, &read, false).await?;
        return Ok(saved);
    }
    let id = dao::insert(
        &mut tx,
        actor.workspace,
        owner,
        &input.metadata,
        &sha256,
        &bytes,
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "files.uploaded",
        id,
        serde_json::json!({"byte_length":bytes.len()}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        "files.upload",
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn edit(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: EditFile,
) -> Result<FileRecord, Error> {
    validate(&input.metadata)?;
    if input.expected_version < 1 {
        return Err(Error::Invalid("A positive expected version is required"));
    }
    let mut tx = pool.begin().await?;
    let (write, read) = mutation(&mut tx, actor, "files.update").await?;
    let request = serde_json::to_value(&input).expect("file edit serializes");
    let operation = format!("files.update:{id}");
    if let Some(saved) = reliability::replay::<FileRecord>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::get(&mut tx, actor.workspace, id, &write, false).await?;
        dao::get(&mut tx, actor.workspace, id, &read, false).await?;
        return Ok(saved);
    }
    let current = dao::get(&mut tx, actor.workspace, id, &write, true).await?;
    if !read.permits_owner(current.owner_membership_id) {
        return Err(Error::NotFound);
    }
    if current.version != input.expected_version || current.archived {
        return Err(Error::Conflict(
            "File changed or is archived; reload before editing",
        ));
    }
    let owner = input
        .metadata
        .owner_membership_id
        .unwrap_or(current.owner_membership_id);
    if !write.permits_owner(owner) || !read.permits_owner(owner) {
        return Err(Error::Forbidden);
    }
    access::require_active_member(&mut tx, actor.workspace, owner).await?;
    dao::update(&mut tx, actor.workspace, id, owner, &input.metadata).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "files.updated",
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn archive(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveFile,
) -> Result<FileRecord, Error> {
    if input.expected_version < 1 {
        return Err(Error::Invalid("A positive expected version is required"));
    }
    let mut tx = pool.begin().await?;
    let (write, read) = mutation(&mut tx, actor, "files.archive").await?;
    let request = serde_json::to_value(&input).expect("file archive serializes");
    let operation = format!("files.archive:{id}");
    if let Some(saved) = reliability::replay::<FileRecord>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::get(&mut tx, actor.workspace, id, &write, false).await?;
        dao::get(&mut tx, actor.workspace, id, &read, false).await?;
        return Ok(saved);
    }
    let current = dao::get(&mut tx, actor.workspace, id, &write, true).await?;
    if !read.permits_owner(current.owner_membership_id) {
        return Err(Error::NotFound);
    }
    if current.version != input.expected_version {
        return Err(Error::Conflict("File changed; reload before archiving"));
    }
    dao::archive(&mut tx, actor.workspace, id, input.archived).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "files.archived",
        id,
        serde_json::json!({"archived":input.archived}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn metadata(filename: &str) -> FileInput {
        FileInput {
            filename: filename.into(),
            media_type: "text/plain".into(),
            notes: String::new(),
            owner_membership_id: None,
        }
    }
    #[test]
    fn upload_validation_preserves_valid_names_and_rejects_paths_and_malformed_data() {
        assert!(validate(&metadata("Résumé 雪.txt")).is_ok());
        for filename in ["", ".", "..", "../a", "a/b", "a\\b", "a\nb", " a"] {
            assert!(validate(&metadata(filename)).is_err());
        }
        assert_eq!(decode("aGk=").unwrap(), b"hi");
        for encoded in ["not base64", "aGk", "aGk=\n", "aGl="] {
            assert!(decode(encoded).is_err());
        }
        assert!(decode(&STANDARD.encode(vec![0; MAX_BYTES + 1])).is_err());
        assert_eq!(
            decode(&STANDARD.encode(vec![0; MAX_BYTES])).unwrap().len(),
            MAX_BYTES
        );
    }
    #[test]
    fn hashes_match_the_standard_known_answer() {
        assert_eq!(
            digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn downloads_require_integrity_and_never_overwrite_existing_files() {
        use std::os::unix::fs::PermissionsExt;
        let directory = std::env::temp_dir().join(format!("looski-file-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("download");
        let mut content = FileContent {
            content_base64: STANDARD.encode(b"verified bytes"),
            file: FileRecord {
                id: Uuid::new_v4(),
                filename: "test".into(),
                media_type: "text/plain".into(),
                notes: String::new(),
                owner_membership_id: Uuid::new_v4(),
                byte_length: 14,
                sha256: digest(b"verified bytes"),
                archived: false,
                version: 1,
                created_at: String::new(),
                updated_at: String::new(),
            },
        };
        assert!(write_download(&path, &content).is_ok());
        assert_eq!(std::fs::read(&path).unwrap(), b"verified bytes");
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(write_download(&path, &content).is_err());
        content.file.sha256 = digest(b"wrong bytes");
        assert!(write_download(&directory.join("invalid"), &content).is_err());
        assert!(!directory.join("invalid").exists());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
