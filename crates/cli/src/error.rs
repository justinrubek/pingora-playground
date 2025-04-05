#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    HickoryResolver(#[from] hickory_resolver::ResolveError),
    // #[error("Hello {0}")]
    // Other(String),
    //
    #[error("unable to query srv record")]
    QuerySrvRecord,
}

pub type Result<T> = std::result::Result<T, Error>;
