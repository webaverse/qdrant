use tonic_build::Builder;

fn main() -> std::io::Result<()> {
    // Build gRPC bits from proto file
    tonic_build::configure()
        // Because we want to attach all validation rules to the generated gRPC types, we must do
        // so by extending the builder. This is ugly, but better than manually implementing
        // `Validation` for all these types and seems to be the best approach. The line below
        // configures all attributes.
        .configure_validation()
        .out_dir("src/grpc/") // saves generated structures at this location
        .compile(
            &["src/grpc/proto/qdrant.proto"], // proto entry point
            &["src/grpc/proto"], // specify the root location to search proto dependencies
        )?;

    // Append trait extension imports to generated gRPC output
    append_to_file("src/grpc/qdrant.rs", "use super::validate::ValidateExt;");

    Ok(())
}

/// Extension to [`Builder`] to configure validation attributes.
trait BuilderExt {
    fn configure_validation(self) -> Self;
    fn validates(self, fields: &[(&str, &str)], extra_derives: &[&str]) -> Self;
    fn derive_validate(self, path: &str) -> Self;
    fn derive_validates(self, paths: &[&str]) -> Self;
    fn field_validate(self, path: &str, constraint: &str) -> Self;
    fn field_validates(self, paths: &[(&str, &str)]) -> Self;
}

impl BuilderExt for Builder {
    fn configure_validation(self) -> Self {
        configure_validation(self)
    }

    fn validates(self, fields: &[(&str, &str)], extra_derives: &[&str]) -> Self {
        // Build list of structs to derive validation on, guess these from list of fields
        let mut derives = fields
            .iter()
            .map(|(field, _)| field.split_once('.').unwrap().0)
            .collect::<Vec<&str>>();
        derives.extend(extra_derives);
        derives.sort_unstable();
        derives.dedup();

        self.derive_validates(&derives).field_validates(fields)
    }

    fn derive_validate(self, path: &str) -> Self {
        self.type_attribute(path, "#[derive(validator::Validate)]")
    }

    fn derive_validates(self, paths: &[&str]) -> Self {
        paths.iter().fold(self, |c, path| c.derive_validate(path))
    }

    fn field_validate(self, path: &str, constraint: &str) -> Self {
        if constraint.is_empty() {
            self.field_attribute(path, "#[validate]")
        } else {
            self.field_attribute(path, format!("#[validate({constraint})]"))
        }
    }

    fn field_validates(self, fields: &[(&str, &str)]) -> Self {
        fields.iter().fold(self, |c, (path, constraint)| {
            c.field_validate(path, constraint)
        })
    }
}

