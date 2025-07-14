use serde::{Deserialize, Serialize};

/// PKM block data received from the frontend
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PKMBlockData {
    pub id: String,
    pub content: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub created: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub updated: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default)]
    pub properties: serde_json::Value,
    #[serde(default)]
    pub references: Vec<PKMReference>,
}

impl PKMBlockData {
    /// Validate the block data to ensure it meets our requirements
    pub fn validate(&self) -> Result<(), String> {
        let mut errors = Vec::new();
        
        if self.id.is_empty() {
            errors.push("Block ID is empty".to_string());
        }
        
        if self.content.is_empty() {
            errors.push("Block content is empty".to_string());
        }
        
        if self.created.is_empty() {
            errors.push("Created timestamp is empty".to_string());
        }
        
        if self.updated.is_empty() {
            errors.push("Updated timestamp is empty".to_string());
        }
        
        if let Some(parent) = &self.parent {
            if parent.is_empty() {
                errors.push("Parent ID is empty".to_string());
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(", "))
        }
    }
}

/// PKM page data received from the frontend
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PKMPageData {
    pub name: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub created: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub updated: String,
    #[serde(default)]
    pub properties: serde_json::Value,
    #[serde(default)]
    pub blocks: Vec<String>,
}

impl PKMPageData {
    /// Validate the page data to ensure it meets our requirements
    pub fn validate(&self) -> Result<(), String> {
        let mut errors = Vec::new();
        
        if self.name.is_empty() {
            errors.push("Page name is empty".to_string());
        }
        
        if self.created.is_empty() {
            errors.push("Created timestamp is empty".to_string());
        }
        
        if self.updated.is_empty() {
            errors.push("Updated timestamp is empty".to_string());
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(", "))
        }
    }
}

/// Reference within PKM content
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PKMReference {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub id: String,
}

/// Custom deserializer for timestamps that can be either strings or integers
fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct TimestampVisitor;

    impl<'de> serde::de::Visitor<'de> for TimestampVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or an integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(TimestampVisitor)
}