use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use atomic_refcell::AtomicRefCell;
use parking_lot::{Mutex, RwLock};
use rocksdb::DB;
use tar::Builder;
use uuid::Uuid;

use crate::common::file_operations::{atomic_save_json, read_json};
use crate::common::version::{StorageVersion, VERSION_FILE};
use crate::common::{check_vector_name, check_vectors_set};
use crate::data_types::named_vectors::NamedVectors;
use crate::data_types::vectors::VectorElementType;
use crate::entry::entry_point::OperationError::TypeInferenceError;
use crate::entry::entry_point::{
    get_service_error, OperationError, OperationResult, SegmentEntry, SegmentFailedState,
};
use crate::id_tracker::IdTrackerSS;
use crate::index::field_index::CardinalityEstimation;
use crate::index::struct_payload_index::StructPayloadIndex;
use crate::index::{PayloadIndex, VectorIndex, VectorIndexEnum};
use crate::spaces::tools::peek_top_smallest_iterable;
use crate::telemetry::SegmentTelemetry;
use crate::types::{
    Filter, Payload, PayloadFieldSchema, PayloadIndexInfo, PayloadKeyType, PayloadKeyTypeRef,
    PayloadSchemaType, PointIdType, PointOffsetType, ScoredPoint, SearchParams, SegmentConfig,
    SegmentInfo, SegmentState, SegmentType, SeqNumberType, WithPayload, WithVector,
};
use crate::utils;
use crate::vector_storage::{ScoredPointOffset, VectorStorage, VectorStorageEnum};

pub const SEGMENT_STATE_FILE: &str = "segment.json";

const SNAPSHOT_PATH: &str = "snapshot";

// Sub-directories of `SNAPSHOT_PATH`:
const DB_BACKUP_PATH: &str = "db_backup";
const PAYLOAD_DB_BACKUP_PATH: &str = "payload_index_db_backup";
const SNAPSHOT_FILES_PATH: &str = "files";

pub struct SegmentVersion;

impl StorageVersion for SegmentVersion {
    fn current() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

/// Segment - an object which manages an independent group of points.
///
/// - Provides storage, indexing and managing operations for points (vectors + payload)
/// - Keeps track of point versions
/// - Persists data
/// - Keeps track of occurred errors
pub struct Segment {
    /// Latest update operation number, applied to this segment
    /// If None, there were no updates and segment is empty
    pub version: Option<SeqNumberType>,
    /// Latest persisted version
    pub persisted_version: Arc<Mutex<Option<SeqNumberType>>>,
    /// Path of the storage root
    pub current_path: PathBuf,
    /// Component for mapping external ids to internal and also keeping track of point versions
    pub id_tracker: Arc<AtomicRefCell<IdTrackerSS>>,
    pub vector_data: HashMap<String, VectorData>,
    pub payload_index: Arc<AtomicRefCell<StructPayloadIndex>>,
    /// Shows if it is possible to insert more points into this segment
    pub appendable_flag: bool,
    /// Shows what kind of indexes and storages are used in this segment
    pub segment_type: SegmentType,
    pub segment_config: SegmentConfig,
    /// Last unhandled error
    /// If not None, all update operations will be aborted until original operation is performed properly
    pub error_status: Option<SegmentFailedState>,
    pub database: Arc<RwLock<DB>>,
    pub flush_thread: Mutex<Option<JoinHandle<OperationResult<SeqNumberType>>>>,
}

pub struct VectorData {
    pub vector_index: Arc<AtomicRefCell<VectorIndexEnum>>,
    pub vector_storage: Arc<AtomicRefCell<VectorStorageEnum>>,
}

impl Segment {
    /// Change vector in-place.
    /// WARN: Available for appendable segments only
    fn update_vector(
        &mut self,
        internal_id: PointOffsetType,
        vectors: NamedVectors,
    ) -> OperationResult<()> {
        debug_assert!(self.is_appendable());
        check_vectors_set(&vectors, &self.segment_config)?;
        for (vector_name, vector) in vectors {
            let vector_name: &str = &vector_name;
            let vector_data = &self.vector_data[vector_name];
            let mut vector_storage = vector_data.vector_storage.borrow_mut();
            vector_storage.insert_vector(internal_id, &vector)?;
        }
        Ok(())
    }

    /// Operation wrapped, which handles previous and new errors in the segment,
    /// automatically updates versions and skips operations if version is too old
    ///
    /// # Arguments
    ///
    /// * `op_num` - sequential operation of the current operation
    /// * `op_point_offset` - if operation is point-related, specify this point offset.
    ///     If point offset is specified, handler will use point version for comparision.
    ///     Otherwise, it will use global storage version
    /// * `op` - operation to be wrapped. Should return `OperationResult` of bool (which is returned outside)
    ///     and optionally new offset of the changed point.
    ///
    /// # Result
    ///
    /// Propagates `OperationResult` of bool (which is returned in the `op` closure)
    ///
    fn handle_version_and_failure<F>(
        &mut self,
        op_num: SeqNumberType,
        op_point_offset: Option<PointOffsetType>,
        operation: F,
    ) -> OperationResult<bool>
    where
        F: FnOnce(&mut Segment) -> OperationResult<(bool, Option<PointOffsetType>)>,
    {
        if let Some(SegmentFailedState {
            version: failed_version,
            point_id: _failed_point_id,
            error,
        }) = &self.error_status
        {
            // Failed operations should not be skipped,
            // fail if newer operation is attempted before proper recovery
            if *failed_version < op_num {
                return Err(OperationError::service_error(format!(
                    "Not recovered from previous error: {error}"
                )));
            } // else: Re-try operation
        }

        let res = self.handle_version(op_num, op_point_offset, operation);

        match get_service_error(&res) {
            None => {
                // Recover error state
                match &self.error_status {
                    None => {} // all good
                    Some(error) => {
                        let point_id = op_point_offset.and_then(|point_offset| {
                            self.id_tracker.borrow().external_id(point_offset)
                        });
                        if error.point_id == point_id {
                            // Fixed
                            log::info!("Recovered from error: {}", error.error);
                            self.error_status = None;
                        }
                    }
                }
            }
            Some(error) => {
                // ToDo: Recover previous segment state
                log::error!(
                    "Segment {:?} operation error: {}",
                    self.current_path.as_path(),
                    error
                );
                let point_id = op_point_offset
                    .and_then(|point_offset| self.id_tracker.borrow().external_id(point_offset));
                self.error_status = Some(SegmentFailedState {
                    version: op_num,
                    point_id,
                    error,
                });
            }
        }
        res
    }

