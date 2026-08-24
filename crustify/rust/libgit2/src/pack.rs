//! Safe wrappers for libgit2 pack APIs.

/// Wraps: git_packbuilder_foreach_cb
/// Safe callable surface for chunks emitted by a packbuilder.
///
/// The C typedef spells the chunk `void *`, but every emission site in
/// `pack-objects.c` hands over storage libgit2 keeps and reads again: an
/// object header on its own stack frame, the delta base OID inside the
/// packbuilder's object array, or the data of a cached `git_odb_object` other
/// handles may share. Each one is fed to `git_hash_update` immediately after
/// the callback returns, so the chunk is read-only for the callee and is
/// borrowed as a shared slice.
pub trait GitPackbuilderForeachCallback {
    /// Receives a transient pack-data chunk. Nonzero stops iteration.
    fn call(&mut self, buffer: &[u8]) -> i32;
}

impl<F> GitPackbuilderForeachCallback for F
where
    F: FnMut(&[u8]) -> i32,
{
    fn call(&mut self, buffer: &[u8]) -> i32 {
        self(buffer)
    }
}

/// Wraps: git_packbuilder_progress
/// Safe callable surface for packbuilder stage progress.
pub trait GitPackbuilderProgressCallback {
    /// Reports the raw stage plus current and total object counts.
    fn call(&mut self, stage: i32, current: u32, total: u32) -> i32;
}

impl<F> GitPackbuilderProgressCallback for F
where
    F: FnMut(i32, u32, u32) -> i32,
{
    fn call(&mut self, stage: i32, current: u32, total: u32) -> i32 {
        self(stage, current, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_surfaces_preserve_arguments_and_results() {
        let bytes = [1u8, 2, 3];
        let mut seen = Vec::new();
        let mut chunks = |chunk: &[u8]| {
            seen.extend_from_slice(chunk);
            chunk.len() as i32
        };
        assert_eq!(GitPackbuilderForeachCallback::call(&mut chunks, &bytes), 3);
        drop(chunks);
        assert_eq!(seen, vec![1, 2, 3]);

        let mut progress = |stage, current, total| stage + current as i32 + total as i32;
        assert_eq!(
            GitPackbuilderProgressCallback::call(&mut progress, 1, 2, 3),
            6
        );
    }
}
