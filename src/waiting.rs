use std::time::Duration;

/// The waiting implementation returns this error to indicate that the program should end
/// due to too many errors, so waiting longer would be pointless.
#[derive(Debug, thiserror::Error)]
pub enum WaitingGaveUp {
    /// Number of consecutive errors wrapped
    #[error("After {0} consecutive errors the waiting process gave up.")]
    Errors(u32),
}

/// This trait can be implemented for objects that can wait between different runs depending on the current status.
/// 
/// The implementation is free to choose any waiting duration and have internal counters to measure errors.
pub trait IntelligentWait {
    /// The last run succeeded, wait a specific time.
    fn success(&mut self);
    /// The last run was skipped as requirements weren't satisfied, wait a specific time.
    fn skipped(&mut self);
    /// The rate limit was hit. Wait a bit longer.
    fn limit_reached(&mut self);
    /// An unexpected but recoverable error occured, wait or terminate program with error.
    fn error(&mut self) -> Result<(), WaitingGaveUp>;
    /// The other methods should use this for waiting a specific duration. Defaults to [std::thread::sleep]
    fn _wait(&self, status: &str, duration: std::time::Duration) {
        log::debug!("[status={status}] wait {duration:?} till next run.");
        std::thread::sleep(duration);
    }
}

/// Default implementation of [IntelligentWait] that exposes some parameters
pub struct DefaultWaiter {
    after_success: Duration,
    after_error: Duration,
    after_skipped: Duration,
    consecutive_errors: u32,
    consecutive_limits: u32,
    max_error_retry: u32,
}
impl DefaultWaiter {
    pub fn new(
        after_success: Duration,
        after_error: Duration,
        after_skipped: Duration,
        max_error_retry: u32,
    ) -> DefaultWaiter {
        DefaultWaiter {
            after_success,
            after_error,
            after_skipped,
            max_error_retry,
            consecutive_errors: 0,
            consecutive_limits: 0,
        }
    }
}
impl IntelligentWait for DefaultWaiter {
    /// Successful run: reset error and limit counters, wait [Self::after_success] long
    fn success(&mut self) {
        self.consecutive_errors = 0;
        self.consecutive_limits = 0;
        self._wait("success", self.after_success);
    }
    /// Skipped run: reset error and limit counters, wait [Self::after_skipped] long
    fn skipped(&mut self) {
        self.consecutive_errors = 0;
        self.consecutive_limits = 0;
        self._wait("skipped", self.after_skipped);
    }
    /// Rate limit reached, wait increasingly longer till next successfull run and then continue
    /// with previous normal duration.
    /// 
    /// If [Self::after_success] is too small this will be called often and may slow down the overall process.
    fn limit_reached(&mut self) {
        self.consecutive_limits += 1;
        log::warn!(
            "Server limit reached, consecutive error count: {}",
            self.consecutive_limits
        );
        self._wait(
            "limit-reached",
            self.after_success * (self.consecutive_limits + 1),
        );
    }
    /// Increase error counter. Terminate after [Self::max_error_retry] errors.
    /// Wait increasingly longer in each step.
    fn error(&mut self) -> Result<(), WaitingGaveUp> {
        self.consecutive_errors += 1;
        let dur = if self.consecutive_errors > self.max_error_retry {
            log::error!(
                "The maximum number of allowed retrying is exceeded ({}/{}). Give up!",
                self.consecutive_errors,
                self.max_error_retry
            );
            return Err(WaitingGaveUp::Errors(self.consecutive_errors));
        } else {
            let dur = self.after_error * (self.consecutive_errors + 1);
            log::warn!(
                "Start retry {} of {} allowed. Wait {dur:?}",
                self.consecutive_errors,
                self.max_error_retry
            );
            dur
        };
        self._wait("error", dur);
        Ok(())
    }
}
