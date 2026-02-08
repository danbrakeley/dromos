pub mod repository;
pub mod schema;

pub use repository::{EdgeRow, NodeMetadata, NodeRow, Repository};
pub use schema::{
    DATA_REVISION, get_stored_data_revision, has_existing_data, run_migrations, set_data_revision,
};
