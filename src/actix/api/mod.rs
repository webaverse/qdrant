pub mod cluster_api;
pub mod collections_api;
pub mod count_api;
pub mod read_params;
pub mod recommend_api;
pub mod retrieve_api;
pub mod search_api;
pub mod service_api;
pub mod snapshot_api;
pub mod update_api;

use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
struct CollectionPath {
    #[validate(length(min = 1, max = 255))]
    name: String,
}
