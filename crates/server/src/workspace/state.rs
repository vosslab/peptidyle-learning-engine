use std::sync::Arc;

pub(super) struct WorkspaceRouteState<S, B> {
    pub(super) store: Arc<S>,
    pub(super) backends: Arc<B>,
}

impl<S, B> Clone for WorkspaceRouteState<S, B> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            backends: Arc::clone(&self.backends),
        }
    }
}
