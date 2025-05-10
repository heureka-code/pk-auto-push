use std::{path::Path, rc::Rc, time::Duration};

pub use crate::git_interaction::{
    GitCommandError, GitInteractionError, run_git_pull, run_git_push,
};
use crate::git_interaction::{run_git_add_all, run_git_commit, run_git_reset_files};

#[derive(Debug, thiserror::Error)]
pub enum NewRunError {
    #[error("Server limit of requests reached")]
    LimitReached(Rc<str>),
    #[error("Cleaning of uncommitted changes failed {0}")]
    ResetFiles(GitCommandError),
    #[error("Pulling new changes failed: {0}")]
    Pull(GitCommandError),
    #[error("Making changes to the sheet's files failed {0}")]
    MakeChanges(#[from] std::io::Error),
    #[error("Adding new changes failed: {0}")]
    AddAll(GitCommandError),
    #[error("Committing new changes failed: {0}")]
    Commit(GitCommandError),
    #[error("Pushing new changes failed: {0}")]
    Push(GitInteractionError),
}
fn new_run_err(
    specific: impl FnOnce(GitCommandError) -> NewRunError,
) -> impl FnOnce(GitInteractionError) -> NewRunError {
    move |interaction| match interaction {
        GitInteractionError::Exec(cmd) => specific(cmd),
        GitInteractionError::LimitReached(stderr) => NewRunError::LimitReached(stderr),
    }
}

/// Make a small change by swapping two files if they exist.
/// If swapping succeeds `Ok(true)` is returned and if the required files
/// weren't present (the process shouldn't run) `Ok(false)` is returned.
/// 
/// It will look into the current folder and will swap exactly one `*.cpp` and one `*.other`
/// file. If there are multiple such files or one is missing, the process is skipped.
pub fn make_changes(folder: &Path, sheet_name: &str) -> Result<bool, std::io::Error> {
    let sheet_folder = folder.join(sheet_name);
    if !sheet_folder.exists() {
        log::debug!("Sheet folder {sheet_folder:?} doesn't exist so process is skipped");
        return Ok(false);
    }

    let files: Vec<String> = sheet_folder
        .read_dir()?
        .flatten()
        .filter_map(|d| {
            if let Ok(f) = d.file_type() {
                if f.is_file() {
                    d.file_name().into_string().ok()
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    if files.len() < 2 {
        log::debug!(
            "Sheet folder {sheet_folder:?} doesn't contain enough files and is skipped: {files:?}"
        );
        return Ok(false);
    }

    let cpp_file = files.iter().find(|e| e.ends_with(".cpp"));
    let other_file = files.iter().find(|e| e.ends_with(".other"));
    if let (Some(cpp), Some(other)) = (cpp_file, other_file) {
        let cpp_path = sheet_folder.join(cpp);
        let other_path = sheet_folder.join(other);

        let cpp_content = std::fs::read_to_string(&cpp_path)?;
        let other_content = std::fs::read_to_string(&other_path)?;

        std::fs::write(cpp_path, other_content)?;
        log::trace!("Successfully wrote content of {other} to {cpp}");
        std::fs::write(other_path, cpp_content)?;
        log::trace!("Successfully wrote content of {cpp} to {other}");
        log::debug!("Successfully swapped content of {cpp} and {other}");
        Ok(true)
    } else {
        log::debug!("Required files for swapping not found. Process is skipped.");
        Ok(false)
    }
}

/// Reset files, make changes, commit and push to server.
/// 
/// If the previous push failed an optional pull can be executed.
/// This can fix the history when another instance independently pushed changes to the server.
/// After this optional pull the program will wait 10s, just to mititage rate limiting.
pub fn cause_new_run(
    folder: &Path,
    sheet_name: &str,
    prepend_pull: bool,
) -> Result<bool, NewRunError> {
    log::debug!("Start causing new server run...");
    run_git_reset_files(folder).map_err(NewRunError::ResetFiles)?;
    log::debug!("git reset local directory");
    if prepend_pull {
        run_git_pull(folder).map_err(new_run_err(NewRunError::Pull))?;
        log::info!("git pull from remote");
    }

    if !make_changes(folder, sheet_name)? {
        return Ok(false);
    }

    run_git_add_all(folder).map_err(NewRunError::AddAll)?;
    log::debug!("git add local changes");
    run_git_commit(folder, sheet_name).map_err(NewRunError::Commit)?;

    if prepend_pull {
        const TILL_PUSH: Duration = Duration::from_secs(10);

        log::debug!("git commit local changes: (sleep {TILL_PUSH:?} till pushing those changes)");
        std::thread::sleep(TILL_PUSH);
    } else {
        log::debug!("git commit local changes");
    }
    run_git_push(folder).map_err(NewRunError::Push)?;
    log::debug!("git push to remote");
    log::debug!("Causing rerun succeeded!");
    Ok(true)
}
