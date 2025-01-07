use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ErrorRegistry {
    errors: HashMap<String, Vec<String>>,
}

impl ErrorRegistry {
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    pub fn register_error(&mut self, error: &str) -> String {
        let error_id = format!("ERR_{}", self.errors.len());
        self.errors
            .entry(error_id.clone())
            .or_insert_with(Vec::new)
            .push(error.to_string());
        error_id
    }

    pub fn get_errors(&self, error_id: &str) -> Option<&Vec<String>> {
        self.errors.get(error_id)
    }
}
