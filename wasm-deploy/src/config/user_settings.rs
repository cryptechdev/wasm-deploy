use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserSettings {
    pub store_code_chunk_size: usize,
}

impl Default for UserSettings {
    fn default() -> Self {
        UserSettings {
            store_code_chunk_size: 2,
        }
    }
}
