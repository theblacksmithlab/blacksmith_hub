use crate::rag_system::types::DocumentType;
use crate::rag_system::ContextBuilder;
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
    fn build_context(&self, documents: Vec<DocumentType>) -> Result<String> {
        let context = documents
            .iter()
            .map(|doc| match doc {
                DocumentType::Default(d) => d.text.clone(),
                DocumentType::W3A(d) => d.text.clone(),
                DocumentType::HybridSearch(d) => {
                    let mut header = format!("=== {} ===", d.metadata.title);

                    if let Some(extra) = &d.metadata.extra {
                        header.push_str(&format!("\n{}", extra));
                    }

                    format!("{}\n\n{}", header, d.text)
                }
            })
            .collect::<Vec<String>>()
            .join("\n\n");

        Ok(context)
    }
}
