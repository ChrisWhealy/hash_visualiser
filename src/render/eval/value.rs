// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A value flowing through the graph: a single word, or a (possibly nested) array. A 1-D vector is `Array` of
/// `Scalar`s; a 2-D matrix is `Array` of `Array`s.
#[derive(Clone)]
pub(crate) enum Value {
    Scalar(u64),
    Array(Vec<Value>),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) fn as_scalar(v: &Value) -> Option<u64> {
    match v {
        Value::Scalar(s) => Some(*s),
        Value::Array(_) => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) fn as_row(v: &Value) -> Option<Vec<u64>> {
    match v {
        Value::Array(items) => items.iter().map(as_scalar).collect(),
        Value::Scalar(_) => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(crate) fn as_matrix(v: &Value) -> Option<Vec<Vec<u64>>> {
    match v {
        Value::Array(rows) => rows.iter().map(as_row).collect(),
        Value::Scalar(_) => None,
    }
}

