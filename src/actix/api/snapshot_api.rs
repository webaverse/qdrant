use actix_files::NamedFile;
use actix_web::rt::time::Instant;
use actix_web::web::Query;
use actix_web::{delete, get, post, put, web, Responder, Result};
use collection::operations::snapshot_ops::SnapshotRecover;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use storage::content_manager::snapshots::recover::do_recover_from_snapshot;
use storage::content_manager::snapshots::{
    do_create_full_snapshot, do_delete_collection_snapshot, do_delete_full_snapshot,
    do_list_full_snapshots, get_full_snapshot_path,
};
use storage::content_manager::toc::TableOfContent;
use storage::dispatcher::Dispatcher;

use crate::actix::helpers::{
    collection_into_actix_error, process_response, storage_into_actix_error,
};
use crate::common::collections::*;

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct SnapshottingParam {
    pub wait: Option<bool>,
}

// Actix specific code
pub async fn do_get_full_snapshot(toc: &TableOfContent, snapshot_name: &str) -> Result<NamedFile> {
    let file_name = get_full_snapshot_path(toc, snapshot_name)
        .await
        .map_err(storage_into_actix_error)?;

    Ok(NamedFile::open(file_name)?)
}

// Actix specific code
pub async fn do_get_snapshot(
    toc: &TableOfContent,
    collection_name: &str,
    snapshot_name: &str,
) -> Result<NamedFile> {
    let collection = toc
        .get_collection(collection_name)
        .await
        .map_err(storage_into_actix_error)?;

    let file_name = collection
        .get_snapshot_path(snapshot_name)
        .await
        .map_err(collection_into_actix_error)?;

    Ok(NamedFile::open(file_name)?)
}

#[get("/collections/{name}/snapshots")]
async fn list_snapshots(toc: web::Data<TableOfContent>, path: web::Path<String>) -> impl Responder {
    let collection_name = path.into_inner();

    let timing = Instant::now();
    let response = do_list_snapshots(toc.get_ref(), &collection_name).await;
    process_response(response, timing)
}

#[post("/collections/{name}/snapshots")]
async fn create_snapshot(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
) -> impl Responder {
    let collection_name = path.into_inner();

    let timing = Instant::now();
    let response = do_create_snapshot(toc.get_ref(), &collection_name).await;
    process_response(response, timing)
}

#[put("/collections/{name}/snapshots/recover")]
async fn recover_from_snapshot(
    dispatcher: web::Data<Dispatcher>,
    path: web::Path<String>,
    request: web::Json<SnapshotRecover>,
    params: Query<SnapshottingParam>,
) -> impl Responder {
    let collection_name = path.into_inner();
    let snapshot_recover = request.into_inner();
    let wait = params.wait.unwrap_or(true);

    let timing = Instant::now();
    let response = do_recover_from_snapshot(
        dispatcher.get_ref(),
        &collection_name,
        snapshot_recover,
        wait,
    )
    .await;
    process_response(response, timing)
}

#[get("/collections/{name}/snapshots/{snapshot_name}")]
async fn get_snapshot(
    toc: web::Data<TableOfContent>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (collection_name, snapshot_name) = path.into_inner();
    do_get_snapshot(toc.get_ref(), &collection_name, &snapshot_name).await
}

#[get("/snapshots")]
async fn list_full_snapshots(toc: web::Data<TableOfContent>) -> impl Responder {
    let timing = Instant::now();
    let response = do_list_full_snapshots(toc.get_ref()).await;
    process_response(response, timing)
}

#[post("/snapshots")]
async fn create_full_snapshot(toc: web::Data<TableOfContent>) -> impl Responder {
    let timing = Instant::now();
    let response = do_create_full_snapshot(toc.get_ref()).await;
    process_response(response, timing)
}

#[get("/snapshots/{snapshot_name}")]
async fn get_full_snapshot(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
) -> impl Responder {
    let snapshot_name = path.into_inner();
    do_get_full_snapshot(toc.get_ref(), &snapshot_name).await
}

#[delete("/snapshots/{snapshot_name}")]
async fn delete_full_snapshot(
    toc: web::Data<TableOfContent>,
    path: web::Path<String>,
) -> impl Responder {
    let snapshot_name = path.into_inner();
    let timing = Instant::now();
    let response = do_delete_full_snapshot(toc.get_ref(), &snapshot_name).await;
    process_response(response, timing)
}

#[delete("/collections/{name}/snapshots/{snapshot_name}")]
async fn delete_collection_snapshot(
    toc: web::Data<TableOfContent>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (collection_name, snapshot_name) = path.into_inner();
    let timing = Instant::now();
    let response =
        do_delete_collection_snapshot(toc.get_ref(), &collection_name, &snapshot_name).await;
    process_response(response, timing)
}

// Configure services
pub fn config_snapshots_api(cfg: &mut web::ServiceConfig) {
    cfg.service(list_snapshots)
        .service(create_snapshot)
        .service(recover_from_snapshot)
        .service(get_snapshot)
        .service(list_full_snapshots)
        .service(create_full_snapshot)
        .service(get_full_snapshot)
        .service(delete_full_snapshot)
        .service(delete_collection_snapshot);
}
