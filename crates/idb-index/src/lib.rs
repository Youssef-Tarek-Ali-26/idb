use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearnedPositionModel {
    pub slope: f64,
    pub intercept: f64,
    pub max_error: usize,
    pub item_count: usize,
    min_key: u128,
    max_key: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictionWindow {
    pub lower: usize,
    pub upper: usize,
}

impl PredictionWindow {
    pub fn range(&self) -> RangeInclusive<usize> {
        self.lower..=self.upper
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IndexError {
    #[error("cannot train learned index with empty keys")]
    EmptyTrainingSet,
}

pub fn train_linear_model(sorted_keys: &[u128]) -> Result<LearnedPositionModel, IndexError> {
    if sorted_keys.is_empty() {
        return Err(IndexError::EmptyTrainingSet);
    }

    let item_count = sorted_keys.len();
    let min_key = sorted_keys[0];
    let max_key = sorted_keys[item_count - 1];

    let (slope, intercept) = if max_key == min_key {
        (0.0, 0.0)
    } else {
        let span = (max_key - min_key) as f64;
        let slope = (item_count.saturating_sub(1) as f64) / span;
        let intercept = -(min_key as f64) * slope;
        (slope, intercept)
    };

    let mut max_error = 0usize;
    for (position, key) in sorted_keys.iter().enumerate() {
        let predicted = predict_position_raw(*key, slope, intercept, item_count);
        let error = position.abs_diff(predicted);
        max_error = max_error.max(error);
    }

    Ok(LearnedPositionModel {
        slope,
        intercept,
        max_error,
        item_count,
        min_key,
        max_key,
    })
}

impl LearnedPositionModel {
    pub fn predict_position(&self, key: u128) -> usize {
        predict_position_raw(key, self.slope, self.intercept, self.item_count)
    }

    pub fn prediction_window(&self, key: u128) -> PredictionWindow {
        let predicted = self.predict_position(key);
        let lower = predicted.saturating_sub(self.max_error);
        let upper = predicted
            .saturating_add(self.max_error)
            .min(self.item_count.saturating_sub(1));
        PredictionWindow { lower, upper }
    }

    pub fn training_bounds(&self) -> (u128, u128) {
        (self.min_key, self.max_key)
    }
}

fn predict_position_raw(key: u128, slope: f64, intercept: f64, item_count: usize) -> usize {
    if item_count == 0 {
        return 0;
    }

    let predicted = slope.mul_add(key as f64, intercept).round();
    let lower = 0.0f64;
    let upper = item_count.saturating_sub(1) as f64;
    predicted.clamp(lower, upper) as usize
}

pub fn exact_lookup(
    sorted_keys: &[u128],
    model: &LearnedPositionModel,
    key: u128,
) -> Option<usize> {
    if sorted_keys.is_empty() {
        return None;
    }

    let window = model.prediction_window(key);
    let lower = window.lower.min(sorted_keys.len() - 1);
    let upper = window.upper.min(sorted_keys.len() - 1);
    if lower > upper {
        return None;
    }

    match sorted_keys[lower..=upper].binary_search(&key) {
        Ok(relative) => Some(lower + relative),
        Err(_) => None,
    }
}

pub fn lower_bound(sorted_keys: &[u128], model: &LearnedPositionModel, key: u128) -> usize {
    if sorted_keys.is_empty() {
        return 0;
    }

    let window = model.prediction_window(key);
    let lower = window.lower.min(sorted_keys.len() - 1);
    let upper = window.upper.min(sorted_keys.len() - 1);

    let slice = &sorted_keys[lower..=upper];
    let rel = slice.partition_point(|candidate| *candidate < key);
    (lower + rel).min(sorted_keys.len())
}

pub fn upper_bound(sorted_keys: &[u128], model: &LearnedPositionModel, key: u128) -> usize {
    if sorted_keys.is_empty() {
        return 0;
    }

    let window = model.prediction_window(key);
    let lower = window.lower.min(sorted_keys.len() - 1);
    let upper = window.upper.min(sorted_keys.len() - 1);

    let slice = &sorted_keys[lower..=upper];
    let rel = slice.partition_point(|candidate| *candidate <= key);
    (lower + rel).min(sorted_keys.len())
}

#[cfg(test)]
mod tests {
    use super::{exact_lookup, lower_bound, train_linear_model, upper_bound, IndexError};

    fn sample_keys() -> Vec<u128> {
        vec![10, 20, 25, 40, 70, 90, 120, 160, 200, 260]
    }

    #[test]
    fn training_rejects_empty_input() {
        let err = train_linear_model(&[]).expect_err("empty training set should fail");
        assert_eq!(err, IndexError::EmptyTrainingSet);
    }

    #[test]
    fn prediction_window_is_bounded_to_dataset() {
        let keys = sample_keys();
        let model = train_linear_model(&keys).expect("model");

        let low_window = model.prediction_window(0);
        assert_eq!(low_window.lower, 0);
        assert!(low_window.upper < keys.len());

        let high_window = model.prediction_window(10_000);
        assert!(high_window.lower < keys.len());
        assert_eq!(high_window.upper, keys.len() - 1);
    }

    #[test]
    fn exact_lookup_matches_true_positions() {
        let keys = sample_keys();
        let model = train_linear_model(&keys).expect("model");

        for (idx, key) in keys.iter().enumerate() {
            let found = exact_lookup(&keys, &model, *key);
            assert_eq!(found, Some(idx), "key {} position mismatch", key);
        }

        assert_eq!(exact_lookup(&keys, &model, 999), None);
        assert_eq!(exact_lookup(&keys, &model, 11), None);
    }

    #[test]
    fn bounds_approximate_range_search_positions() {
        let keys = sample_keys();
        let model = train_linear_model(&keys).expect("model");

        let lb = lower_bound(&keys, &model, 26);
        let ub = upper_bound(&keys, &model, 160);

        assert_eq!(lb, 3); // first >= 26 is 40 at index 3
        assert_eq!(ub, 8); // first > 160 is 200 at index 8
    }
}
