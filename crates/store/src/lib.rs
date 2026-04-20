use ohmywu_domain::AppSettings;

#[derive(Debug, Clone)]
pub struct InMemoryStore {
    settings: AppSettings,
}

impl InMemoryStore {
    pub fn new(settings: AppSettings) -> Self {
        Self { settings }
    }

    pub fn settings(&self) -> AppSettings {
        self.settings.clone()
    }
}