    /// Manage segment version checking
    /// If current version if higher than operation version - do not perform the operation
    /// Update current version if operation successfully executed
    fn handle_version<F>(
        &mut self,
        op_num: SeqNumberType,
        op_point_offset: Option<PointOffsetType>,
        operation: F,
    ) -> OperationResult<bool>
    where
        F: FnOnce(&mut Segment) -> OperationResult<(bool, Option<PointOffsetType>)>,
    {
        match op_point_offset {
            None => {
                // Not a point operation, use global version to check if already applied
                if self.version.unwrap_or(0) > op_num {
                    return Ok(false); // Skip without execution
                }
            }
            Some(point_offset) => {
                // Check if point not exists or have lower version
                if self
                    .id_tracker
                    .borrow()
                    .internal_version(point_offset)
                    .map_or(false, |current_version| current_version > op_num)
                {
                    return Ok(false);
                }
            }
        }

        let res = operation(self);

        if res.is_ok() {
            self.version = Some(max(op_num, self.version.unwrap_or(0)));
            if let Ok((_, Some(point_id))) = res {
                self.id_tracker
                    .borrow_mut()
                    .set_internal_version(point_id, op_num)?;
            }
        }
        res.map(|(res, _)| res)
    }

    fn lookup_internal_id(&self, point_id: PointIdType) -> OperationResult<PointOffsetType> {
        let internal_id_opt = self.id_tracker.borrow().internal_id(point_id);
        match internal_id_opt {
            Some(internal_id) => Ok(internal_id),
            None => Err(OperationError::PointIdError {
                missed_point_id: point_id,
            }),
        }
    }

    fn get_state(&self) -> SegmentState {
        SegmentState {
            version: self.version,
            config: self.segment_config.clone(),
        }
    }

    pub fn save_state(state: &SegmentState, current_path: &Path) -> OperationResult<()> {
        let state_path = current_path.join(SEGMENT_STATE_FILE);
        Ok(atomic_save_json(&state_path, state)?)
    }

    pub fn load_state(current_path: &Path) -> OperationResult<SegmentState> {
        let state_path = current_path.join(SEGMENT_STATE_FILE);
        Ok(read_json(&state_path)?)
    }

    /// Retrieve vector by internal ID
    ///
    /// Returns None if the vector does not exists or deleted
    #[inline]
    fn vector_by_offset(
        &self,
        vector_name: &str,
        point_offset: PointOffsetType,
    ) -> OperationResult<Option<Vec<VectorElementType>>> {
        check_vector_name(vector_name, &self.segment_config)?;
        let vector_data = &self.vector_data[vector_name];
        if !self.id_tracker.borrow().is_deleted(point_offset) {
            Ok(Some(
                vector_data
                    .vector_storage
                    .borrow()
                    .get_vector(point_offset)
                    .to_vec(),
            ))
        } else {
            Ok(None)
        }
    }

    fn all_vectors_by_offset(
        &self,
        point_offset: PointOffsetType,
    ) -> OperationResult<NamedVectors> {
        let mut vectors = NamedVectors::default();
        for (vector_name, vector_data) in &self.vector_data {
            vectors.insert(
                vector_name.clone(),
                vector_data
                    .vector_storage
                    .borrow()
                    .get_vector(point_offset)
                    .to_vec(),
            );
        }
        Ok(vectors)
    }

    /// Retrieve payload by internal ID
    #[inline]
    fn payload_by_offset(&self, point_offset: PointOffsetType) -> OperationResult<Payload> {
        self.payload_index.borrow().payload(point_offset)
    }

    pub fn save_current_state(&self) -> OperationResult<()> {
        Self::save_state(&self.get_state(), &self.current_path)
    }

    fn infer_from_payload_data(
        &self,
        key: PayloadKeyTypeRef,
    ) -> OperationResult<Option<PayloadSchemaType>> {
        let payload_index = self.payload_index.borrow();
        payload_index.infer_payload_type(key)
    }

    pub fn restore_snapshot(snapshot_path: &Path, segment_id: &str) -> OperationResult<()> {
        let segment_path = snapshot_path.parent().unwrap().join(segment_id);

        let archive_file = File::open(snapshot_path).map_err(|err| {
            OperationError::service_error(format!(
                "failed to open segment snapshot archive {snapshot_path:?}: {err}"
            ))
        })?;

        tar::Archive::new(archive_file)
            .unpack(&segment_path)
            .map_err(|err| {
                OperationError::service_error(format!(
                    "failed to unpack segment snapshot archive {snapshot_path:?}: {err}"
                ))
            })?;

        let snapshot_path = segment_path.join(SNAPSHOT_PATH);

        if snapshot_path.exists() {
            let db_backup_path = snapshot_path.join(DB_BACKUP_PATH);
            let payload_index_db_backup = snapshot_path.join(PAYLOAD_DB_BACKUP_PATH);

            crate::rocksdb_backup::restore(&db_backup_path, &segment_path)?;

            if payload_index_db_backup.is_dir() {
                StructPayloadIndex::restore_database_snapshot(
                    &payload_index_db_backup,
                    &segment_path,
                )?;
            }

            let files_path = snapshot_path.join(SNAPSHOT_FILES_PATH);
            utils::fs::move_all(&files_path, &segment_path)?;

            fs::remove_dir_all(&snapshot_path).map_err(|err| {
                OperationError::service_error(format!(
                    "failed to remove {snapshot_path:?} directory: {err}"
                ))
            })?;
        } else {
            log::info!("Attempt to restore legacy snapshot format");
            // Do nothing, legacy format is just plain archive
        }

        Ok(())
    }

    // Joins flush thread if exists
    // Returns lock to guarantee that there will be no other flush in a different thread
    fn lock_flushing(
        &self,
    ) -> OperationResult<parking_lot::MutexGuard<Option<JoinHandle<OperationResult<SeqNumberType>>>>>
    {
        let mut lock = self.flush_thread.lock();
        let mut join_handle: Option<JoinHandle<OperationResult<SeqNumberType>>> = None;
        std::mem::swap(&mut join_handle, &mut lock);
        if let Some(join_handle) = join_handle {
            // Flush result was reported to segment, so we don't need this value anymore
            let _background_flush_result = join_handle
                .join()
                .map_err(|_err| OperationError::service_error("failed to join flush thread"))??;
        }
        Ok(lock)
    }

    fn is_background_flushing(&self) -> bool {
        let lock = self.flush_thread.lock();
        if let Some(join_handle) = lock.as_ref() {
            !join_handle.is_finished()
        } else {
            false
        }
    }

