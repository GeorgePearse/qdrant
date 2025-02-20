use crate::entry::entry_point::OperationResult;
use crate::index::field_index::geo_index::GeoMapIndex;
use crate::index::field_index::map_index::MapIndex;
use crate::index::field_index::numeric_index::NumericIndex;
use crate::index::field_index::{CardinalityEstimation, PayloadBlockCondition};
use crate::types::{
    FieldCondition, FloatPayloadType, IntPayloadType, PayloadKeyType, PointOffsetType,
};
use serde_json::Value;

pub trait PayloadFieldIndex {
    /// Return number of points with at least one value indexed in here
    fn indexed_points(&self) -> usize;

    /// Load index from disk.
    fn load(&mut self) -> OperationResult<bool>;

    /// Remove db content of the current payload index
    fn clear(self) -> OperationResult<()>;

    /// Flush all pending updates to disk.
    fn flush(&self) -> OperationResult<()>;

    /// Get iterator over points fitting given `condition`
    /// Return `None` if condition does not match the index type
    fn filter(
        &self,
        condition: &FieldCondition,
    ) -> Option<Box<dyn Iterator<Item = PointOffsetType> + '_>>;

    /// Return estimation of points amount which satisfy given condition
    fn estimate_cardinality(&self, condition: &FieldCondition) -> Option<CardinalityEstimation>;

    /// Iterate conditions for payload blocks with minimum size of `threshold`
    /// Required for building HNSW index
    fn payload_blocks(
        &self,
        threshold: usize,
        key: PayloadKeyType,
    ) -> Box<dyn Iterator<Item = PayloadBlockCondition> + '_>;

    /// Returns an amount of unique indexed points
    fn count_indexed_points(&self) -> usize;
}

pub trait ValueIndexer<T> {
    /// Add multiple values associated with a single point
    fn add_many(&mut self, id: PointOffsetType, values: Vec<T>) -> OperationResult<()>;

    /// Extract index-able value from payload `Value`
    fn get_value(&self, value: &Value) -> Option<T>;

    /// Add point with payload to index
    fn add_point(&mut self, id: PointOffsetType, payload: &Value) -> OperationResult<()> {
        match payload {
            Value::Array(values) => {
                self.add_many(id, values.iter().flat_map(|x| self.get_value(x)).collect())
            }
            _ => {
                if let Some(x) = self.get_value(payload) {
                    self.add_many(id, vec![x])
                } else {
                    Ok(())
                }
            }
        }
    }

    /// remove a point from the index
    fn remove_point(&mut self, id: PointOffsetType) -> OperationResult<()>;
}

/// Common interface for all possible types of field indexes
/// Enables polymorphism on field indexes
/// TODO: Rename with major release
#[allow(clippy::enum_variant_names)]
pub enum FieldIndex {
    IntIndex(NumericIndex<IntPayloadType>),
    IntMapIndex(MapIndex<IntPayloadType>),
    KeywordIndex(MapIndex<String>),
    FloatIndex(NumericIndex<FloatPayloadType>),
    GeoIndex(GeoMapIndex),
}

impl FieldIndex {
    fn get_payload_field_index(&self) -> &dyn PayloadFieldIndex {
        match self {
            FieldIndex::IntIndex(payload_field_index) => payload_field_index,
            FieldIndex::IntMapIndex(payload_field_index) => payload_field_index,
            FieldIndex::KeywordIndex(payload_field_index) => payload_field_index,
            FieldIndex::FloatIndex(payload_field_index) => payload_field_index,
            FieldIndex::GeoIndex(payload_field_index) => payload_field_index,
        }
    }

    #[allow(dead_code)]
    fn get_payload_field_index_mut(&mut self) -> &mut dyn PayloadFieldIndex {
        match self {
            FieldIndex::IntIndex(ref mut payload_field_index) => payload_field_index,
            FieldIndex::IntMapIndex(ref mut payload_field_index) => payload_field_index,
            FieldIndex::KeywordIndex(ref mut payload_field_index) => payload_field_index,
            FieldIndex::FloatIndex(ref mut payload_field_index) => payload_field_index,
            FieldIndex::GeoIndex(ref mut payload_field_index) => payload_field_index,
        }
    }

