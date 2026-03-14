use crate::dto::TesFileType;
use crate::dto::TesOutput;
use crate::dto::TesOutputFileLog;
use crate::filer::backend::StorageBackend;
use crate::filer::error::{FilerError, Result};
use crate::filer::url::{parse_storage_url, warn_bucket_mismatch};
use crate::filer::util::resolve_workspace_path;
use std::collections::VecDeque;
use std::path::Path;

pub async fn collect_output(
    backend: &impl StorageBackend,
    output: &TesOutput,
    workspace: &Path,
    scheme: &str,
    configured_bucket: &str,
) -> Result<Vec<TesOutputFileLog>> {
    let src_path = resolve_workspace_path(workspace, &output.path)?;
    let parsed = parse_storage_url(&output.url)?;
    warn_bucket_mismatch(parsed.bucket, configured_bucket);
    let dest_key = parsed.key;

    if contains_glob_pattern(&output.path) {
        collect_glob_output(
            backend,
            output,
            workspace,
            dest_key,
            scheme,
            configured_bucket,
        )
        .await
    } else {
        if !tokio::fs::try_exists(&src_path).await.unwrap_or(false) {
            return Err(FilerError::PathNotFound(src_path));
        }

        if tokio::fs::metadata(&src_path).await?.is_dir()
            || output.r#type == Some(TesFileType::Directory)
        {
            collect_directory_output(backend, &src_path, dest_key, scheme, configured_bucket).await
        } else {
            let log = collect_file_output(backend, &src_path, dest_key, scheme, configured_bucket)
                .await?;
            Ok(vec![log])
        }
    }
}

fn contains_glob_pattern(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

async fn collect_file_output(
    backend: &impl StorageBackend,
    src: &Path,
    dest_key: &str,
    scheme: &str,
    bucket: &str,
) -> Result<TesOutputFileLog> {
    let data = tokio::fs::read(src).await?;
    let size = data.len();
    backend.put(dest_key, data.into()).await?;
    Ok(TesOutputFileLog::new(
        format!("{}://{}/{}", scheme, bucket, dest_key),
        src.to_string_lossy().to_string(),
        size.to_string(),
    ))
}

async fn collect_directory_output(
    backend: &impl StorageBackend,
    src_dir: &Path,
    dest_prefix: &str,
    scheme: &str,
    bucket: &str,
) -> Result<Vec<TesOutputFileLog>> {
    let mut logs = Vec::new();
    let mut queue: VecDeque<(std::path::PathBuf, String)> = VecDeque::new();
    queue.push_back((src_dir.to_path_buf(), dest_prefix.to_string()));

    while let Some((current_dir, current_dest)) = queue.pop_front() {
        let mut entries = tokio::fs::read_dir(&current_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            let entry_name = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| FilerError::PathNotFound(entry_path.clone()))?;

            let entry_dest = if current_dest.ends_with('/') {
                format!("{}{}", current_dest, entry_name)
            } else {
                format!("{}/{}", current_dest, entry_name)
            };

            if entry_path.is_dir() {
                queue.push_back((entry_path, entry_dest));
            } else {
                logs.push(
                    collect_file_output(backend, &entry_path, &entry_dest, scheme, bucket).await?,
                );
            }
        }
    }

    Ok(logs)
}

async fn collect_glob_output(
    backend: &impl StorageBackend,
    output: &TesOutput,
    workspace: &Path,
    dest_base: &str,
    scheme: &str,
    bucket: &str,
) -> Result<Vec<TesOutputFileLog>> {
    let pattern = resolve_workspace_path(workspace, &output.path)?;
    let pattern_str = pattern
        .to_str()
        .ok_or_else(|| FilerError::GlobPattern("invalid path encoding".into()))?;

    let prefix_to_strip = output
        .path_prefix
        .as_ref()
        .map(|p| p.trim_start_matches('/'))
        .unwrap_or("");

    let base_dir = workspace.join(prefix_to_strip);
    let base_dir_str = base_dir
        .to_str()
        .ok_or_else(|| FilerError::GlobPattern("invalid base dir encoding".into()))?;

    let mut logs = Vec::new();

    for entry in glob::glob(pattern_str).map_err(|e| FilerError::GlobPattern(e.to_string()))? {
        let entry = entry.map_err(|e| FilerError::GlobPattern(e.to_string()))?;

        let relative = entry
            .strip_prefix(base_dir_str)
            .map_err(|_| FilerError::GlobPattern("path prefix mismatch".into()))?;

        let entry_dest = if dest_base.ends_with('/') {
            format!("{}{}", dest_base, relative.to_string_lossy())
        } else {
            format!("{}/{}", dest_base, relative.to_string_lossy())
        };

        if entry.is_dir() {
            logs.extend(
                collect_directory_output(backend, &entry, &entry_dest, scheme, bucket).await?,
            );
        } else {
            logs.push(collect_file_output(backend, &entry, &entry_dest, scheme, bucket).await?);
        }
    }

    Ok(logs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_glob_pattern() {
        assert!(contains_glob_pattern("/data/*.txt"));
        assert!(contains_glob_pattern("/data/file?.txt"));
        assert!(contains_glob_pattern("/data/[abc].txt"));
        assert!(!contains_glob_pattern("/data/file.txt"));
    }
}