    /// Converts raw ScoredPointOffset search result into ScoredPoint result
    fn process_search_result(
        &self,
        internal_result: &[ScoredPointOffset],
        with_payload: &WithPayload,
        with_vector: &WithVector,
    ) -> OperationResult<Vec<ScoredPoint>> {
        let id_tracker = self.id_tracker.borrow();
        internal_result
            .iter()
            .filter_map(|&scored_point_offset| {
                let point_offset = scored_point_offset.idx;
                let external_id = id_tracker.external_id(point_offset);
                match external_id {
                    Some(point_id) => Some((point_id, scored_point_offset)),
                    None => {
                        log::warn!(
                            "Point with internal ID {} not found in id tracker, skipping",
                            point_offset
                        );
                        None
                    }
                }
            })
            .map(|(point_id, scored_point_offset)| {
                let point_offset = scored_point_offset.idx;
                let point_version = id_tracker.internal_version(point_offset).ok_or_else(|| {
                    OperationError::service_error(format!(
                        "Corrupter id_tracker, no version for point {point_id}"
                    ))
                })?;
                let payload = if with_payload.enable {
                    let initial_payload = self.payload_by_offset(point_offset)?;
                    let processed_payload = if let Some(i) = &with_payload.payload_selector {
                        i.process(initial_payload)
                    } else {
                        initial_payload
                    };
                    Some(processed_payload)
                } else {
                    None
                };
                let vector = match with_vector {
                    WithVector::Bool(false) => None,
                    WithVector::Bool(true) => {
                        Some(self.all_vectors_by_offset(point_offset)?.into())
                    }
                    WithVector::Selector(vectors) => {
                        let mut result = NamedVectors::default();
                        for vector_name in vectors {
                            let vector_opt = self.vector_by_offset(vector_name, point_offset)?;
                            match vector_opt {
                                None => {
                                    return Err(OperationError::service_error(
                                        "Vector {vector_name} not found at offset {point_offset}",
                                    ))
                                }
                                Some(vector) => result.insert(vector_name.clone(), vector),
                            }
                        }
                        Some(result.into())
                    }
                };

                Ok(ScoredPoint {
                    id: point_id,
                    version: point_version,
                    score: scored_point_offset.score,
                    payload,
                    vector,
                })
            })
            .collect()
    }

    pub fn filtered_read_by_index(
        &self,
        offset: Option<PointIdType>,
        limit: Option<usize>,
        condition: &Filter,
    ) -> Vec<PointIdType> {
        let payload_index = self.payload_index.borrow();
        let id_tracker = self.id_tracker.borrow();

        let ids_iterator = payload_index
            .query_points(condition)
            .filter_map(|internal_id| {
                let external_id = id_tracker.external_id(internal_id);
                match external_id {
                    Some(external_id) => match offset {
                        Some(offset) if external_id < offset => None,
                        _ => Some(external_id),
                    },
                    None => None,
                }
            });

        let mut page = match limit {
            Some(limit) => peek_top_smallest_iterable(ids_iterator, limit),
            None => ids_iterator.collect(),
        };
        page.sort_unstable();
        page
    }

    pub fn filtered_read_by_id_stream(
        &self,
        offset: Option<PointIdType>,
        limit: Option<usize>,
        condition: &Filter,
    ) -> Vec<PointIdType> {
        let payload_index = self.payload_index.borrow();
        let filter_context = payload_index.filter_context(condition);
        self.id_tracker
            .borrow()
            .iter_from(offset)
            .filter(move |(_, internal_id)| filter_context.check(*internal_id))
            .map(|(external_id, _)| external_id)
            .take(limit.unwrap_or(usize::MAX))
            .collect()
    }

    /// Check consistency of the segment's data and repair it if possible.
    pub fn check_consistency_and_repair(&mut self) -> OperationResult<()> {
        let mut internal_ids_to_delete = HashSet::new();
        let id_tracker = self.id_tracker.borrow();
        for internal_id in id_tracker.iter_ids() {
            if id_tracker.external_id(internal_id).is_none() {
                internal_ids_to_delete.insert(internal_id);
            }
        }

        if !internal_ids_to_delete.is_empty() {
            log::info!(
                "Found {} points in vector storage without external id - those will be deleted",
                internal_ids_to_delete.len(),
            );
            for internal_id in &internal_ids_to_delete {
                self.payload_index.borrow_mut().drop(*internal_id)?;
            }

            // We do not drop version here, because it is already not loaded into memory.
            // There are no explicit mapping between internal ID and version, so all dangling
            // versions will be ignored automatically.
            // Those versions could be overwritten by new points, but it is not a problem.
            // They will also be deleted by the next optimization.
        }

        // flush entire segment if needed
        if !internal_ids_to_delete.is_empty() {
            self.flush(true)?;
        }
        Ok(())
    }
}

/// This is a basic implementation of `SegmentEntry`,
/// meaning that it implements the _actual_ operations with data and not any kind of proxy or wrapping
impl SegmentEntry for Segment {
    fn version(&self) -> SeqNumberType {
        self.version.unwrap_or(0)
    }

    fn point_version(&self, point_id: PointIdType) -> Option<SeqNumberType> {
        let id_tracker = self.id_tracker.borrow();
        id_tracker
            .internal_id(point_id)
            .and_then(|internal_id| id_tracker.internal_version(internal_id))
    }

    fn search(
        &self,
        vector_name: &str,
        vector: &[VectorElementType],
        with_payload: &WithPayload,
        with_vector: &WithVector,
        filter: Option<&Filter>,
        top: usize,
        params: Option<&SearchParams>,
    ) -> OperationResult<Vec<ScoredPoint>> {
        check_vector_name(vector_name, &self.segment_config)?;
        let vector_data = &self.vector_data[vector_name];
        let expected_vector_dim = vector_data.vector_storage.borrow().vector_dim();
        if vector.len() != expected_vector_dim {
            return Err(OperationError::WrongVector {
                expected_dim: expected_vector_dim,
                received_dim: vector.len(),
            });
        }

        let internal_result =
            &vector_data
                .vector_index
                .borrow()
                .search(&[vector], filter, top, params)[0];

        self.process_search_result(internal_result, with_payload, with_vector)
    }

    fn search_batch(
        &self,
        vector_name: &str,
        vectors: &[&[VectorElementType]],
        with_payload: &WithPayload,
        with_vector: &WithVector,
        filter: Option<&Filter>,
        top: usize,
        params: Option<&SearchParams>,
    ) -> OperationResult<Vec<Vec<ScoredPoint>>> {
        check_vector_name(vector_name, &self.segment_config)?;
        let vector_data = &self.vector_data[vector_name];
        let expected_vector_dim = vector_data.vector_storage.borrow().vector_dim();
        for vector in vectors {
            if vector.len() != expected_vector_dim {
                return Err(OperationError::WrongVector {
                    expected_dim: expected_vector_dim,
                    received_dim: vector.len(),
                });
            }
        }

        let internal_results = vector_data
            .vector_index
            .borrow()
            .search(vectors, filter, top, params);

        let res = internal_results
            .iter()
            .map(|internal_result| {
                self.process_search_result(internal_result, with_payload, with_vector)
            })
            .collect();

        res
    }

