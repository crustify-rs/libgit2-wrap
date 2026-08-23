//! Safe wrappers for libgit2 pack APIs.

/// Wraps: git_packbuilder_foreach_cb
/// Safe callable surface for chunks emitted by a packbuilder.
pub trait GitPackbuilderForeachCallback {
    /// Receives a transient pack-data chunk. Nonzero stops iteration.
    fn call(&mut self, buffer: &mut [u8]) -> i32;
}

impl<F> GitPackbuilderForeachCallback for F
where
    F: FnMut(&mut [u8]) -> i32,
{
    fn call(&mut self, buffer: &mut [u8]) -> i32 {
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
        let mut bytes = [1, 2, 3];
        let mut chunks = |chunk: &mut [u8]| {
            chunk[0] = 9;
            chunk.len() as i32
        };
        assert_eq!(
            GitPackbuilderForeachCallback::call(&mut chunks, &mut bytes),
            3
        );
        assert_eq!(bytes[0], 9);

        let mut progress = |stage, current, total| stage + current as i32 + total as i32;
        assert_eq!(
            GitPackbuilderProgressCallback::call(&mut progress, 1, 2, 3),
            6
        );
    }
}
