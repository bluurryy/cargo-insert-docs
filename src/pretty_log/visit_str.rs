use std::fmt;

use tracing::field::{Field, Visit};

pub trait VisitStr {
    fn record_str(&mut self, field: &Field, value: &str);
}

pub struct VisitAsStr<'a, T>(pub &'a mut T);

impl<T: VisitStr> Visit for VisitAsStr<'_, T> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_str(field, &format!("{value:?}"))
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.record_str(field, &format!("{value}"))
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_str(field, &format!("{value}"))
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_str(field, &format!("{value}"))
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.record_str(field, &format!("{value}"))
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.record_str(field, &format!("{value}"))
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_str(field, &format!("{value}"))
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.record_str(field, value);
    }
}
