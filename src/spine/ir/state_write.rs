//! Writes to state that outlives one call, as a language profile sees them.

/// What one recorded write touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StateWriteKind {
    /// Assignment to an own-instance member (`this.x = …`).
    OwnMember,
    /// Assignment to a name declared outside the function.
    OuterName,
    /// An own-member or outer-name assignment guarded by a check of the same target.
    LazyInit,
    /// A network request with a method other than a read.
    Request,
}

/// One write that is not a method call on a receiver.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StateWrite {
    pub line: usize,
    pub target: String,
    pub kind: StateWriteKind,
}
