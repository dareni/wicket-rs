pub struct MountedMapper {}
pub struct PackageMapper {}
pub struct ResourceMapper {}
pub struct BookmarkableMapper {}

#[derive(Default)]
pub struct SystemMapper {}

impl SystemMapper {
    pub fn new() -> Self {
        Self::default()
    }
}