    fn upsert_vector(
        &mut self,
        op_num: SeqNumberType,
        point_id: PointIdType,
        vectors: &NamedVectors,
    ) -> OperationResult<bool> {
        debug_assert!(self.is_appendable());
        check_vectors_set(vectors, &self.segment_config)?;
        let stored_internal_point = self.id_tracker.borrow().internal_id(point_id);
        self.handle_version_and_failure(op_num, stored_internal_point, |segment| {
            let mut processed_vectors = NamedVectors::default();
            for (vector_name, vector) in vectors.iter() {
                let vector_name: &str = vector_name;
                let vector: &[VectorElementType] = vector;
                let vector_data = &segment.vector_data[vector_name];
                let vector_dim = vector_data.vector_storage.borrow().vector_dim();
                if vector_dim != vector.len() {
                    return Err(OperationError::WrongVector {
                        expected_dim: vector_dim,
                        received_dim: vector.len(),
                    });
                }

                let processed_vector_opt = segment.segment_config.vector_data[vector_name]
                    .distance
                    .preprocess_vector(vector);
                match processed_vector_opt {
                    None => processed_vectors.insert_ref(vector_name, vector),
                    Some(preprocess_vector) => {
                        processed_vectors.insert(vector_name.to_string(), preprocess_vector)
                    }
                }
            }

            if let Some(existing_internal_id) = stored_internal_point {
                segment.update_vector(existing_internal_id, processed_vectors)?;
                Ok((true, Some(existing_internal_id)))
            } else {
                let new_index = segment.id_tracker.borrow().internal_size() as PointOffsetType;

                for (vector_name, processed_vector) in processed_vectors {
                    let vector_name: &str = &vector_name;
                    segment.vector_data[vector_name]
                        .vector_storage
                        .borrow_mut()
                        .insert_vector(new_index, &processed_vector)?;
                }
                segment
                    .id_tracker
                    .borrow_mut()
                    .set_link(point_id, new_index)?;
                Ok((false, Some(new_index)))
            }
        })
    }

    fn delete_point(
        &mut self,
        op_num: SeqNumberType,
        point_id: PointIdType,
    ) -> OperationResult<bool> {
        let internal_id = self.id_tracker.borrow().internal_id(point_id);
        match internal_id {
            None => Ok(false), // Point already not exists
            Some(internal_id) => {
                self.handle_version_and_failure(op_num, Some(internal_id), |segment| {
                    segment.payload_index.borrow_mut().drop(internal_id)?;
                    segment.id_tracker.borrow_mut().drop(point_id)?;
                    Ok((true, Some(internal_id)))
                })
            }
        }
    }

    fn set_full_payload(
        &mut self,
        op_num: SeqNumberType,
        point_id: PointIdType,
        full_payload: &Payload,
    ) -> OperationResult<bool> {
        let internal_id = self.id_tracker.borrow().internal_id(point_id);
        self.handle_version_and_failure(op_num, internal_id, |segment| match internal_id {
            Some(internal_id) => {
                segment
                    .payload_index
                    .borrow_mut()
                    .assign_all(internal_id, full_payload)?;
                Ok((true, Some(internal_id)))
            }
            None => Err(OperationError::PointIdError {
                missed_point_id: point_id,
            }),
        })
    }

    fn set_payload(
        &mut self,
        op_num: SeqNumberType,
        point_id: PointIdType,
        payload: &Payload,
    ) -> OperationResult<bool> {
        let internal_id = self.id_tracker.borrow().internal_id(point_id);
        self.handle_version_and_failure(op_num, internal_id, |segment| match internal_id {
            Some(internal_id) => {
                segment
                    .payload_index
                    .borrow_mut()
                    .assign(internal_id, payload)?;
                Ok((true, Some(internal_id)))
            }
            None => Err(OperationError::PointIdError {
                missed_point_id: point_id,
            }),
        })
    }

    fn delete_payload(
        &mut self,
        op_num: SeqNumberType,
        point_id: PointIdType,
        key: PayloadKeyTypeRef,
    ) -> OperationResult<bool> {
        let internal_id = self.id_tracker.borrow().internal_id(point_id);
        self.handle_version_and_failure(op_num, internal_id, |segment| match internal_id {
            Some(internal_id) => {
                segment
                    .payload_index
                    .borrow_mut()
                    .delete(internal_id, key)?;
                Ok((true, Some(internal_id)))
            }
            None => Err(OperationError::PointIdError {
                missed_point_id: point_id,
            }),
        })
    }

    fn clear_payload(
        &mut self,
        op_num: SeqNumberType,
        point_id: PointIdType,
    ) -> OperationResult<bool> {
        let internal_id = self.id_tracker.borrow().internal_id(point_id);
        self.handle_version_and_failure(op_num, internal_id, |segment| match internal_id {
            Some(internal_id) => {
                segment.payload_index.borrow_mut().drop(internal_id)?;
                Ok((true, Some(internal_id)))
            }
            None => Err(OperationError::PointIdError {
                missed_point_id: point_id,
            }),
        })
    }

    fn vector(
        &self,
        vector_name: &str,
        point_id: PointIdType,
    ) -> OperationResult<Vec<VectorElementType>> {
        let internal_id = self.lookup_internal_id(point_id)?;
        let vector_opt = self.vector_by_offset(vector_name, internal_id)?;
        if let Some(vector) = vector_opt {
            Ok(vector)
        } else {
            let segment_path = self.current_path.display();
            Err(OperationError::service_error(format!(
                "Vector {vector_name} not found at offset {internal_id} for point {point_id}, segment {segment_path}",
            )))
        }
    }

    fn all_vectors(&self, point_id: PointIdType) -> OperationResult<NamedVectors> {
        let mut result = NamedVectors::default();
        for vector_name in self.vector_data.keys() {
            result.insert(vector_name.clone(), self.vector(vector_name, point_id)?);
        }
        Ok(result)
    }

    fn payload(&self, point_id: PointIdType) -> OperationResult<Payload> {
        let internal_id = self.lookup_internal_id(point_id)?;
        self.payload_by_offset(internal_id)
    }

    fn iter_points(&self) -> Box<dyn Iterator<Item = PointIdType> + '_> {
        // Sorry for that, but I didn't find any way easier.
        // If you try simply return iterator - it won't work because AtomicRef should exist
        // If you try to make callback instead - you won't be able to create <dyn SegmentEntry>
        // Attempt to create return borrowed value along with iterator failed because of insane lifetimes
        unsafe { self.id_tracker.as_ptr().as_ref().unwrap().iter_external() }
    }

