use crate::api::ApiClientError;

#[derive(Debug, Clone, Default)]
pub enum LoadingState<T> {
    #[default]
    Idle,
    Loading,
    Ready(T),
    Failed(ApiClientError),
}

impl<T> LoadingState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }
}
