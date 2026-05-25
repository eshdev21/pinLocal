use serde::{Deserialize, Serialize};
use rand::prelude::IndexedRandom;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub board_ids: Vec<i32>,
    pub folder_paths: Vec<String>, // Helpful for UI display
}

impl Workspace {
    pub fn new(name: Option<String>, board_ids: Vec<i32>, folder_paths: Vec<String>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let name = name.unwrap_or_else(Self::generate_random_name);
        Self {
            id,
            name,
            board_ids,
            folder_paths,
        }
    }

    fn generate_random_name() -> String {
        const ADJECTIVES: &[&str] = &["Vibrant", "Silent", "Golden", "Mystic", "Ethereal", "Urban", "Wild", "Serene", "Cosmic", "Lush"];
        const NOUNS: &[&str] = &["Gallery", "Collection", "Archive", "Vault", "Harbor", "Studio", "Canvas", "Spectrum", "Vista", "Nexus"];
        
        let mut rng = rand::rng();
        format!("{} {}", ADJECTIVES.choose(&mut rng).unwrap(), NOUNS.choose(&mut rng).unwrap())
    }
}


// Legacy migration logic removed. Configuration is now managed via SQLite and AppConfig struct.
