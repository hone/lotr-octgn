use std::path::{Path, PathBuf};

use git2::Repository;

pub struct GitCache<'a> {
    git_url: String,
    cache_dir: &'a Path,
    pub sets_dir: PathBuf,
}

impl<'a> GitCache<'a> {
    pub fn new(git_url: String, cache_dir: &'a Path) -> Self {
        let sets_dir = cache_dir.join("o8g").join("Sets");

        GitCache {
            git_url,
            cache_dir,
            sets_dir,
        }
    }

    fn update(&self) -> Result<(), git2::Error> {
        let git_dir = self.cache_dir.join(".git");
        let repo = Repository::open(git_dir)?;
        let mut remote = repo.find_remote("origin")?;
        remote.fetch(&["master"], None, None)?;

        let branch_error = git2::Error::from_str("Couldn't find branch 'origin/master'");
        let origin_master_tuple = repo
            .branches(Some(git2::BranchType::Remote))?
            .find(|branch_tuple| {
                let (branch, _branch_type) = branch_tuple.as_ref().unwrap();
                branch.name().unwrap().unwrap() == "origin/master"
            })
            .ok_or(branch_error)?;

        let (origin_master, _branch_type) = origin_master_tuple?;

        repo.checkout_tree(
            &origin_master.into_reference().peel_to_tree()?.as_object(),
            None,
        )?;

        Ok(())
    }

    pub fn update_or_fetch(&self) -> Result<(), Box<std::error::Error>> {
        self.update().or_else(|_err| {
            fs_extra::dir::remove(&self.cache_dir)?;
            Repository::clone(&self.git_url, &self.cache_dir)?;

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write};

    use fs_extra::dir;
    use tempdir::TempDir;

    const GIT_URL: &str = "fixtures/octgn";

    #[test]
    fn test_fetch_or_update_octgn_git_dir_new() {
        let tmp_dir = TempDir::new("octgn").unwrap();

        let git_cache = GitCache::new(GIT_URL.to_string(), &tmp_dir.path());
        let result = git_cache.update_or_fetch();
        assert!(result.is_ok());
        assert!(tmp_dir.path().join("LotR set editor.xlsm").exists());
    }

    #[test]
    fn test_fetch_or_update_octgn_git_dir_existing() {
        let tmp_dir = TempDir::new("octgn").unwrap();
        let cache_git_dir = tmp_dir.path().join("cache");
        // must be called "fixtures/octgn", so the git repo is valid
        let origin_git_dir = tmp_dir.path().join("fixtures/octgn");
        let git_cache = GitCache::new(GIT_URL.to_string(), &cache_git_dir);
        let mut copy_options = dir::CopyOptions::new();
        copy_options.copy_inside = true;

        // need to create this directory structure due to git submodule path
        dir::copy(GIT_URL, &tmp_dir, &copy_options).unwrap();
        dir::move_dir(
            &tmp_dir.path().join("octgn"),
            &origin_git_dir,
            &copy_options,
        )
        .unwrap();

        // due to submodules need to create relative path to submodule .git directory
        let submodule_git_dir = tmp_dir
            .path()
            .join(".git")
            .join("modules")
            .join("fixtures")
            .join("octgn");
        dir::copy(
            ".git/modules/fixtures/octgn",
            &submodule_git_dir,
            &copy_options,
        )
        .unwrap();

        Repository::clone(&origin_git_dir.to_str().unwrap(), &cache_git_dir).unwrap();

        let mut file = File::create(&origin_git_dir.join("new_file.txt")).unwrap();
        file.write_all(b"New File").unwrap();

        // add new commit
        let origin_repo = Repository::open(&origin_git_dir).unwrap();
        let head_ref = origin_repo.head().unwrap();
        let head_commit = head_ref.peel_to_commit().unwrap();
        let mut index = origin_repo.index().unwrap();
        let add_file_path = Path::new("new_file.txt");
        index.add_path(&add_file_path).unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = origin_repo.find_tree(tree_id).unwrap();
        let sig = origin_repo.signature().unwrap();
        origin_repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Add New File",
                &tree,
                &[&head_commit],
            )
            .unwrap();

        let result = git_cache.update_or_fetch();
        assert!(result.is_ok());
        assert!(cache_git_dir.join("new_file.txt").exists());
    }

    #[test]
    fn test_fetch_or_update_octgn_git_dir_bad_dir() {
        let tmp_dir = TempDir::new("octgn").unwrap();
        let repo = Repository::clone(GIT_URL, &tmp_dir.path()).unwrap();
        repo.remote_delete("origin").unwrap();
        let git_cache = GitCache::new(GIT_URL.to_string(), &tmp_dir.path());

        let result = git_cache.update_or_fetch();
        assert!(result.is_ok());
        assert!(tmp_dir.path().join("LotR set editor.xlsm").exists());
    }
}
