use crate::types::Ref;

#[derive(Clone, Debug)]
pub struct Function<'mv8>(pub(crate) Ref<'mv8>);
