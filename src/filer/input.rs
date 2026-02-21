use crate::dto::TesInput;
use crate::filer::error::{FilerError, Result};
use crate::filer::storage::{Storage, extract_path};
use std::path::Path;

pub async fn stage_input(storage: &dyn Storage, input: &TesInput, workspace: &Path) -> Result<()> {
    let dest_path = resolve_dest_path(workspace, &input.path)?;

    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| FilerError::Io(e))?;
    }

    if let Some(content) = &input.content {
        write_content_to_path(content, &dest_path).await?;
    } else if let Some(url) = &input.url {
        download_from_storage(storage, url, &dest_path).await?;
    } else {
        return Err(FilerError::MissingInputSource(input.path.clone()));
    }

    Ok(())
}

fn resolve_dest_path(workspace: &Path, input_path: &str) -> Result<std::path::PathBuf> {
    let relative = input_path.strip_prefix('/').unwrap_or(input_path);

    let mut dest = workspace.to_path_buf();
    for component in Path::new(relative).components() {
        dest.push(component);
    }

    Ok(dest)
}

async fn write_content_to_path(content: &str, dest: &Path) -> Result<()> {
    tokio::fs::write(dest, content)
        .await
        .map_err(FilerError::Io)
}

async fn download_from_storage(storage: &dyn Storage, url: &str, dest: &Path) -> Result<()> {
    let path = extract_path(url)?;
    let data = storage.get(path).await?;

    tokio::fs::write(dest, data).await.map_err(FilerError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_dest_path() {
        let workspace = PathBuf::from("/workspace");

        assert_eq!(
            resolve_dest_path(&workspace, "/data/input.txt").unwrap(),
            PathBuf::from("/workspace/data/input.txt")
        );

        assert_eq!(
            resolve_dest_path(&workspace, "relative/path.txt").unwrap(),
            PathBuf::from("/workspace/relative/path.txt")
        );
    }
}
