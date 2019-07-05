use crate::SetTag;
use std::ops::{Deref, DerefMut};
use version_tag::VersionTag;

pub struct Tagged<T: ?Sized> {
    tag: VersionTag,
    value: T,
}

impl<T: ?Sized> Tagged<T> {
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
