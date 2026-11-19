/// Position of a file in one scan's file list.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct FileId(u32);

impl FileId {
    /// Marks a separator position in the duplication buffer, not a file.
    pub const SEPARATOR: FileId = FileId(u32::MAX);

    pub fn from_index(index: usize) -> Self {
        FileId(u32::try_from(index).expect("a scan holds fewer than u32::MAX files"))
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}
