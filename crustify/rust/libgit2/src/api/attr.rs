//! Safe wrappers for libgit2 attr APIs.

use core::ffi::CStr;

use crate::attr::Attribute;

/// Wraps: git_attr_foreach_cb
/// Safe callable surface for one transient attribute assignment.
pub trait GitAttrForeachCallback {
    /// Receives an attribute name and its classified value.
    ///
    /// Both borrows are valid only for this invocation. Returning nonzero
    /// stops iteration and propagates that value to the caller.
    fn call<'a>(&mut self, name: &'a CStr, value: Attribute<'a>) -> i32;
}

impl<F> GitAttrForeachCallback for F
where
    F: for<'a> FnMut(&'a CStr, Attribute<'a>) -> i32,
{
    fn call<'a>(&mut self, name: &'a CStr, value: Attribute<'a>) -> i32 {
        self(name, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_implements_attribute_callback() {
        let mut seen = |name: &CStr, value: Attribute<'_>| {
            i32::from(name == c"binary" && value == Attribute::True)
        };
        assert_eq!(
            GitAttrForeachCallback::call(&mut seen, c"binary", Attribute::True),
            1
        );
    }
}
