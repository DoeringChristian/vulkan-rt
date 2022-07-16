mod gltf;
pub use self::gltf::*;

use std::path::Path;

use crate::model::{InstanceKey, ShaderGroupKey};

pub trait Loader<T> {
    type Ctx;
    fn load_to(
        &self,
        path: impl AsRef<Path>,
        ctx: &Self::Ctx,
        dst: &T,
        default_hit_groups: Vec<ShaderGroupKey>,
    ) -> Vec<InstanceKey>;
}
