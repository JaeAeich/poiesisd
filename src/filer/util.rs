use crate::filer::error::Result;
use std::path::{Path, PathBuf};

/// Resolves a container-absolute path (e.g. `/data/file.txt`) into a
/// workspace-rooted path by stripping the leading `/` and joining.
pub fn resolve_workspace_path(workspace: &Path, container_path: &str) -> Result<PathBuf> {
    let relative = container_path.strip_prefix('/').unwrap_or(container_path);

    let mut dest = workspace.to_path_buf();
    for component in Path::new(relative).components() {
        dest.push(component);
    }

    Ok(dest)
}
