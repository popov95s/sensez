use crate::spine::ir::PerfLine;

pub trait PerformanceProfile: Send + Sync {
    fn receiver_root<'a>(&self, receiver: &'a str) -> &'a str;
    fn is_mutating_call(&self, method: &str) -> bool;
    fn is_bounded_loop(&self, subject: &str) -> bool;
    fn is_external_loop_call(
        &self,
        method: &str,
        receiver: &str,
        receiver_type: Option<&str>,
        loops: &[PerfLine],
    ) -> bool;
}