/// Configure additional attributes required for validation on generated gRPC types.
///
/// These are grouped by service file.
#[rustfmt::skip]
fn configure_validation(builder: Builder) -> Builder {
    builder
        // Service: collections.proto
        .validates(&[
            ("GetCollectionInfoRequest.collection_name", "length(min = 1, max = 255)"),
            ("CreateCollection.collection_name", "length(min = 1, max = 255)"),
            ("CreateCollection.hnsw_config", ""),
            ("CreateCollection.wal_config", ""),
            ("CreateCollection.optimizers_config", ""),
            ("CreateCollection.vectors_config", ""),
            ("UpdateCollection.collection_name", "length(min = 1, max = 255)"),
            ("UpdateCollection.optimizers_config", ""),
            ("UpdateCollection.params", ""),
            ("UpdateCollection.timeout", "custom = \"crate::grpc::validate::validate_u64_range_min_1\""),
            ("DeleteCollection.collection_name", "length(min = 1, max = 255)"),
            ("DeleteCollection.timeout", "custom = \"crate::grpc::validate::validate_u64_range_min_1\""),
            ("ChangeAliases.timeout", "custom = \"crate::grpc::validate::validate_u64_range_min_1\""),
            ("ListCollectionAliasesRequest.collection_name", "length(min = 1, max = 255)"),
            ("HnswConfigDiff.ef_construct", "custom = \"crate::grpc::validate::validate_u64_range_min_4\""),
            ("WalConfigDiff.wal_capacity_mb", "custom = \"crate::grpc::validate::validate_u64_range_min_1\""),
            ("OptimizersConfigDiff.deleted_threshold", "custom = \"crate::grpc::validate::validate_f64_range_1\""),
            ("OptimizersConfigDiff.vacuum_min_vector_number", "custom = \"crate::grpc::validate::validate_u64_range_min_100\""),
            ("OptimizersConfigDiff.memmap_threshold", "custom = \"crate::grpc::validate::validate_u64_range_min_1000\""),
            ("OptimizersConfigDiff.indexing_threshold", "custom = \"crate::grpc::validate::validate_u64_range_min_1000\""),
            ("VectorsConfig.config", ""),
            ("VectorParams.size", "range(min = 1)"),
            ("VectorParamsMap.map", ""),
        ], &[
            "ListCollectionsRequest",
            "CollectionParamsDiff",
            "ListAliasesRequest",
        ])
        // Service: collections_internal.proto
        .validates(&[
            ("GetCollectionInfoRequestInternal.get_collection_info_request", ""),
            ("InitiateShardTransferRequest.collection_name", "length(min = 1, max = 255)"),
        ], &[])
        // Service: points.proto
        .validates(&[
            ("UpsertPoints.collection_name", "length(min = 1, max = 255)"),
            ("DeletePoints.collection_name", "length(min = 1, max = 255)"),
            ("GetPoints.collection_name", "length(min = 1, max = 255)"),
            ("SetPayloadPoints.collection_name", "length(min = 1, max = 255)"),
            ("DeletePayloadPoints.collection_name", "length(min = 1, max = 255)"),
            ("ClearPayloadPoints.collection_name", "length(min = 1, max = 255)"),
            ("CreateFieldIndexCollection.collection_name", "length(min = 1, max = 255)"),
            ("CreateFieldIndexCollection.field_name", "length(min = 1)"),
            ("DeleteFieldIndexCollection.collection_name", "length(min = 1, max = 255)"),
            ("DeleteFieldIndexCollection.field_name", "length(min = 1)"),
            ("SearchPoints.collection_name", "length(min = 1, max = 255)"),
            ("SearchPoints.limit", "range(min = 1)"),
            ("SearchPoints.vector_name", "custom = \"crate::grpc::validate::validate_not_empty\""),
            ("SearchBatchPoints.collection_name", "length(min = 1, max = 255)"),
            ("SearchBatchPoints.search_points", ""),
            ("ScrollPoints.collection_name", "length(min = 1, max = 255)"),
            ("ScrollPoints.limit", "custom = \"crate::grpc::validate::validate_u32_range_min_1\""),
            ("RecommendPoints.collection_name", "length(min = 1, max = 255)"),
            ("RecommendBatchPoints.collection_name", "length(min = 1, max = 255)"),
            ("RecommendBatchPoints.recommend_points", ""),
            ("CountPoints.collection_name", "length(min = 1, max = 255)"),
        ], &[])
        // Service: points_internal_service.proto
        .validates(&[
            ("UpsertPointsInternal.upsert_points", ""),
            ("DeletePointsInternal.delete_points", ""),
            ("SetPayloadPointsInternal.set_payload_points", ""),
            ("DeletePayloadPointsInternal.delete_payload_points", ""),
            ("ClearPayloadPointsInternal.clear_payload_points", ""),
            ("CreateFieldIndexCollectionInternal.create_field_index_collection", ""),
            ("DeleteFieldIndexCollectionInternal.delete_field_index_collection", ""),
            ("SearchPointsInternal.search_points", ""),
            ("SearchBatchPointsInternal.collection_name", "length(min = 1, max = 255)"),
            ("SearchBatchPointsInternal.search_points", ""),
            ("RecommendPointsInternal.recommend_points", ""),
            ("ScrollPointsInternal.scroll_points", ""),
            ("GetPointsInternal.get_points", ""),
            ("CountPointsInternal.count_points", ""),
            ("SyncPointsInternal.sync_points", ""),
            ("SyncPoints.collection_name", "length(min = 1, max = 255)"),
        ], &[])
        // Service: raft_service.proto
        .validates(&[
            ("AddPeerToKnownMessage.uri", "custom = \"crate::grpc::validate::validate_not_empty\""),
            ("AddPeerToKnownMessage.port", "custom = \"crate::grpc::validate::validate_u32_range_min_1\""),
        ], &[])
        // Service: snapshot_service.proto
        .validates(&[
            ("CreateSnapshotRequest.collection_name", "length(min = 1, max = 255)"),
            ("ListSnapshotsRequest.collection_name", "length(min = 1, max = 255)"),
            ("DeleteSnapshotRequest.collection_name", "length(min = 1, max = 255)"),
            ("DeleteSnapshotRequest.snapshot_name", "length(min = 1)"),
            ("DeleteFullSnapshotRequest.snapshot_name", "length(min = 1)"),
        ], &[
            "CreateFullSnapshotRequest",
            "ListFullSnapshotsRequest",
        ])
}

fn append_to_file(path: &str, line: &str) {
    use std::fs::OpenOptions;
    use std::io::prelude::*;
    writeln!(
        OpenOptions::new()
            .write(true)
            .append(true)
            .open(path)
            .unwrap(),
        "{line}",
    )
    .unwrap()
}