    fn read_filtered<'a>(
        &'a self,
        offset: Option<PointIdType>,
        limit: Option<usize>,
        filter: Option<&'a Filter>,
    ) -> Vec<PointIdType> {
        match filter {
            None => self
                .id_tracker
                .borrow()
                .iter_from(offset)
                .map(|x| x.0)
                .take(limit.unwrap_or(usize::MAX))
                .collect(),
            Some(condition) => {
                let query_cardinality = {
                    let payload_index = self.payload_index.borrow();
                    payload_index.estimate_cardinality(condition)
                };

                // ToDo: Add telemetry for this heuristics

                // Calculate expected number of condition checks required for
                // this scroll request with is stream strategy.
                // Example:
                //  - cardinality = 1000
                //  - limit = 10
                //  - total = 10000
                //  - point filter prob = 1000 / 10000 = 0.1
                //  - expected_checks = 10 / 0.1  = 100
                //  -------------------------------
                //  - cardinality = 10
                //  - limit = 10
                //  - total = 10000
                //  - point filter prob = 10 / 10000 = 0.001
                //  - expected_checks = 10 / 0.001  = 10000

                let total_points = self.points_count() + 1 /* + 1 for division-by-zero */;
                // Expected number of successful checks per point
                let check_probability = (query_cardinality.exp as f64 + 1.0/* protect from zero */)
                    / total_points as f64;
                let exp_stream_checks =
                    (limit.unwrap_or(total_points) as f64 / check_probability) as usize;

                // Assume it would require about `query cardinality` checks.
                // We are interested in approximate number of checks, so we can
                // use `query cardinality` as a starting point.
                let exp_index_checks = query_cardinality.max;

                if exp_stream_checks > exp_index_checks {
                    self.filtered_read_by_index(offset, limit, condition)
                } else {
                    self.filtered_read_by_id_stream(offset, limit, condition)
                }
            }
        }
    }

    fn read_range(&self, from: Option<PointIdType>, to: Option<PointIdType>) -> Vec<PointIdType> {
        let id_tracker = self.id_tracker.borrow();
        let iterator = id_tracker.iter_from(from).map(|x| x.0);
        match to {
            None => iterator.collect(),
            Some(to_id) => iterator.take_while(|x| *x < to_id).collect(),
        }
    }

    fn has_point(&self, point_id: PointIdType) -> bool {
        self.id_tracker.borrow().internal_id(point_id).is_some()
    }

    fn points_count(&self) -> usize {
        self.id_tracker.borrow().points_count()
    }

    fn estimate_points_count<'a>(&'a self, filter: Option<&'a Filter>) -> CardinalityEstimation {
        match filter {
            None => {
                let total_count = self.points_count();
                CardinalityEstimation {
                    primary_clauses: vec![],
                    min: total_count,
                    exp: total_count,
                    max: total_count,
                }
            }
            Some(filter) => {
                let payload_index = self.payload_index.borrow();
                payload_index.estimate_cardinality(filter)
            }
        }
    }

    fn deleted_count(&self) -> usize {
        self.id_tracker.borrow().deleted_count()
    }

    fn segment_type(&self) -> SegmentType {
        self.segment_type
    }

    fn info(&self) -> SegmentInfo {
        let payload_index = self.payload_index.borrow();
        let schema = payload_index
            .indexed_fields()
            .into_iter()
            .map(|(key, index_schema)| {
                let points_count = payload_index.indexed_points(&key);
                (key, PayloadIndexInfo::new(index_schema, points_count))
            })
            .collect();

        SegmentInfo {
            segment_type: self.segment_type,
            num_vectors: self.points_count() * self.vector_data.len(),
            num_points: self.points_count(),
            num_deleted_vectors: self.deleted_count(),
            ram_usage_bytes: 0,  // ToDo: Implement
            disk_usage_bytes: 0, // ToDo: Implement
            is_appendable: self.appendable_flag,
            index_schema: schema,
        }
    }

    fn config(&self) -> SegmentConfig {
        self.segment_config.clone()
    }

    fn is_appendable(&self) -> bool {
        self.appendable_flag
    }

    fn flush(&self, sync: bool) -> OperationResult<SeqNumberType> {
        let current_persisted_version: Option<SeqNumberType> = *self.persisted_version.lock();
        if !sync && self.is_background_flushing() {
            return Ok(current_persisted_version.unwrap_or(0));
        }

        let mut background_flush_lock = self.lock_flushing()?;
        match (self.version, current_persisted_version) {
            (None, _) => {
                // Segment is empty, nothing to flush
                return Ok(current_persisted_version.unwrap_or(0));
            }
            (Some(version), Some(persisted_version)) => {
                if version == persisted_version {
                    // Segment is already flushed
                    return Ok(persisted_version);
                }
            }
            (_, _) => {}
        }

        let vector_storage_flushers: Vec<_> = self
            .vector_data
            .values()
            .map(|v| v.vector_storage.borrow().flusher())
            .collect();
        let state = self.get_state();
        let current_path = self.current_path.clone();
        let id_tracker_mapping_flusher = self.id_tracker.borrow().mapping_flusher();
        let payload_index_flusher = self.payload_index.borrow().flusher();
        let id_tracker_versions_flusher = self.id_tracker.borrow().versions_flusher();
        let persisted_version = self.persisted_version.clone();

        // Flush order is important:
        //
        // 1. Flush id mapping. So during recovery the point will be recovered er in proper segment.
        // 2. Flush vectors and payloads.
        // 3. Flush id versions last. So presence of version indicates that all other data is up-to-date.
        //
        // Example of recovery from WAL in case of partial flush:
        //
        // In-memory state:
        //
        //     Segment 1                  Segment 2
        //
        //    ID-mapping     vst.1       ID-mapping     vst.2
        //   ext     int
        //  ┌───┐   ┌───┐   ┌───┐       ┌───┐   ┌───┐   ┌───┐
        //  │100├───┤1  │   │1  │       │300├───┤1  │   │1  │
        //  └───┘   └───┘   │2  │       └───┘   └───┘   │2  │
        //                  │   │                       │   │
        //  ┌───┐   ┌───┐   │   │       ┌───┐   ┌───┐   │   │
        //  │200├───┤2  │   │   │       │400├───┤2  │   │   │
        //  └───┘   └───┘   └───┘       └───┘   └───┘   └───┘
        //
        //
        //  ext - external id
        //  int - internal id
        //  vst - vector storage
        //
        //  ─────────────────────────────────────────────────
        //   After flush, segments could be partially preserved:
        //
        //  ┌───┐   ┌───┐   ┌───┐       ┌───┐   ┌───┐   ┌───┐
        //  │100├───┤1  │   │ 1 │       │300├───┤1  │   │ * │
        //  └───┘   └───┘   │   │       └───┘   └───┘   │ * │
        //                  │   │                       │ 3 │
        //                  │   │       ┌───┐   ┌───┐   │   │
        //                  │   │       │400├───┤2  │   │   │
        //                  └───┘       └───┘   └───┘   └───┘
        //  WAL:      ▲
        //            │                 ┌───┐   ┌───┐
        //  100───────┘      ┌────────► │200├───┤3  │
        //                   |          └───┘   └───┘
        //  200──────────────┘
        //
        //  300
        //
        //  400

        let flush_op = move || {
            // Flush mapping first to prevent having orphan internal ids.
            id_tracker_mapping_flusher().map_err(|err| {
                OperationError::service_error(format!("Failed to flush id_tracker mapping: {err}"))
            })?;
            for vector_storage_flusher in vector_storage_flushers {
                vector_storage_flusher().map_err(|err| {
                    OperationError::service_error(format!("Failed to flush vector_storage: {err}"))
                })?;
            }
            payload_index_flusher().map_err(|err| {
                OperationError::service_error(format!("Failed to flush payload_index: {err}"))
            })?;
            // Id Tracker contains versions of points. We need to flush it after vector_storage and payload_index flush.
            // This is because vector_storage and payload_index flush are not atomic.
            // If payload or vector flush fails, we will be able to recover data from WAL.
            // If Id Tracker flush fails, we are also able to recover data from WAL
            //  by simply overriding data in vector and payload storages.
            // Once versions are saved - points are considered persisted.
            id_tracker_versions_flusher().map_err(|err| {
                OperationError::service_error(format!("Failed to flush id_tracker versions: {err}"))
            })?;
            Self::save_state(&state, &current_path).map_err(|err| {
                OperationError::service_error(format!("Failed to flush segment state: {err}"))
            })?;
            *persisted_version.lock() = state.version;

            debug_assert!(state.version.is_some());
            Ok(state.version.unwrap_or(0))
        };

        if sync {
            flush_op()
        } else {
            *background_flush_lock = Some(
                std::thread::Builder::new()
                    .name("background_flush".to_string())
                    .spawn(flush_op)
                    .unwrap(),
            );
            Ok(current_persisted_version.unwrap_or(0))
        }
    }

    fn drop_data(self) -> OperationResult<()> {
        let current_path = self.current_path.clone();
        drop(self);
        let mut deleted_path = current_path.clone();
        deleted_path.set_extension("deleted");
        fs::rename(&current_path, &deleted_path)?;
        fs::remove_dir_all(&deleted_path).map_err(|err| {
            OperationError::service_error(format!(
                "Can't remove segment data at {}, error: {}",
                deleted_path.to_str().unwrap_or_default(),
                err
            ))
        })
    }

    fn data_path(&self) -> PathBuf {
        self.current_path.clone()
    }

    fn delete_field_index(&mut self, op_num: u64, key: PayloadKeyTypeRef) -> OperationResult<bool> {
        self.handle_version_and_failure(op_num, None, |segment| {
            segment.payload_index.borrow_mut().drop_index(key)?;
            Ok((true, None))
        })
    }

    fn create_field_index(
        &mut self,
        op_num: u64,
        key: PayloadKeyTypeRef,
        field_type: Option<&PayloadFieldSchema>,
    ) -> OperationResult<bool> {
        self.handle_version_and_failure(op_num, None, |segment| match field_type {
            Some(schema) => {
                segment
                    .payload_index
                    .borrow_mut()
                    .set_indexed(key, schema.clone())?;
                Ok((true, None))
            }
            None => match segment.infer_from_payload_data(key)? {
                None => Err(TypeInferenceError {
                    field_name: key.to_string(),
                }),
                Some(schema_type) => {
                    segment
                        .payload_index
                        .borrow_mut()
                        .set_indexed(key, schema_type.into())?;
                    Ok((true, None))
                }
            },
        })
    }

    fn get_indexed_fields(&self) -> HashMap<PayloadKeyType, PayloadFieldSchema> {
        self.payload_index.borrow().indexed_fields()
    }

    fn check_error(&self) -> Option<SegmentFailedState> {
        self.error_status.clone()
    }

    fn delete_filtered<'a>(
        &'a mut self,
        op_num: SeqNumberType,
        filter: &'a Filter,
    ) -> OperationResult<usize> {
        let mut deleted_points = 0;
        for point_id in self.read_filtered(None, None, Some(filter)) {
            deleted_points += self.delete_point(op_num, point_id)? as usize;
        }

        Ok(deleted_points)
    }

    fn vector_dim(&self, vector_name: &str) -> OperationResult<usize> {
        check_vector_name(vector_name, &self.segment_config)?;
        let vector_data_config = &self.segment_config.vector_data[vector_name];
        Ok(vector_data_config.size)
    }

    fn vector_dims(&self) -> HashMap<String, usize> {
        self.segment_config
            .vector_data
            .iter()
            .map(|(vector_name, vector_config)| (vector_name.clone(), vector_config.size))
            .collect()
    }

    fn take_snapshot(&self, snapshot_dir_path: &Path) -> OperationResult<PathBuf> {
        log::debug!(
            "Taking snapshot of segment {:?} into {:?}",
            self.current_path,
            snapshot_dir_path
        );

        if !snapshot_dir_path.exists() {
            return Err(OperationError::service_error(format!(
                "the snapshot path {snapshot_dir_path:?} does not exist"
            )));
        }

        if !snapshot_dir_path.is_dir() {
            return Err(OperationError::service_error(format!(
                "the snapshot path {snapshot_dir_path:?} is not a directory",
            )));
        }

        // flush segment to capture latest state
        self.flush(true)?;

        let tmp_path = self.current_path.join(format!("tmp-{}", Uuid::new_v4()));

        let db_backup_path = tmp_path.join(DB_BACKUP_PATH);
        let payload_index_db_backup_path = tmp_path.join(PAYLOAD_DB_BACKUP_PATH);

        {
            let db = self.database.read();
            crate::rocksdb_backup::create(&db, &db_backup_path)?;
        }

        self.payload_index
            .borrow()
            .take_database_snapshot(&payload_index_db_backup_path)?;

        let segment_id = self
            .current_path
            .file_stem()
            .and_then(|f| f.to_str())
            .unwrap();

        let archive_path = snapshot_dir_path.join(format!("{segment_id}.tar"));

        // If `archive_path` exists, we still want to overwrite it
        let file = File::create(&archive_path).map_err(|err| {
            OperationError::service_error(format!(
                "failed to create segment snapshot archive {archive_path:?}: {err}"
            ))
        })?;

        let mut builder = Builder::new(file);

        builder
            .append_dir_all(SNAPSHOT_PATH, &tmp_path)
            .map_err(|err| utils::tar::failed_to_append_error(&tmp_path, err))?;

        let files = Path::new(SNAPSHOT_PATH).join(SNAPSHOT_FILES_PATH);

        for vector_data in self.vector_data.values() {
            for file in vector_data.vector_index.borrow().files() {
                utils::tar::append_file_relative_to_base(
                    &mut builder,
                    &self.current_path,
                    &file,
                    &files,
                )?;
            }

            for file in vector_data.vector_storage.borrow().files() {
                utils::tar::append_file_relative_to_base(
                    &mut builder,
                    &self.current_path,
                    &file,
                    &files,
                )?;
            }
        }

        for file in self.payload_index.borrow().files() {
            utils::tar::append_file_relative_to_base(
                &mut builder,
                &self.current_path,
                &file,
                &files,
            )?;
        }

        utils::tar::append_file(
            &mut builder,
            &self.current_path.join(SEGMENT_STATE_FILE),
            &files.join(SEGMENT_STATE_FILE),
        )?;

        utils::tar::append_file(
            &mut builder,
            &self.current_path.join(VERSION_FILE),
            &files.join(VERSION_FILE),
        )?;

        builder.finish()?;

        // remove tmp directory in background
        let _ = std::thread::spawn(move || {
            let res = std::fs::remove_dir_all(&tmp_path);
            if let Err(err) = res {
                log::error!(
                    "Failed to remove tmp directory at {}: {:?}",
                    tmp_path.display(),
                    err
                );
            }
        });

        Ok(archive_path)
    }

    fn get_telemetry_data(&self) -> SegmentTelemetry {
        let vector_index_searches: Vec<_> = self
            .vector_data
            .iter()
            .map(|(k, v)| {
                let mut telemetry = v.vector_index.borrow().get_telemetry_data();
                telemetry.index_name = Some(k.clone());
                telemetry
            })
            .collect();

        SegmentTelemetry {
            info: self.info(),
            config: self.config(),
            vector_index_searches,
            payload_field_indices: self.payload_index.borrow().get_telemetry_data(),
        }
    }
}

