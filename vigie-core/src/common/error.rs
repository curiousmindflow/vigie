use thiserror::Error;

#[derive(Debug, Error)]
pub enum VigieError {
    #[error("The protocol period is not over yet")]
    PeriodNotOver,
    #[error("The timeout is not reached yet")]
    TimeoutNotReached,
}
