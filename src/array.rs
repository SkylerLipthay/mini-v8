use crate::types::Ref;

#[derive(Clone, Debug)]
pub struct Array<'mv8>(pub(crate) Ref<'mv8>);
