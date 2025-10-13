//! Iterator trait implementations

use crate::{EventBatch, ProtoEvent};
use std::iter::FromIterator;

/// Implement FromIterator for EventBatch
///
/// This allows collecting iterators of ProtoEvent into an EventBatch.
///
/// # Example
///
/// ```
/// use proton_beam_core::{EventBatch, ProtoEvent, ProtoEventBuilder};
///
/// let events: Vec<ProtoEvent> = vec![
///     ProtoEventBuilder::new().id("1").build(),
///     ProtoEventBuilder::new().id("2").build(),
///     ProtoEventBuilder::new().id("3").build(),
/// ];
///
/// // Collect into EventBatch
/// let batch: EventBatch = events.into_iter().collect();
/// assert_eq!(batch.events.len(), 3);
/// ```
impl FromIterator<ProtoEvent> for EventBatch {
    fn from_iter<T: IntoIterator<Item = ProtoEvent>>(iter: T) -> Self {
        EventBatch {
            events: iter.into_iter().collect(),
        }
    }
}

/// Allow EventBatch to be extended from an iterator
impl Extend<ProtoEvent> for EventBatch {
    fn extend<T: IntoIterator<Item = ProtoEvent>>(&mut self, iter: T) {
        self.events.extend(iter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtoEventBuilder;

    #[test]
    fn test_from_iterator_empty() {
        let events: Vec<ProtoEvent> = vec![];
        let batch: EventBatch = events.into_iter().collect();

        assert_eq!(batch.events.len(), 0);
    }

    #[test]
    fn test_from_iterator_single() {
        let events = vec![ProtoEventBuilder::new().id("test1").build()];

        let batch: EventBatch = events.into_iter().collect();

        assert_eq!(batch.events.len(), 1);
        assert_eq!(batch.events[0].id, "test1");
    }

    #[test]
    fn test_from_iterator_multiple() {
        let events = vec![
            ProtoEventBuilder::new()
                .id("event1")
                .content("First")
                .build(),
            ProtoEventBuilder::new()
                .id("event2")
                .content("Second")
                .build(),
            ProtoEventBuilder::new()
                .id("event3")
                .content("Third")
                .build(),
        ];

        let batch: EventBatch = events.into_iter().collect();

        assert_eq!(batch.events.len(), 3);
        assert_eq!(batch.events[0].id, "event1");
        assert_eq!(batch.events[1].id, "event2");
        assert_eq!(batch.events[2].id, "event3");
    }

    #[test]
    fn test_from_iterator_with_map() {
        let ids = vec!["a", "b", "c"];

        let batch: EventBatch = ids
            .into_iter()
            .map(|id| ProtoEventBuilder::new().id(id).kind(1).build())
            .collect();

        assert_eq!(batch.events.len(), 3);
        assert_eq!(batch.events[0].id, "a");
        assert_eq!(batch.events[1].id, "b");
        assert_eq!(batch.events[2].id, "c");
        assert!(batch.events.iter().all(|e| e.kind == 1));
    }

    #[test]
    fn test_from_iterator_with_filter() {
        let events = vec![
            ProtoEventBuilder::new().id("1").kind(1).build(),
            ProtoEventBuilder::new().id("2").kind(2).build(),
            ProtoEventBuilder::new().id("3").kind(1).build(),
            ProtoEventBuilder::new().id("4").kind(3).build(),
        ];

        // Filter and collect only kind 1 events
        let batch: EventBatch = events.into_iter().filter(|e| e.kind == 1).collect();

        assert_eq!(batch.events.len(), 2);
        assert_eq!(batch.events[0].id, "1");
        assert_eq!(batch.events[1].id, "3");
    }

    #[test]
    fn test_extend() {
        let mut batch = EventBatch {
            events: vec![ProtoEventBuilder::new().id("1").build()],
        };

        let new_events = vec![
            ProtoEventBuilder::new().id("2").build(),
            ProtoEventBuilder::new().id("3").build(),
        ];

        batch.extend(new_events);

        assert_eq!(batch.events.len(), 3);
        assert_eq!(batch.events[0].id, "1");
        assert_eq!(batch.events[1].id, "2");
        assert_eq!(batch.events[2].id, "3");
    }

    #[test]
    fn test_extend_from_iterator() {
        let mut batch = EventBatch {
            events: vec![ProtoEventBuilder::new().id("start").build()],
        };

        batch.extend((1..=3).map(|i| ProtoEventBuilder::new().id(format!("event{}", i)).build()));

        assert_eq!(batch.events.len(), 4);
        assert_eq!(batch.events[0].id, "start");
        assert_eq!(batch.events[1].id, "event1");
        assert_eq!(batch.events[2].id, "event2");
        assert_eq!(batch.events[3].id, "event3");
    }
}
