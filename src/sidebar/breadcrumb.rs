use std::path::{Path, PathBuf};

/// One clickable step in the sidebar's path bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub label: String,
    pub path: PathBuf,
}

/// The folders from the top of the path down to `root`, with the home folder
/// shown as `~` and nothing above it. Outside home, the path starts at `/`.
pub fn crumbs(root: &Path, home: Option<&Path>) -> Vec<Crumb> {
    let top = home.filter(|home| root.starts_with(home));
    let mut crumbs: Vec<Crumb> = root
        .ancestors()
        .take_while(|dir| top.is_none_or(|home| dir.starts_with(home)))
        .map(|dir| Crumb {
            label: label(dir, top),
            path: dir.to_path_buf(),
        })
        .collect();
    crumbs.reverse();
    crumbs
}

fn label(dir: &Path, home: Option<&Path>) -> String {
    if home == Some(dir) {
        return "~".into();
    }
    dir.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string())
}

/// `root` with the home folder shortened to `~`, for tooltips.
pub fn display_path(root: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| root.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => root.display().to_string(),
    }
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(crumbs: &[Crumb]) -> Vec<&str> {
        crumbs.iter().map(|crumb| crumb.label.as_str()).collect()
    }

    #[test]
    fn paths_under_home_start_at_tilde() {
        let home = Path::new("/home/pink");
        let crumbs = crumbs(Path::new("/home/pink/code/slate"), Some(home));
        assert_eq!(labels(&crumbs), ["~", "code", "slate"]);
        assert_eq!(crumbs[0].path, home);
        assert_eq!(crumbs[2].path, Path::new("/home/pink/code/slate"));
    }

    #[test]
    fn paths_outside_home_start_at_the_filesystem_root() {
        let crumbs = crumbs(Path::new("/etc/xdg"), Some(Path::new("/home/pink")));
        assert_eq!(labels(&crumbs), ["/", "etc", "xdg"]);
    }

    #[test]
    fn home_and_filesystem_root_are_single_crumbs() {
        let home = Path::new("/home/pink");
        assert_eq!(labels(&crumbs(home, Some(home))), ["~"]);
        assert_eq!(labels(&crumbs(Path::new("/"), Some(home))), ["/"]);
    }

    #[test]
    fn a_sibling_with_a_matching_prefix_is_not_under_home() {
        let crumbs = crumbs(Path::new("/home/pinkish/notes"), Some(Path::new("/home/pink")));
        assert_eq!(labels(&crumbs), ["/", "home", "pinkish", "notes"]);
    }

    #[test]
    fn display_path_shortens_home() {
        let home = Some(Path::new("/home/pink"));
        assert_eq!(display_path(Path::new("/home/pink"), home), "~");
        assert_eq!(display_path(Path::new("/home/pink/code"), home), "~/code");
        assert_eq!(display_path(Path::new("/etc"), home), "/etc");
    }
}
