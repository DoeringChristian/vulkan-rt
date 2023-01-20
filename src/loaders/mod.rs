mod gltf;
pub use self::gltf::*;

use std::path::Path;

pub trait Loader<T> {
    fn append(&self, path: impl AsRef<Path>, dst: &mut T) -> usize;
}
