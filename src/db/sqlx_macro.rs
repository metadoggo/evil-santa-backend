macro_rules! maybe_bind_unsep {
  ($sep:ident, $column:expr, $op:expr, $maybe_value:expr) => {
    if let Some(value) = $maybe_value {
      $sep
        .push(format!(" {} {} ", $column, $op))
        .push_bind_unseparated(value);
    } 
  };
  ($sep:ident, $column:expr, $op:expr, $maybe_value:expr, $map:expr) => {
    if let Some(value) = $maybe_value {
      $sep
        .push(format!(" {} {} ", $column, $op))
        .push_bind_unseparated($map(value));
    }
  };
}

macro_rules! maybe_bind_unsep_eq_json {
  ($sep:ident, $column:expr, $option:expr) => {
    if let Some(value) = $option {
      $sep
        .push(format!(" {} = ", $column))
        .push_bind_unseparated(Json(value));
    }
  };
}

macro_rules! bind_unsep {
  ($sep:ident, $column:expr, $op:expr, $value:expr) => {
    $sep
      .push(format!(" {} {} ", $column, $op))
      .push_bind_unseparated($value);
  };
}

macro_rules! offset_limit {
  ($query:ident, $offset:expr, $limit:expr) => {
    $query.push(" OFFSET ").push($offset);
    $query.push(" LIMIT ").push($limit);
  };
}

