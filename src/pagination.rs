//! Turns cursor-paged list endpoints into a [`Stream`] of items.

use std::future::Future;
use std::pin::Pin;

use futures_core::Stream;
use futures_util::{stream, StreamExt, TryStreamExt};

use crate::error::Result;
use crate::models::Page;

/// A boxed stream of list items, as returned by the `list_all` methods.
///
/// Each item is a `Result`: a failed page request ends the stream with that
/// error.
pub type ItemStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;

enum State {
    Start(Option<String>),
    Next(String),
    Done,
}

/// Calls `fetch` with each cursor in turn until `next_cursor` is `None`.
///
/// `fetch` receives `None` for the first page (or the starting cursor the
/// caller passed). The stream also stops if the server hands back the cursor
/// it was just given, so a server bug cannot loop forever.
pub(crate) fn paginate<T, F, Fut>(start: Option<String>, mut fetch: F) -> ItemStream<T>
where
    T: Send + 'static,
    F: FnMut(Option<String>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<Page<T>>> + Send + 'static,
{
    let pages = stream::try_unfold(State::Start(start), move |state| {
        let cursor = match state {
            State::Done => None,
            State::Start(c) => Some(c),
            State::Next(c) => Some(Some(c)),
        };
        let fut = cursor.map(|c| (c.clone(), fetch(c)));
        async move {
            let Some((sent, fut)) = fut else {
                return Ok::<_, crate::Error>(None);
            };
            let page = fut.await?;
            let next = match page.next_cursor {
                Some(n) if !n.is_empty() && Some(&n) != sent.as_ref() => State::Next(n),
                _ => State::Done,
            };
            Ok(Some((page.items, next)))
        }
    });
    pages
        .map_ok(|items| stream::iter(items.into_iter().map(Ok)))
        .try_flatten()
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn walks_all_pages_and_stops() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen2 = seen.clone();
        let s = paginate(None, move |cursor: Option<String>| {
            seen2.lock().unwrap().push(cursor.clone());
            async move {
                Ok(match cursor.as_deref() {
                    None => Page {
                        items: vec![1, 2],
                        next_cursor: Some("b".into()),
                    },
                    Some("b") => Page {
                        items: vec![3],
                        next_cursor: Some("c".into()),
                    },
                    _ => Page {
                        items: vec![4],
                        next_cursor: None,
                    },
                })
            }
        });
        let all: Vec<i32> = s.try_collect().await.unwrap();
        assert_eq!(all, vec![1, 2, 3, 4]);
        assert_eq!(
            *seen.lock().unwrap(),
            vec![None, Some("b".to_string()), Some("c".to_string())]
        );
    }

    #[tokio::test]
    async fn repeated_cursor_ends_the_stream() {
        let s = paginate(None, |_c: Option<String>| async {
            Ok(Page {
                items: vec![1],
                next_cursor: Some("same".into()),
            })
        });
        let all: Vec<i32> = s.try_collect().await.unwrap();
        assert_eq!(all, vec![1, 1]);
    }
}
