use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    Calls,
    Imports,
    Inherits,
    References,
    Implements,
    Contains,
    DependsOn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub relationship_type: RelationshipType,
    pub from_id: String,
    pub to_id: String,
    pub metadata: HashMap<String, String>,
}

impl Relationship {
    pub fn new(relationship_type: RelationshipType, from_id: String, to_id: String) -> Self {
        Relationship {
            relationship_type,
            from_id,
            to_id,
            metadata: HashMap::new(),
        }
    }

    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.add_metadata(key, value);
        self
    }
}
