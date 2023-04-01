use std::path::Path;
use std::sync::Arc;

use atomic_refcell::AtomicRefCell;
use bitvec::vec::BitVec;
use rand::prelude::StdRng;
use rand::SeedableRng;

use crate::common::Flusher;
use crate::entry::entry_point::OperationResult;
use crate::fixtures::payload_fixtures::{
    generate_diverse_payload, FLT_KEY, GEO_KEY, INT_KEY, STR_KEY, TEXT_KEY,
};
use crate::id_tracker::IdTracker;
use crate::index::plain_payload_index::PlainPayloadIndex;
use crate::index::struct_payload_index::StructPayloadIndex;
use crate::index::PayloadIndex;
use crate::payload_storage::in_memory_payload_storage::InMemoryPayloadStorage;
use crate::payload_storage::query_checker::SimpleConditionChecker;
use crate::payload_storage::PayloadStorage;
use crate::types::{PayloadSchemaType, PointIdType, PointOffsetType, SeqNumberType};

/// Warn: Use for tests only
///
/// This struct mimics the interface of `PointsIterator` and `IdTracker` only for basic cases
pub struct FixtureIdTracker {
    ids: Vec<PointOffsetType>,
    deleted: BitVec,
}

impl FixtureIdTracker {
    pub fn new(num_points: usize) -> Self {
        let mut deleted = BitVec::with_capacity(num_points);
        deleted.resize(num_points, false);
        Self {
            ids: (0..num_points).map(|x| x as PointOffsetType).collect(),
            deleted,
        }
    }
}

impl IdTracker for FixtureIdTracker {
    fn internal_version(&self, _internal_id: PointOffsetType) -> Option<SeqNumberType> {
        Some(0)
    }

    fn set_internal_version(
        &mut self,
        _internal_id: PointOffsetType,
        _version: SeqNumberType,
    ) -> OperationResult<()> {
        Ok(())
    }

    fn internal_id(&self, external_id: PointIdType) -> Option<PointOffsetType> {
        Some(match external_id {
            PointIdType::NumId(id) => {
                assert!(id < self.ids.len() as u64);
                id as PointOffsetType
            }
            PointIdType::Uuid(_) => unreachable!(),
        })
    }

    fn external_id(&self, internal_id: PointOffsetType) -> Option<PointIdType> {
        assert!(internal_id < self.ids.len() as PointOffsetType);
        Some(PointIdType::NumId(internal_id as u64))
    }

    fn set_link(
        &mut self,
        _external_id: PointIdType,
        _internal_id: PointOffsetType,
    ) -> OperationResult<()> {
        Ok(())
    }

    fn drop(&mut self, external_id: PointIdType) -> OperationResult<()> {
        let internal_id = self.internal_id(external_id).unwrap() as usize;
        self.deleted.set(internal_id, true);
        Ok(())
    }

    fn iter_external(&self) -> Box<dyn Iterator<Item = PointIdType> + '_> {
        Box::new(
            self.ids
                .iter()
                .copied()
                .map(|id| PointIdType::NumId(id as u64)),
        )
    }

    fn iter_internal(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(self.ids.iter().copied())
    }

    fn iter_from(
        &self,
        external_id: Option<PointIdType>,
    ) -> Box<dyn Iterator<Item = (PointIdType, PointOffsetType)> + '_> {
        let start = match external_id {
            None => 0,
            Some(id) => match id {
                PointIdType::NumId(num) => num,
                PointIdType::Uuid(_) => unreachable!(),
            },
        } as PointOffsetType;

        Box::new(
            self.ids
                .iter()
                .copied()
                .skip_while(move |x| *x < start)
                .map(|x| (PointIdType::NumId(x as u64), x)),
        )
    }

    fn points_count(&self) -> usize {
        self.ids.len()
    }

    fn iter_ids(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(
            self.ids
                .iter()
                .copied()
                .filter(|id| !self.deleted[*id as usize]),
        )
    }

    fn internal_size(&self) -> usize {
        self.ids.len()
    }

    fn mapping_flusher(&self) -> Flusher {
        Box::new(|| Ok(()))
    }

    fn versions_flusher(&self) -> Flusher {
        Box::new(|| Ok(()))
    }

    fn is_deleted(&self, key: PointOffsetType) -> bool {
        let key = key as usize;
        if key >= self.deleted.len() {
            return true;
        }
        self.deleted[key]
    }

    fn deleted_bitvec(&self) -> &BitVec {
        &self.deleted
    }
}

/// Creates in-memory payload storage and fills it with random points
///
/// # Arguments
///
/// * `num_points` - how many random points to insert
///
/// # Result
///
/// Payload storage fixture
///
fn create_payload_storage_fixture(num_points: usize, seed: u64) -> InMemoryPayloadStorage {
    let mut payload_storage = InMemoryPayloadStorage::default();
    let mut rng = StdRng::seed_from_u64(seed);

    for id in 0..num_points {
        let payload = generate_diverse_payload(&mut rng);
        payload_storage
            .assign(id as PointOffsetType, &payload)
            .unwrap();
    }

    payload_storage
}

/// Function generates `PlainPayloadIndex` with random payload for testing
///
/// # Arguments
///
/// * `path` - temp directory path
/// * `num_points` - how many payloads generate?
///
/// # Result
///
/// `PlainPayloadIndex`
///
pub fn create_plain_payload_index(path: &Path, num_points: usize, seed: u64) -> PlainPayloadIndex {
    let payload_storage = create_payload_storage_fixture(num_points, seed);
    let id_tracker = Arc::new(AtomicRefCell::new(FixtureIdTracker::new(num_points)));

    let condition_checker = Arc::new(SimpleConditionChecker::new(
        Arc::new(AtomicRefCell::new(payload_storage.into())),
        id_tracker.clone(),
    ));

    PlainPayloadIndex::open(condition_checker, id_tracker, path).unwrap()
}

/// Function generates `StructPayloadIndex` with random payload for testing.
/// It will also create indexes for payloads
///
/// # Arguments
///
/// * `path` - temp directory path
/// * `num_points` - how many payloads generate?
///
/// # Result
///
/// `StructPayloadIndex`
///
pub fn create_struct_payload_index(
    path: &Path,
    num_points: usize,
    seed: u64,
) -> StructPayloadIndex {
    let payload_storage = Arc::new(AtomicRefCell::new(
        create_payload_storage_fixture(num_points, seed).into(),
    ));
    let id_tracker = Arc::new(AtomicRefCell::new(FixtureIdTracker::new(num_points)));

    let mut index = StructPayloadIndex::open(payload_storage, id_tracker, path).unwrap();

    index
        .set_indexed(STR_KEY, PayloadSchemaType::Keyword.into())
        .unwrap();
    index
        .set_indexed(INT_KEY, PayloadSchemaType::Integer.into())
        .unwrap();
    index
        .set_indexed(FLT_KEY, PayloadSchemaType::Float.into())
        .unwrap();
    index
        .set_indexed(GEO_KEY, PayloadSchemaType::Geo.into())
        .unwrap();

    index
        .set_indexed(TEXT_KEY, PayloadSchemaType::Text.into())
        .unwrap();

    index
}
