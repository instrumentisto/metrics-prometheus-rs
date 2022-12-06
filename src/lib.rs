pub mod failure;
pub mod metric;
pub mod recorder;
pub mod storage;

#[doc(inline)]
pub use self::{metric::Metric, recorder::Recorder};

pub fn register() -> Result<Recorder, metrics::SetRecorderError> {
    Recorder::new().register()
}
