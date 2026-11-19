/// Whether a class base outside the scan may call its methods.
#[derive(Debug, Clone, Copy)]
pub enum ExternalBases {
    /// Out-of-scan bases say nothing about method liveness.
    Ignored,
    /// Any out-of-scan base or interface may call methods.
    ImplyContract,
}
