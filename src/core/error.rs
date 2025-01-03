use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct ErrorRegistry {
    errors: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl ErrorRegistry {
    pub fn new() -> Self {
        ErrorRegistry {
            errors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn record_error(&self, category: &str, message: String) {
        let mut errors = self.errors.lock().unwrap();
        errors
            .entry(category.to_string())
            .or_insert_with(Vec::new)
            .push(message);
    }

    pub fn get_errors(&self, category: &str) -> Vec<String> {
        let errors = self.errors.lock().unwrap();
        errors
            .get(category)
            .cloned()
            .unwrap_or_else(Vec::new)
    }
}