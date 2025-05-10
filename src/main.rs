//! Code: [https://github.com/heureka-code/pk-auto-push](https://github.com/heureka-code/pk-auto-push)

use std::{path::PathBuf, process::exit, str::FromStr, time::Duration};

pub(crate) mod git_interaction;
mod looping;
mod new_push;
mod sheet_name;
mod waiting;
use looping::update_loop;
use waiting::DefaultWaiter;

fn main() {
    dotenvy::dotenv().unwrap();
    env_logger::init();
    let path = PathBuf::from_str(&std::env::var("REPO_PATH").expect("Environment variable 'REPO_PATH' missing")).unwrap();
    if path.exists() {
        log::info!("The provided repository path exists. Program will start");
    } else {
        log::error!("The provided path doesn't exist {path:?}. Program will terminate");
        exit(2);
    }
    let res = update_loop(
        &path,
        DefaultWaiter::new(
            Duration::from_secs(7),
            Duration::from_secs(5 * 60),
            Duration::from_secs(30 * 60),
            10,
        ),
        sheet_name::get_current_sheet_name,
    );
    let Err(err) = res;
    log::error!("End of program reached: {err}");
    exit(1);
}
