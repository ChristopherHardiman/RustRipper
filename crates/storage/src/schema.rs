//! SQLite database schema and operations

use rustripper_core::Result;
use std::path::Path;

/// Database interface for job and rip history
pub struct Database {
    path: std::path::PathBuf,
}

impl Database {
    /// Create a new database connection
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Initialize database schema
    pub async fn init(&self) -> Result<()> {
        // TODO: Create tables and migrations
        Ok(())
    }

    /// Save job to database
    pub async fn save_job(&self, job_id: &str) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Load job from database
    pub async fn load_job(&self, job_id: &str) -> Result<()> {
        // TODO: Implement
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::new("/tmp/test.db");
        assert_eq!(db.path, std::path::PathBuf::from("/tmp/test.db"));
    }
}
