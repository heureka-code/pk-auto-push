use std::{convert::Infallible, path::Path};

use crate::{
    git_interaction::run_git_reset_commit,
    new_push::{GitCommandError, GitInteractionError},
    waiting::{IntelligentWait, WaitingGaveUp},
};

/// A fatal, unrecoverable error that the program wasn't able to ignore.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The git commands failed too often exceeding the allowed amount.
    #[error("waiting error: {0}")]
    Waiting(#[from] WaitingGaveUp),
    /// It was impossible to reset the local working directory.
    /// Proceeding could have had unexspected changes to the repository.
    #[error("Resetting working dirctory failed: {0}")]
    FileReset(GitCommandError),
}

/// Main application loop.
/// This loop will continuosly make changes and upload those to the server.
/// 
/// After each upload the instance of [IntelligentWait] provided as parameter
/// is used to delay the next run of the loop depending of the current upload's status.
///
/// This function will only return if a fatal error occurs.
pub fn update_loop<W: IntelligentWait, I: AsRef<str>, P: Fn() -> I>(
    path: impl AsRef<Path>,
    mut wait_after: W,
    get_process_run_label: P,
) -> Result<Infallible, Error> {
    let mut inner = |path: &Path| {
        let mut maybe_diverged = false;
        loop {
            let _sheet = get_process_run_label();
            let sheet = _sheet.as_ref();
            log::info!("Start new upload process for {sheet}");
            let res = crate::new_push::cause_new_run(path, &sheet, maybe_diverged);
            match res {
                Ok(true) => {
                    log::info!("Upload process succeeded!");
                    wait_after.success();
                }
                Ok(false) => {
                    log::info!("Process skipped as nothing is to do!");
                    wait_after.skipped();
                }
                Err(err) => {
                    use crate::new_push::NewRunError;
                    match err {
                        NewRunError::LimitReached(_) => {
                            wait_after.limit_reached();
                        }
                        NewRunError::Push(push_err) => {
                            if let Err(reset_err) = run_git_reset_commit(path) {
                                log::error!(
                                    "Push and reverting last commit failed. (Shut down program!): {reset_err}"
                                );
                            } else {
                                log::info!(
                                    "Push failed but reverting last commit succeeded."
                                );
                            }
                            if let GitInteractionError::Exec(cmd) = push_err {
                                log::warn!(
                                    "Git push failed not because of rate limit so next time a pull will be executed first in case the histories have diverged: {cmd}"
                                );
                                maybe_diverged = true;
                            }
                            wait_after.limit_reached();
                        }
                        NewRunError::ResetFiles(e) => {
                            log::error!(
                                "Failed to clean working directory using git reset (Shut down program!): {e}"
                            );
                            return Err(Error::FileReset(e));
                        }
                        e => {
                            log::error!("Unexpected error occured: {e:?}");
                            wait_after.error()?;
                        }
                    }
                }
            }
        }
    };
    inner(path.as_ref())
}
