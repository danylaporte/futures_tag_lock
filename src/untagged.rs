use crate::SetTag;
use std::ops::{Deref, DerefMut};
use version_tag::VersionTag;

pub struct Untagged<T: ?Sized> {
    value: T,
}

impl<T> Untagged<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: ?Sized> Deref for Untagged<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: ?Sized> DerefMut for Untagged<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: ?Sized> SetTag for Untagged<T> {
    fn set_tag(&mut self, _: VersionTag) {}
}
