use std::path::Path;

use anyhow::Context as _;

const DEFAULT_REF: &str = "refs/heads/main";
const REMOTE_NAME: &str = "origin";
const REMOTE_FETCH_REFS: [&str; 1] = ["refs/heads/*:refs/remotes/origin/*"];

pub struct GitRepo {
    repo: git2::Repository,
    default_ref: &'static str,
}

impl GitRepo {
    pub fn new(dir: impl AsRef<Path>, remote_url: &str) -> anyhow::Result<Self> {
        let repo = if dir.as_ref().exists() && dir.as_ref().join(".git").exists() {
            let repo = git2::Repository::open(dir)?;

            match repo.find_remote(REMOTE_NAME) {
                Ok(remote) => {
                    if remote.url() != Some(remote_url) {
                        tracing::warn!(
                            "the existing remote url is different then the provided one, existing: {}, provided: {}",
                            remote.url().unwrap_or("none"),
                            remote_url
                        );
                    }
                }
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    repo.remote_set_url(REMOTE_NAME, remote_url)?;
                }
                Err(e) => Err(e)?,
            }

            repo
        } else {
            git2::Repository::clone(remote_url, dir)?
        };

        Ok(Self {
            repo,
            default_ref: DEFAULT_REF,
        })
    }

    fn inner(&self) -> &git2::Repository {
        &self.repo
    }

    fn file_path(&self, file: &str) -> anyhow::Result<std::path::PathBuf> {
        let path = self
            .repo
            .workdir()
            .context("the git repo doesn't have workdir")?
            .join(file);
        Ok(path)
    }

    fn disable_global_git_config() -> Result<(), git2::Error> {
        unsafe {
            git2::opts::set_search_path(git2::ConfigLevel::System, "")?;
            git2::opts::set_search_path(git2::ConfigLevel::Global, "")?;
            git2::opts::set_search_path(git2::ConfigLevel::XDG, "")?;
        }
        Ok(())
    }

    pub fn current_ref(&self) -> anyhow::Result<git2::Reference<'_>> {
        Ok(self.repo.find_reference(self.default_ref)?)
    }

    pub fn current_commit(&self) -> anyhow::Result<git2::AnnotatedCommit<'_>> {
        let rf = self.current_ref()?;
        Ok(self.repo.reference_to_annotated_commit(&rf)?)
    }

    pub fn fetch(&self) -> anyhow::Result<git2::AnnotatedCommit<'_>> {
        let mut remote = self.repo.find_remote(REMOTE_NAME)?;
        remote.fetch(&REMOTE_FETCH_REFS, None, None)?;

        let fetch_head = self.repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;

        Ok(fetch_commit)
    }

    pub fn fast_forward(
        &self,
        lb: &mut git2::Reference,
        rc: &git2::AnnotatedCommit,
    ) -> Result<(), git2::Error> {
        let name = match lb.name() {
            Some(s) => s.to_string(),
            None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
        };
        let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
        println!("{}", msg);
        lb.set_target(rc.id(), &msg)?;
        self.repo.set_head(&name)?;
        self.repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default()
                // TODO: since i just copy this from example :P
                // For some reason the force is required to make the working directory actually get updated
                // I suspect we should be adding some logic to handle dirty working directory states
                // but this is just an example so maybe not.
                .force(),
        ))?;
        Ok(())
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