    pub fn load(&mut self) -> OperationResult<bool> {
        match self {
            FieldIndex::IntIndex(ref mut payload_field_index) => payload_field_index.load(),
            FieldIndex::IntMapIndex(ref mut payload_field_index) => payload_field_index.load(),
            FieldIndex::KeywordIndex(ref mut payload_field_index) => payload_field_index.load(),
            FieldIndex::FloatIndex(ref mut payload_field_index) => payload_field_index.load(),
            FieldIndex::GeoIndex(ref mut payload_field_index) => payload_field_index.load(),
        }
    }

    pub fn clear(self) -> OperationResult<()> {
        match self {
            FieldIndex::IntIndex(index) => index.clear(),
            FieldIndex::IntMapIndex(index) => index.clear(),
            FieldIndex::KeywordIndex(index) => index.clear(),
            FieldIndex::FloatIndex(index) => index.clear(),
            FieldIndex::GeoIndex(index) => index.clear(),
        }
    }

    pub fn recreate(&self) -> OperationResult<()> {
        match self {
            FieldIndex::IntIndex(index) => index.recreate(),
            FieldIndex::IntMapIndex(index) => index.recreate(),
            FieldIndex::KeywordIndex(index) => index.recreate(),
            FieldIndex::FloatIndex(index) => index.recreate(),
            FieldIndex::GeoIndex(index) => index.recreate(),
        }
    }

    pub fn indexed_points(&self) -> usize {
        self.get_payload_field_index().indexed_points()
    }

    pub fn flush(&self) -> OperationResult<()> {
        self.get_payload_field_index().flush()
    }

    pub fn filter(
        &self,
        condition: &FieldCondition,
    ) -> Option<Box<dyn Iterator<Item = PointOffsetType> + '_>> {
        self.get_payload_field_index().filter(condition)
    }

    pub fn estimate_cardinality(
        &self,
        condition: &FieldCondition,
    ) -> Option<CardinalityEstimation> {
        self.get_payload_field_index()
            .estimate_cardinality(condition)
    }

    pub fn payload_blocks(
        &self,
        threshold: usize,
        key: PayloadKeyType,
    ) -> Box<dyn Iterator<Item = PayloadBlockCondition> + '_> {
        self.get_payload_field_index()
            .payload_blocks(threshold, key)
    }

    pub fn count_indexed_points(&self) -> usize {
        self.get_payload_field_index().count_indexed_points()
    }

    pub fn add_point(&mut self, id: PointOffsetType, payload: &Value) -> OperationResult<()> {
        match self {
            FieldIndex::IntIndex(ref mut payload_field_index) => {
                payload_field_index.add_point(id, payload)
            }
            FieldIndex::IntMapIndex(ref mut payload_field_index) => {
                payload_field_index.add_point(id, payload)
            }
            FieldIndex::KeywordIndex(ref mut payload_field_index) => {
                payload_field_index.add_point(id, payload)
            }
            FieldIndex::FloatIndex(ref mut payload_field_index) => {
                payload_field_index.add_point(id, payload)
            }
            FieldIndex::GeoIndex(ref mut payload_field_index) => {
                payload_field_index.add_point(id, payload)
            }
        }
    }

    pub fn remove_point(&mut self, point_id: PointOffsetType) -> OperationResult<()> {
        match self {
            FieldIndex::IntIndex(index) => index.remove_point(point_id),
            FieldIndex::IntMapIndex(index) => index.remove_point(point_id),
            FieldIndex::KeywordIndex(index) => index.remove_point(point_id),
            FieldIndex::FloatIndex(index) => index.remove_point(point_id),
            FieldIndex::GeoIndex(index) => index.remove_point(point_id),
        }
    }
}
