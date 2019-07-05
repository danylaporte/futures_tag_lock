use crate::SetTag;
use std::ops::{Deref, DerefMut};
use version_tag::VersionTag;

/// A wrapper class that handle the version tag and the value.
/// 
/// # Example
///
/// ```
/// use futures::Future;
/// use futures_tag_locks::{RwLock, Tagged};
/// use tokio::executor::current_thread::block_on_all;
///
/// let lock = RwLock::new(Tagged::new(10));
/// let old_tag = block_on_all(lock.read().map(|t| t.tag())).unwrap();
///
/// block_on_all(lock.write().map(|mut w| {
///     // the tag should not have been changed here.
///     assert_eq!(old_tag, w.tag());
///     
///     // get the actual value in the lock
///     assert_eq!(10, **w);
///
///     // set the value in the lock
///     **w = 12;
///     
///     // after this write access, the tagged value will be marked with
///     // this new tag.
///     let _ = w.new_tag();
/// })).unwrap();
/// ```
pub struct Tagged<T: ?Sized> {
    tag: VersionTag,
    value: T,
}

impl<T> Tagged<T> {
    pub fn new(value: T) -> Self {
        Self {
            tag: VersionTag::new(),
            value,
        }
    }

    pub fn tag(&self) -> VersionTag {
        self.tag
    }
}

impl<T: ?Sized> Deref for Tagged<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: ?Sized> DerefMut for Tagged<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: ?Sized> SetTag for Tagged<T> {
    fn set_tag(&mut self, tag: VersionTag) {
        self.tag = tag;
    }
}
