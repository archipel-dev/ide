//! Runs provider requests on background threads with cancellation.

use crate::{AiError, CancelToken, ChatRequest, ChatResponse, Provider, StreamEvent};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(Debug, Clone, PartialEq)]
pub enum AiUpdate {
    Event(StreamEvent),
    /// Final outcome; always the last update of a request.
    Finished(Result<ChatResponse, AiError>),
}

pub struct AiService {
    sender: Sender<(u64, AiUpdate)>,
    receiver: Receiver<(u64, AiUpdate)>,
    running: HashMap<u64, CancelToken>,
    /// Updates synthesized locally (cancellation) delivered on the next poll.
    pending: Vec<(u64, AiUpdate)>,
    next_id: u64,
}

impl Default for AiService {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            running: HashMap::new(),
            pending: Vec::new(),
            next_id: 1,
        }
    }
}

impl AiService {
    /// Starts streaming `request` and returns its id.
    pub fn start(&mut self, provider: Arc<dyn Provider>, request: ChatRequest) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let cancel = CancelToken::default();
        self.running.insert(id, cancel.clone());
        let sender = self.sender.clone();
        std::thread::Builder::new()
            .name(format!("archipel-ide-ai-{id}"))
            .spawn(move || {
                let events = sender.clone();
                let result = provider.stream(&request, &cancel, &mut |event| {
                    let _ = events.send((id, AiUpdate::Event(event)));
                });
                let _ = sender.send((id, AiUpdate::Finished(result)));
            })
            .map_err(|error| {
                self.running.remove(&id);
                self.pending.push((
                    id,
                    AiUpdate::Finished(Err(AiError::Network(format!(
                        "could not start request thread: {error}"
                    )))),
                ));
            })
            .ok();
        id
    }

    /// Cancels a request. Its `Finished(Err(Cancelled))` arrives on the next poll and
    /// no further updates are delivered for it, even if the provider thread is still
    /// blocked on the network.
    pub fn cancel(&mut self, id: u64) {
        if let Some(token) = self.running.remove(&id) {
            token.cancel();
            self.pending
                .push((id, AiUpdate::Finished(Err(AiError::Cancelled))));
        }
    }

    pub fn cancel_all(&mut self) {
        let ids: Vec<u64> = self.running.keys().copied().collect();
        for id in ids {
            self.cancel(id);
        }
    }

    pub fn is_running(&self, id: u64) -> bool {
        self.running.contains_key(&id)
    }

    pub fn is_busy(&self) -> bool {
        !self.running.is_empty() || !self.pending.is_empty()
    }

    pub fn poll(&mut self) -> Vec<(u64, AiUpdate)> {
        let mut updates = std::mem::take(&mut self.pending);
        while let Ok((id, update)) = self.receiver.try_recv() {
            if !self.running.contains_key(&id) {
                continue;
            }
            if matches!(update, AiUpdate::Finished(_)) {
                self.running.remove(&id);
            }
            updates.push((id, update));
        }
        updates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatMessage, MockProvider, MockReply};
    use std::time::{Duration, Instant};

    fn drain(service: &mut AiService, id: u64) -> Vec<AiUpdate> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut updates = Vec::new();
        while Instant::now() < deadline {
            for (update_id, update) in service.poll() {
                assert_eq!(update_id, id);
                let finished = matches!(update, AiUpdate::Finished(_));
                updates.push(update);
                if finished {
                    return updates;
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("request did not finish");
    }

    #[test]
    fn streams_in_background_and_finishes() {
        let provider = Arc::new(MockProvider::new([MockReply::text("hello there")]));
        let mut service = AiService::default();
        let id = service.start(
            provider,
            ChatRequest::chat("", vec![ChatMessage::user("hi")]),
        );
        assert!(service.is_busy());
        let updates = drain(&mut service, id);
        assert_eq!(
            updates[1..3],
            [
                AiUpdate::Event(StreamEvent::TextDelta("hello ".into())),
                AiUpdate::Event(StreamEvent::TextDelta("there".into()))
            ]
        );
        assert!(
            matches!(updates.last(), Some(AiUpdate::Finished(Ok(r))) if r.text() == "hello there")
        );
        assert!(!service.is_busy());
    }

    #[test]
    fn cancellation_finishes_immediately_and_silences_the_thread() {
        let mut slow = MockReply::text("a b c d e f g h");
        slow.delay = Duration::from_millis(30);
        let provider = Arc::new(MockProvider::new([slow]));
        let mut service = AiService::default();
        let id = service.start(
            provider,
            ChatRequest::chat("", vec![ChatMessage::user("hi")]),
        );
        service.cancel(id);
        let updates = service.poll();
        assert_eq!(
            updates,
            vec![(id, AiUpdate::Finished(Err(AiError::Cancelled)))]
        );
        std::thread::sleep(Duration::from_millis(100));
        assert!(service.poll().is_empty());
        assert!(!service.is_busy());
    }
}
