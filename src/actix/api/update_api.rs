use actix_web::rt::time::Instant;
use actix_web::web::Query;
use actix_web::{delete, post, put, web, Responder};
use collection::operations::payload_ops::{DeletePayload, SetPayload};
use collection::operations::point_ops::{PointInsertOperations, PointsSelector, WriteOrdering};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use storage::content_manager::toc::TableOfContent;

use crate::actix::helpers::process_response;
use crate::common::points::{
    do_clear_payload, do_create_index, do_delete_index, do_delete_payload, do_delete_points,
    do_overwrite_payload, do_set_payload, do_upsert_points, CreateFieldIndex,
};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct UpdateParam {
    pub wait: Option<bool>,
    pub ordering: Option<WriteOrdering>,
}

#[put("/collections/{name}/points")]
pub async fn upsert_points(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
    operation: web::Json<PointInsertOperations>,
    params: Query<UpdateParam>,
) -> impl Responder {
    let collection_name = path.into_inner();
    let operation = operation.into_inner();
    let wait = params.wait.unwrap_or(false);
    let ordering = params.ordering.unwrap_or_default();
    let timing = Instant::now();

    let response = do_upsert_points(
        toc.get_ref(),
        &collection_name,
        operation,
        None,
        wait,
        ordering,
    )
    .await;
    process_response(response, timing)
}

#[post("/collections/{name}/points/delete")]
pub async fn delete_points(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
    operation: web::Json<PointsSelector>,
    params: Query<UpdateParam>,
) -> impl Responder {
    let collection_name = path.into_inner();
    let operation = operation.into_inner();
    let wait = params.wait.unwrap_or(false);
    let ordering = params.ordering.unwrap_or_default();
    let timing = Instant::now();

    let response = do_delete_points(
        toc.get_ref(),
        &collection_name,
        operation,
        None,
        wait,
        ordering,
    )
    .await;
    process_response(response, timing)
}

#[post("/collections/{name}/points/payload")]
pub async fn set_payload(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
    operation: web::Json<SetPayload>,
    params: Query<UpdateParam>,
) -> impl Responder {
    let collection_name = path.into_inner();
    let operation = operation.into_inner();
    let wait = params.wait.unwrap_or(false);
    let ordering = params.ordering.unwrap_or_default();
    let timing = Instant::now();

    let response = do_set_payload(
        toc.get_ref(),
        &collection_name,
        operation,
        None,
        wait,
        ordering,
    )
    .await;
    process_response(response, timing)
}

#[put("/collections/{name}/points/payload")]
pub async fn overwrite_payload(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
    operation: web::Json<SetPayload>,
    params: Query<UpdateParam>,
) -> impl Responder {
    let collection_name = path.into_inner();
    let operation = operation.into_inner();
    let wait = params.wait.unwrap_or(false);
    let ordering = params.ordering.unwrap_or_default();
    let timing = Instant::now();

    let response = do_overwrite_payload(
        toc.get_ref(),
        &collection_name,
        operation,
        None,
        wait,
        ordering,
    )
    .await;
    process_response(response, timing)
}

#[post("/collections/{name}/points/payload/delete")]
pub async fn delete_payload(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
    operation: web::Json<DeletePayload>,
    params: Query<UpdateParam>,
) -> impl Responder {
    let collection_name = path.into_inner();
    let operation = operation.into_inner();
    let wait = params.wait.unwrap_or(false);
    let ordering = params.ordering.unwrap_or_default();
    let timing = Instant::now();

    let response = do_delete_payload(
        toc.get_ref(),
        &collection_name,
        operation,
        None,
        wait,
        ordering,
    )
    .await;
    process_response(response, timing)
}

#[post("/collections/{name}/points/payload/clear")]
pub async fn clear_payload(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
    operation: web::Json<PointsSelector>,
    params: Query<UpdateParam>,
) -> impl Responder {
    let collection_name = path.into_inner();
    let operation = operation.into_inner();
    let wait = params.wait.unwrap_or(false);
    let ordering = params.ordering.unwrap_or_default();
    let timing = Instant::now();

    let response = do_clear_payload(
        toc.get_ref(),
        &collection_name,
        operation,
        None,
        wait,
        ordering,
    )
    .await;
    process_response(response, timing)
}

#[put("/collections/{name}/index")]
pub async fn create_field_index(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
    operation: web::Json<CreateFieldIndex>,
    params: Query<UpdateParam>,
) -> impl Responder {
    let collection_name = path.into_inner();
    let operation = operation.into_inner();
    let wait = params.wait.unwrap_or(false);
    let ordering = params.ordering.unwrap_or_default();
    let timing = Instant::now();

    let response = do_create_index(
        toc.get_ref(),
        &collection_name,
        operation,
        None,
        wait,
        ordering,
    )
    .await;
    process_response(response, timing)
}

#[delete("/collections/{name}/index/{field_name}")]
pub async fn delete_field_index(
    toc: web::Data<TableOfContent>,
    path: web::Path<(String, String)>,
    params: Query<UpdateParam>,
) -> impl Responder {
    let (collection_name, field_name) = path.into_inner();
    let wait = params.wait.unwrap_or(false);
    let ordering = params.ordering.unwrap_or_default();
    let timing = Instant::now();

    let response = do_delete_index(
        toc.get_ref(),
        &collection_name,
        field_name,
        None,
        wait,
        ordering,
    )
    .await;
    process_response(response, timing)
}

// Configure services
pub fn config_update_api(cfg: &mut web::ServiceConfig) {
    cfg.service(upsert_points)
        .service(delete_points)
        .service(set_payload)
        .service(overwrite_payload)
        .service(delete_payload)
        .service(clear_payload)
        .service(create_field_index)
        .service(delete_field_index);
}
