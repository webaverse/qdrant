use crate::vector_storage::vector_storage::{VectorMatcher, ScoredPoint, VectorCounter};
use crate::index::index::{Index, PayloadIndex};
use crate::types::{Filter, PointOffsetType, ScoreType, VectorElementType};
use crate::payload_storage::payload_storage::{ConditionChecker};
use std::rc::Rc;
use std::cell::RefCell;


pub struct PlainPayloadIndex {
    condition_checker: Rc<RefCell<dyn ConditionChecker>>,
    vector_counter: Rc<RefCell<dyn VectorCounter>>,
}


impl PlainPayloadIndex {
    pub fn new(condition_checker: Rc<RefCell<dyn ConditionChecker>>,
               vector_counter: Rc<RefCell<dyn VectorCounter>>) -> Self {
        PlainPayloadIndex {
            condition_checker,
            vector_counter,
        }
    }
}

impl PayloadIndex for PlainPayloadIndex {
    fn estimate_cardinality(&self, query: &Filter) -> (usize, usize) {
        let mut matched_points = 0;
        let vector_count = self.vector_counter.borrow().vector_count();
        let condition_checker = self.condition_checker.borrow();
        for i in 0..vector_count {
            if condition_checker.check(i, query) {
                matched_points += 1;
            }
        }
        (matched_points, matched_points)
    }

    fn query_points(&self, query: &Filter) -> Vec<usize> {
        let mut matched_points = vec![];
        let vector_count = self.vector_counter.borrow().vector_count();
        let condition_checker = self.condition_checker.borrow();
        for i in 0..vector_count {
            if condition_checker.check(i, query) {
                matched_points.push(i);
            }
        }
        return matched_points;
    }
}


pub struct PlainIndex {
    vector_matcher: Rc<RefCell<dyn VectorMatcher>>,
    payload_index: Rc<RefCell<dyn PayloadIndex>>,
}

impl PlainIndex {
    pub fn new(
        vector_matcher: Rc<RefCell<dyn VectorMatcher>>,
        payload_index: Rc<RefCell<dyn PayloadIndex>>,
    ) -> PlainIndex {
        return PlainIndex {
            vector_matcher,
            payload_index,
        };
    }
}


impl Index for PlainIndex {
    fn search(&self, vector: &Vec<VectorElementType>, filter: Option<&Filter>, top: usize) -> Vec<(PointOffsetType, ScoreType)> {
        match filter {
            Some(filter) => {
                let filtered_ids = self.payload_index.borrow().query_points(filter);
                self.vector_matcher.borrow().score_points(vector, &filtered_ids, 0)
            }
            None => self.vector_matcher.borrow().score_all(vector, top)
        }.iter().map(ScoredPoint::to_tuple).collect()
    }
}