impl Drop for Segment {
    fn drop(&mut self) {
        let _lock = self.lock_flushing();
    }
}

#[cfg(test)]
mod tests {
    use tempfile::Builder;

    use super::*;
    use crate::data_types::vectors::{only_default_vector, DEFAULT_VECTOR_NAME};
    use crate::entry::entry_point::OperationError::PointIdError;
    use crate::segment_constructor::{build_segment, load_segment};
    use crate::types::{Distance, Indexes, SegmentConfig, StorageType, VectorDataConfig};

    // no longer valid since users are now allowed to store arbitrary json objects.
    // TODO(gvelo): add tests for invalid payload types on indexed fields.
    // #[test]
    // fn test_set_invalid_payload_from_json() {
    //     let data1 = r#"
    //     {
    //         "invalid_data"
    //     }"#;
    //     let data2 = r#"
    //     {
    //         "array": [1, "hello"],
    //     }"#;
    //
    //     let dir = Builder::new().prefix("payload_dir").tempdir().unwrap();
    //     let dim = 2;
    //     let config = SegmentConfig {
    //         vector_size: dim,
    //         index: Indexes::Plain {},
    //         payload_index: Some(PayloadIndexType::Plain),
    //         storage_type: StorageType::InMemory,
    //         distance: Distance::Dot,
    //     };
    //
    //     let mut segment =
    //         build_segment(dir.path(), &config, Arc::new(SchemaStorage::new())).unwrap();
    //     segment.upsert_point(0, 0.into(), &[1.0, 1.0]).unwrap();
    //
    //     let result1 = segment.set_full_payload_with_json(0, 0.into(), &data1.to_string());
    //     assert!(result1.is_err());
    //
    //     let result2 = segment.set_full_payload_with_json(0, 0.into(), &data2.to_string());
    //     assert!(result2.is_err());
    // }

