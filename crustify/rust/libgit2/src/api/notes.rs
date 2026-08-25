//! Safe wrappers for libgit2 notes APIs.

/// Wraps: git_note_foreach_cb
/// Safe callable surface for a synchronous note traversal.
pub trait GitNoteForeachCallback {
    /// Visits the note blob ID and the annotated object ID.
    fn call(
        &mut self,
        blob_id: crate::oid::OidRef<'_>,
        annotated_object_id: crate::oid::OidRef<'_>,
    ) -> i32;
}

impl<F> GitNoteForeachCallback for F
where
    F: FnMut(crate::oid::OidRef<'_>, crate::oid::OidRef<'_>) -> i32,
{
    fn call(
        &mut self,
        blob_id: crate::oid::OidRef<'_>,
        annotated_object_id: crate::oid::OidRef<'_>,
    ) -> i32 {
        self(blob_id, annotated_object_id)
    }
}
