use tx_error::{CodeMsg};
/// Repository error type
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("REPOSITORY")]
pub enum RepositoryError {
    #[err(1000, "记录不存在")]
    Database,

    #[err(1001,"Not found")]
    NotFound,

    #[err(1002,"Duplicate entry")]
    Duplicate,

    #[err(1003,"Validation error")]
    Validation,

    #[err("Internal error")]
    Internal,
}
