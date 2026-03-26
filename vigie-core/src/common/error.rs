use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("The protocol period is not over yet")]
    PeriodNotOver,
    #[error("The timeout is not reached yet")]
    TimeoutNotReached,
}
