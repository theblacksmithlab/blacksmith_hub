use crate::rag_system::{ContextBuilder, Document};
use anyhow::Result;
use async_trait::async_trait;

pub struct DefaultContextBuilder {
    separator: String,
}

impl DefaultContextBuilder {
    pub fn new() -> Self {
        Self {
            separator: "\n\n".to_string(),
        }
    }

    pub fn with_separator(mut self, separator: String) -> Self {
        self.separator = separator;
        self
    }
}

#[async_trait]
impl ContextBuilder for DefaultContextBuilder {
    fn build_context(&self, documents: Vec<Document>) -> Result<String> {
        if documents.is_empty() {
            return Ok(String::new());
        }

        let context = documents
            .iter()
            .map(|doc| doc.content.clone())
            .collect::<Vec<String>>()
            .join(&self.separator);

        Ok(context)
    }
}
