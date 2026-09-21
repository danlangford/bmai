// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

pub fn derive(package_version: &str, describe: &str) -> String {
    let Some(value) = describe.strip_prefix("bmair-v") else {
        return if describe == "unknown" {
            package_version.to_owned()
        } else {
            format!("{package_version}+g{}", describe.replace('-', "."))
        };
    };
    let Some((version_and_count, commit)) = value.rsplit_once("-g") else {
        return value.to_owned();
    };
    let Some((version, count)) = version_and_count.rsplit_once('-') else {
        return value.to_owned();
    };
    let dirty = commit.strip_suffix("-dirty");
    let commit = dirty.unwrap_or(commit);
    if count == "0" && dirty.is_none() {
        version.to_owned()
    } else {
        let dirty = if dirty.is_some() { ".dirty" } else { "" };
        format!("{package_version}-dev.{count}+g{commit}{dirty}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_release_tag_is_the_release_version() {
        assert_eq!(derive("0.2.0", "bmair-v0.2.0-0-gdeadbee"), "0.2.0");
    }

    #[test]
    fn development_version_uses_upcoming_version_and_git_distance() {
        assert_eq!(
            derive("0.2.0", "bmair-v0.1.0-23-g95ba190"),
            "0.2.0-dev.23+g95ba190"
        );
        assert_eq!(
            derive("0.2.0", "bmair-v0.1.0-23-g95ba190-dirty"),
            "0.2.0-dev.23+g95ba190.dirty"
        );
    }

    #[test]
    fn repository_without_release_tags_retains_commit_identity() {
        assert_eq!(derive("0.2.0", "95ba190"), "0.2.0+g95ba190");
        assert_eq!(derive("0.2.0", "unknown"), "0.2.0");
    }
}
