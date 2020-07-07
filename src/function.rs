use crate::*;

/// Reference to a JavaScript function.
#[derive(Clone)]
pub struct Function<'mv8>(pub(crate) Ref<'mv8>);
