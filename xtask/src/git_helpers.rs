use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn git_command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(root);
    sanitize_inherited_git_index(root, &mut command);
    command
}

fn sanitize_inherited_git_index(root: &Path, command: &mut Command) {
    let Ok(index) = std::env::var("GIT_INDEX_FILE") else {
        return;
    };
    if should_remove_inherited_git_index(root, &index) {
        command.env_remove("GIT_INDEX_FILE");
    }
}

fn should_remove_inherited_git_index(root: &Path, index: &str) -> bool {
    let index_path = PathBuf::from(index);
    let absolute_index = if index_path.is_absolute() { index_path } else { root.join(index_path) };
    !absolute_index.starts_with(root.join(".git"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_git_index_is_removed() {
        let root = Path::new("/repo");

        assert!(should_remove_inherited_git_index(root, "/tmp/external-index"));
        assert!(should_remove_inherited_git_index(root, "external-index"));
        assert!(!should_remove_inherited_git_index(root, "/repo/.git/index"));
        assert!(!should_remove_inherited_git_index(root, ".git/worktrees/feature/index"));
    }
}
