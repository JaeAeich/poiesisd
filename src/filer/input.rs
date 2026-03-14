use crate::dto::TesFileType;
use crate::dto::TesInput;
use crate::filer::backend::StorageBackend;
use crate::filer::error::{FilerError, Result};
use crate::filer::url::{parse_storage_url, warn_bucket_mismatch};
use crate::filer::util::resolve_workspace_path;
use std::path::Path;

pub async fn stage_input(
    backend: &impl StorageBackend,
    input: &TesInput,
    workspace: &Path,
    configured_bucket: &str,
) -> Result<()> {
    let dest_path = resolve_workspace_path(workspace, &input.path)?;

    if let Some(content) = &input.content {
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&dest_path, content).await?;
    } else if let Some(url) = &input.url {
        let parsed = parse_storage_url(url)?;
        warn_bucket_mismatch(parsed.bucket, configured_bucket);

        if input.r#type == Some(TesFileType::Directory) {
            tokio::fs::create_dir_all(&dest_path).await?;
            let objects = backend.list(parsed.key).await?;
            for obj_key in objects {
                let relative = obj_key
                    .strip_prefix(parsed.key)
                    .unwrap_or(&obj_key)
                    .trim_start_matches('/');
                let file_dest = dest_path.join(relative);
                if let Some(parent) = file_dest.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                let data = backend.get(&obj_key).await?;
                tokio::fs::write(&file_dest, data).await?;
            }
        } else {
            if let Some(parent) = dest_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let data = backend.get(parsed.key).await?;
            tokio::fs::write(&dest_path, data).await?;
        }
    } else {
        return Err(FilerError::MissingInputSource(input.path.clone()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::filer::util::resolve_workspace_path;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_workspace_path() {
        let workspace = PathBuf::from("/workspace");

        assert_eq!(
            resolve_workspace_path(&workspace, "/data/input.txt").unwrap(),
            PathBuf::from("/workspace/data/input.txt")
        );

        assert_eq!(
            resolve_workspace_path(&workspace, "relative/path.txt").unwrap(),
            PathBuf::from("/workspace/relative/path.txt")
        );
    }
}
