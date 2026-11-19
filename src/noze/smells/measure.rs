/// A measured value next to the threshold it was checked against.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Measure {
    value: usize,
    threshold: usize,
}

impl Measure {
    pub(crate) fn new(value: usize, threshold: usize) -> Self {
        Measure { value, threshold }
    }

    pub(crate) fn value(self) -> u32 {
        saturate(self.value)
    }

    pub(crate) fn threshold(self) -> u32 {
        saturate(self.threshold)
    }
}

fn saturate(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}