    #[test]
    fn test_search_batch_equivalence_single() {
        let dir = Builder::new().prefix("segment_dir").tempdir().unwrap();
        let dim = 4;
        let config = SegmentConfig {
            vector_data: HashMap::from([(
                DEFAULT_VECTOR_NAME.to_owned(),
                VectorDataConfig {
                    size: dim,
                    distance: Distance::Dot,
                    hnsw_config: None,
                    quantization_config: None,
                },
            )]),
            index: Indexes::Plain {},
            storage_type: StorageType::InMemory,
            ..Default::default()
        };
        let mut segment = build_segment(dir.path(), &config).unwrap();

        let vec4 = vec![1.1, 1.0, 0.0, 1.0];
        segment
            .upsert_vector(100, 4.into(), &only_default_vector(&vec4))
            .unwrap();
        let vec6 = vec![1.0, 1.0, 0.5, 1.0];
        segment
            .upsert_vector(101, 6.into(), &only_default_vector(&vec6))
            .unwrap();
        segment.delete_point(102, 1.into()).unwrap();

        let query_vector = vec![1.0, 1.0, 1.0, 1.0];
        let search_result = segment
            .search(
                DEFAULT_VECTOR_NAME,
                &query_vector,
                &WithPayload::default(),
                &false.into(),
                None,
                10,
                None,
            )
            .unwrap();
        eprintln!("search_result = {search_result:#?}");

        let search_batch_result = segment
            .search_batch(
                DEFAULT_VECTOR_NAME,
                &[&query_vector],
                &WithPayload::default(),
                &false.into(),
                None,
                10,
                None,
            )
            .unwrap();
        eprintln!("search_batch_result = {search_batch_result:#?}");

        assert!(!search_result.is_empty());
        assert_eq!(search_result, search_batch_result[0].clone())
    }

    #[test]
    fn test_from_filter_attributes() {
        let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "metadata": {
                "height": 50,
                "width": 60
            }
        }"#;

        let dir = Builder::new().prefix("payload_dir").tempdir().unwrap();
        let dim = 2;
        let config = SegmentConfig {
            vector_data: HashMap::from([(
                DEFAULT_VECTOR_NAME.to_owned(),
                VectorDataConfig {
                    size: dim,
                    distance: Distance::Dot,
                    hnsw_config: None,
                    quantization_config: None,
                },
            )]),
            index: Indexes::Plain {},
            storage_type: StorageType::InMemory,
            ..Default::default()
        };

        let mut segment = build_segment(dir.path(), &config).unwrap();
        segment
            .upsert_vector(0, 0.into(), &only_default_vector(&[1.0, 1.0]))
            .unwrap();

        let payload: Payload = serde_json::from_str(data).unwrap();

        segment.set_full_payload(0, 0.into(), &payload).unwrap();

        let filter_valid_str = r#"
        {
            "must": [
                {
                    "key": "metadata.height",
                    "match": {
                        "value": 50
                    }
                }
            ]
        }"#;

        let filter_valid: Filter = serde_json::from_str(filter_valid_str).unwrap();
        let filter_invalid_str = r#"
        {
            "must": [
                {
                    "key": "metadata.height",
                    "match": {
                        "value": 60
                    }
                }
            ]
        }"#;

