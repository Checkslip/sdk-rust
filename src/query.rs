//! Helpers that turn parameter structs into query pairs.

use time::OffsetDateTime;

pub(crate) type Query = Vec<(&'static str, String)>;

pub(crate) fn push_opt<T: ToString>(q: &mut Query, key: &'static str, v: &Option<T>) {
    if let Some(v) = v {
        q.push((key, v.to_string()));
    }
}

pub(crate) fn push_list<T: ToString>(q: &mut Query, key: &'static str, v: &[T]) {
    for item in v {
        q.push((key, item.to_string()));
    }
}

pub(crate) fn push_time(q: &mut Query, key: &'static str, v: Option<OffsetDateTime>) {
    if let Some(t) = v {
        if let Ok(s) = t.format(&time::format_description::well_known::Rfc3339) {
            q.push((key, s));
        }
    }
}

pub(crate) fn push_date(q: &mut Query, key: &'static str, v: Option<time::Date>) {
    if let Some(d) = v {
        q.push((
            key,
            format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day()),
        ));
    }
}
