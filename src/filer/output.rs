use crate::dto::TesFileType;
use crate::dto::TesOutput;
use crate::filer::error::{FilerError, Result};
use crate::filer::storage::{Storage, extract_path};
use std::collections::VecDeque;
use std::path::Path;

pub async fn collect_output(
    storage: &dyn Storage,
    output: &TesOutput,
    workspace: &Path,
) -> Result<()> {
    let src_path = resolve_src_path(workspace, &output.path)?;

    if !src_path.exists() {
        return Err(FilerError::PathNotFound(src_path));
    }

    let dest_path = extract_path(&output.url)?;

    if contains_glob_pattern(&output.path) {
        collect_glob_output(storage, output, workspace, dest_path).await
    } else if src_path.is_dir() || output.r#type == Some(TesFileType::Directory) {
        collect_directory_output(storage, &src_path, dest_path).await
    } else {
        collect_file_output(storage, &src_path, dest_path).await
    }
}

fn resolve_src_path(workspace: &Path, output_path: &str) -> Result<std::path::PathBuf> {
    let relative = output_path.strip_prefix('/').unwrap_or(output_path);

    let mut src = workspace.to_path_buf();
    for component in Path::new(relative).components() {
        src.push(component);
    }

    Ok(src)
}

fn contains_glob_pattern(path: &str) -> bool {
    let chars_to_check = ['*', '?', '['];
    path.chars().any(|c| chars_to_check.contains(&c))
}

async fn collect_file_output(storage: &dyn Storage, src: &Path, dest: &str) -> Result<()> {
    let data = tokio::fs::read(src).await.map_err(FilerError::Io)?;
    storage.put(dest, data.into()).await
}

async fn collect_directory_output(
    storage: &dyn Storage,
    src_dir: &Path,
    dest_prefix: &str,
) -> Result<()> {
    let mut queue: VecDeque<(std::path::PathBuf, String)> = VecDeque::new();
    queue.push_back((src_dir.to_path_buf(), dest_prefix.to_string()));

    while let Some((current_dir, current_dest)) = queue.pop_front() {
        let mut entries = tokio::fs::read_dir(&current_dir)
            .await
            .map_err(FilerError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(FilerError::Io)? {
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
                collect_file_output(storage, &entry_path, &entry_dest).await?;
            }
        }
    }

    Ok(())
}

async fn collect_glob_output(
    storage: &dyn Storage,
    output: &TesOutput,
    workspace: &Path,
    dest_base: &str,
) -> Result<()> {
    let pattern = resolve_src_path(workspace, &output.path)?;
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
            collect_directory_output(storage, &entry, &entry_dest).await?;
        } else {
            collect_file_output(storage, &entry, &entry_dest).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_contains_glob_pattern() {
        assert!(contains_glob_pattern("/data/*.txt"));
        assert!(contains_glob_pattern("/data/file?.txt"));
        assert!(contains_glob_pattern("/data/[abc].txt"));
        assert!(!contains_glob_pattern("/data/file.txt"));
    }

    #[test]
    fn test_resolve_src_path() {
        let workspace = PathBuf::from("/workspace");

        assert_eq!(
            resolve_src_path(&workspace, "/data/output.txt").unwrap(),
            PathBuf::from("/workspace/data/output.txt")
        );
    }
}
