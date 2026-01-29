/// Abort signal sent when execution should be terminated.
/// Can be sent by any module (executor, tracker, radio, etc.).
#[derive(Debug, Clone)]
pub struct AbortSignal {
    pub step: usize,
    pub reason: String,
}
