mod gltf;
pub use self::gltf::*;

use std::path::Path;

pub trait Loader<T> {
    fn load(&self, path: impl AsRef<Path>, dst: &mut T);
}
