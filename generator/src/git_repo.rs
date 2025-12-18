use std::path::Path;

use anyhow::Context as _;
use chrono::TimeZone as _;

pub struct GitRepo {
    repo: git2::Repository,
}

impl GitRepo {
    pub fn new(dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let dir = dir.as_ref();

        let repo = git2::Repository::open(dir)
            .with_context(|| format!("cannot open git repo: {}", dir.display()))?;

        Ok(Self { repo })
    }

    pub fn as_inner(&self) -> &git2::Repository {
        &self.repo
    }

    pub fn into_inner(self) -> git2::Repository {
        self.repo
    }

    /// Returns all commits that modified the given file path.
    /// Return empty list if the file not found.
    pub fn commits_for_file(
        &self,
        file_path: impl AsRef<Path>,
    ) -> anyhow::Result<Vec<git2::Commit<'_>>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut ret = vec![];

        for oid in revwalk {
            let oid = oid?;

            let commit = self.repo.find_commit(oid)?;
            let tree = commit.tree()?;

            // Compare with parent
            let parent_tree = if commit.parent_count() > 0 {
                Some(commit.parent(0)?.tree()?)
            } else {
                None
            };

            let mut diff_opts = git2::DiffOptions::new();
            diff_opts.pathspec(file_path.as_ref());

            let diff = self.repo.diff_tree_to_tree(
                parent_tree.as_ref(),
                Some(&tree),
                Some(&mut diff_opts),
            )?;

            if diff.deltas().len() > 0 {
                ret.push(commit);
            }
        }

        Ok(ret)
    }
}

pub fn git_time_to_datetime(time: git2::Time) -> chrono::DateTime<chrono::FixedOffset> {
    let offset_seconds = time.offset_minutes() * 60;

    let offset = chrono::FixedOffset::east_opt(offset_seconds)
        .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());

    offset
        .timestamp_opt(time.seconds(), 0)
        .single()
        .expect("invalid timestamp")
}
