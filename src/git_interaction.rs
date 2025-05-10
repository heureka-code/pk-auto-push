use std::{
    path::Path,
    process::{Command, ExitStatus},
    rc::Rc,
};

/// This wraps the error that occurs when a git command failed to execute.
/// It doesn't include rate-limiting, for this see [GitInteractionError].
#[derive(Debug, thiserror::Error)]
pub enum GitCommandError {
    /// Capturing the output of the command or issuing at all failed
    #[error("Failed to execute git command {0}")]
    Exec(#[from] std::io::Error),
    /// The command ran but exited with a non-zero status code
    #[error("Unknown git error occured: status={0}, stderr={1:?}")]
    Other(ExitStatus, Rc<str>),
}

/// Error wrapping git commands that communicate with the server.
/// 
/// If a git push/pull command failes, the error message is scanned for the patterns
/// `"Connection refused"` and `"ssh:"`. If both are found the error is considered
/// a rate limitation.
#[derive(Debug, thiserror::Error)]
pub enum GitInteractionError {
    /// The git command failed but it doesn't seem to be the rate limit that caused it.
    #[error("Failed to execute cli command {0}")]
    Exec(#[from] GitCommandError),
    /// The git command failed and it's assumed that it was caused by the rate limit.
    #[error("Server limit of requests reached")]
    LimitReached(Rc<str>),
}

fn run_git_server_command(folder: &Path, command: &mut Command) -> Result<(), GitInteractionError> {
    let output = command
        .current_dir(folder)
        .output()
        .map_err(GitCommandError::Exec)?;

    // let stdout = String::from_utf8(output.stdout);
    if output.status.success() {
        Ok(())
    } else {
        let stderr: Rc<str> = String::from_utf8_lossy(&output.stderr).into();
        Err(
            if stderr.contains("Connection refused") && stderr.contains("ssh:") {
                GitInteractionError::LimitReached(stderr)
            } else {
                GitCommandError::Other(output.status, stderr).into()
            },
        )
    }
}
pub fn run_git_local_command(folder: &Path, command: &mut Command) -> Result<(), GitCommandError> {
    let output = command.current_dir(folder).output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr: Rc<str> = String::from_utf8_lossy(&output.stderr).into();
        Err(GitCommandError::Other(output.status, stderr))
    }
}

/// run `git reset --hard`
pub fn run_git_reset_files(folder: &Path) -> Result<(), GitCommandError> {
    run_git_local_command(folder, Command::new("git").args(["reset", "--hard"]))
}

/// run `git reset HEAD~`
/// 
/// This will delete the last local commit so that the history from the server can be taken to overwrite the local one.
pub fn run_git_reset_commit(folder: &Path) -> Result<(), GitCommandError> {
    run_git_local_command(folder, Command::new("git").args(["reset", "HEAD~"]))
}

/// run `git add --all`
pub fn run_git_add_all(folder: &Path) -> Result<(), GitCommandError> {
    run_git_local_command(folder, Command::new("git").args(["add", "--all"]))
}
/// run `git commit -m "..."` with a message indicating the automatic push
pub fn run_git_commit(folder: &Path, sheet_name: &str) -> Result<(), GitCommandError> {
    run_git_local_command(
        folder,
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(format!("[automatic] push for rerun of {sheet_name}")),
    )
}

/// run `git pull origin main`
pub fn run_git_pull<P: AsRef<Path>>(folder: P) -> Result<(), GitInteractionError> {
    run_git_server_command(
        folder.as_ref(),
        Command::new("git").args(["pull", "origin", "main"]),
    )
}

/// run `git push origin main`
pub fn run_git_push<P: AsRef<Path>>(folder: P) -> Result<(), GitInteractionError> {
    run_git_server_command(
        folder.as_ref(),
        Command::new("git").args(["push", "origin", "main"]),
    )
}
