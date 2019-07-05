use version_tag::VersionTag;

pub trait SetTag {
    fn set_tag(&mut self, tag: VersionTag);
}

impl<T: SetTag> SetTag for Option<T> {
    fn set_tag(&mut self, tag: VersionTag) {
        if let Some(v) = self {
            v.set_tag(tag);
        }
    }
}
