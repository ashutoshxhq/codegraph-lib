use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    Function,
    Method,
    Class,
    Interface,
    Module,
    TypeDefinition,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CodeNode {
    pub id: String,
    pub node_type: NodeType,
    pub name: String,
    pub file_path: String,
    pub line_range: (usize, usize),
    pub content: String,
    pub summary: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl Hash for CodeNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.node_type.hash(state);
        self.name.hash(state);
        self.file_path.hash(state);
        self.line_range.hash(state);
        self.content.hash(state);
        self.summary.hash(state);
    }
}

impl Serialize for CodeNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("CodeNode", 7)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("node_type", &self.node_type)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("file_path", &self.file_path)?;
        state.serialize_field("line_range", &self.line_range)?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("metadata", &self.metadata)?;
        state.end()
    }
}

impl CodeNode {
    pub fn new(
        id: String,
        node_type: NodeType,
        name: String,
        file_path: String,
        line_range: (usize, usize),
        content: String,
    ) -> Self {
        CodeNode {
            id,
            node_type,
            name,
            file_path,
            line_range,
            content,
            summary: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = Some(summary);
        self
    }

    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.add_metadata(key, value);
        self
    }
}
