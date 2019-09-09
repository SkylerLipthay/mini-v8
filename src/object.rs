use crate::types::Ref;

#[derive(Clone, Debug)]
pub struct Object<'mv8>(pub(crate) Ref<'mv8>);