        let filter_invalid: Filter = serde_json::from_str(filter_invalid_str).unwrap();
        let results_with_valid_filter = segment
            .search(
                DEFAULT_VECTOR_NAME,
                &[1.0, 1.0],
                &WithPayload::default(),
                &false.into(),
                Some(&filter_valid),
                1,
                None,
            )
            .unwrap();
        assert_eq!(results_with_valid_filter.len(), 1);
        assert_eq!(results_with_valid_filter.first().unwrap().id, 0.into());
        let results_with_invalid_filter = segment
            .search(
                DEFAULT_VECTOR_NAME,
                &[1.0, 1.0],
                &WithPayload::default(),
                &false.into(),
                Some(&filter_invalid),
                1,
                None,
            )
            .unwrap();
        assert!(results_with_invalid_filter.is_empty());
    }

    #[test]
    fn test_snapshot() {
        let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "metadata": {
                "height": 50,
                "width": 60
            }
        }"#;

        let segment_base_dir = Builder::new().prefix("segment_dir").tempdir().unwrap();
        let config = SegmentConfig {
            vector_data: HashMap::from([(
                DEFAULT_VECTOR_NAME.to_owned(),
                VectorDataConfig {
                    size: 2,
                    distance: Distance::Dot,
                    hnsw_config: None,
                    quantization_config: None,
                },
            )]),
            index: Indexes::Plain {},
            storage_type: StorageType::InMemory,
            ..Default::default()
        };

        let mut segment = build_segment(segment_base_dir.path(), &config).unwrap();

        segment
            .upsert_vector(0, 0.into(), &only_default_vector(&[1.0, 1.0]))
            .unwrap();

        segment
            .set_full_payload(1, 0.into(), &serde_json::from_str(data).unwrap())
            .unwrap();

        let snapshot_dir = Builder::new().prefix("snapshot_dir").tempdir().unwrap();

        // snapshotting!
        let archive = segment.take_snapshot(snapshot_dir.path()).unwrap();
        let archive_extension = archive.extension().unwrap();
        let archive_name = archive.file_name().unwrap().to_str().unwrap().to_string();

        // correct file extension
        assert_eq!(archive_extension, "tar");

        // archive name contains segment id
        let segment_id = segment
            .current_path
            .file_stem()
            .and_then(|f| f.to_str())
            .unwrap();
        assert!(archive_name.starts_with(segment_id));

        // restore snapshot
        Segment::restore_snapshot(&archive, segment_id).unwrap();

        let restored_segment = load_segment(&snapshot_dir.path().join(segment_id))
            .unwrap()
            .unwrap();

        // validate restored snapshot is the same as original segment
        assert_eq!(segment.vector_dims(), restored_segment.vector_dims());

        assert_eq!(segment.points_count(), restored_segment.points_count());

        for id in segment.iter_points() {
            let vectors = segment.all_vectors(id).unwrap();
            let restored_vectors = restored_segment.all_vectors(id).unwrap();
            assert_eq!(vectors, restored_vectors);

            let payload = segment.payload(id).unwrap();
            let restored_payload = restored_segment.payload(id).unwrap();
            assert_eq!(payload, restored_payload);
        }
    }

    #[test]
    fn test_background_flush() {
        let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "metadata": {
                "height": 50,
                "width": 60
            }
        }"#;

        let segment_base_dir = Builder::new().prefix("segment_dir").tempdir().unwrap();
        let config = SegmentConfig {
            vector_data: HashMap::from([(
                DEFAULT_VECTOR_NAME.to_owned(),
                VectorDataConfig {
                    size: 2,
                    distance: Distance::Dot,
                    hnsw_config: None,
                    quantization_config: None,
                },
            )]),
            index: Indexes::Plain {},
            storage_type: StorageType::InMemory,
            ..Default::default()
        };

        let mut segment = build_segment(segment_base_dir.path(), &config).unwrap();
        segment
            .upsert_vector(0, 0.into(), &only_default_vector(&[1.0, 1.0]))
            .unwrap();

        let payload: Payload = serde_json::from_str(data).unwrap();
        segment.set_full_payload(0, 0.into(), &payload).unwrap();
        segment.flush(false).unwrap();

        // call flush second time to check that background flush finished successful
        segment.flush(true).unwrap();
    }

    #[test]
    fn test_check_consistency() {
        let dir = Builder::new().prefix("segment_dir").tempdir().unwrap();
        let dim = 4;
        let config = SegmentConfig {
            vector_data: HashMap::from([(
                DEFAULT_VECTOR_NAME.to_owned(),
                VectorDataConfig {
                    size: dim,
                    distance: Distance::Dot,
                    hnsw_config: None,
                    quantization_config: None,
                },
            )]),
            index: Indexes::Plain {},
            storage_type: StorageType::InMemory,
            payload_storage_type: Default::default(),
            quantization_config: None,
        };
        let mut segment = build_segment(dir.path(), &config).unwrap();

        let vec4 = vec![1.1, 1.0, 0.0, 1.0];
        segment
            .upsert_vector(100, 4.into(), &only_default_vector(&vec4))
            .unwrap();
        let vec6 = vec![1.0, 1.0, 0.5, 1.0];
        segment
            .upsert_vector(101, 6.into(), &only_default_vector(&vec6))
            .unwrap();

        // first pass on consistent data
        segment.check_consistency_and_repair().unwrap();

        let query_vector = vec![1.0, 1.0, 1.0, 1.0];
        let search_result = segment
            .search(
                DEFAULT_VECTOR_NAME,
                &query_vector,
                &WithPayload::default(),
                &false.into(),
                None,
                10,
                None,
            )
            .unwrap();

        assert_eq!(search_result.len(), 2);
        assert_eq!(search_result[0].id, 6.into());
        assert_eq!(search_result[1].id, 4.into());

        assert!(matches!(
            segment.vector(DEFAULT_VECTOR_NAME, 6.into()),
            Ok(_)
        ));

        let internal_id = segment.lookup_internal_id(6.into()).unwrap();

        // make id_tracker inconsistent
        segment.id_tracker.borrow_mut().drop(6.into()).unwrap();

        let search_result = segment
            .search(
                DEFAULT_VECTOR_NAME,
                &query_vector,
                &WithPayload::default(),
                &false.into(),
                None,
                10,
                None,
            )
            .unwrap();

        // only one result because of inconsistent id_tracker
        assert_eq!(search_result.len(), 1);
        assert_eq!(search_result[0].id, 4.into());

        // querying by external id is broken
        assert!(
            matches!(segment.vector(DEFAULT_VECTOR_NAME, 6.into()), Err(PointIdError {missed_point_id }) if missed_point_id == 6.into())
        );

        // but querying by internal id still works
        matches!(
            segment.vector_by_offset(DEFAULT_VECTOR_NAME, internal_id),
            Ok(Some(_))
        );

        // fix segment's data
        segment.check_consistency_and_repair().unwrap();

        // querying by internal id now consistent
        matches!(
            segment.vector_by_offset(DEFAULT_VECTOR_NAME, internal_id),
            Ok(None)
        );
    }
}
