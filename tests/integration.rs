use env_sync::sync::{EnvSync, EnvSyncOptions};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_sync_integration() {
  let temp_dir = TempDir::new().unwrap();

  let local_path = temp_dir.path().join(".env");
  let template_path = temp_dir.path().join(".env.template");

  let local_content = "# Database configuration
API_KEY=secret123 # Keep this secret!
DB_HOST=localhost
DB_PORT=";
  let template_content = "# Database configuration
API_KEY=
DB_HOST=production.example.com
DB_PORT=5432 # Default postgres port

# New feature
NEW_VAR=default # Feature flag";

  fs::write(&local_path, local_content).unwrap();
  fs::write(&template_path, template_content).unwrap();

  let options = EnvSyncOptions {
    local_file: Some(local_path.clone()),
    template_file: template_path,
  };

  EnvSync::sync_with_options(options).unwrap();

  let synced_content = fs::read_to_string(&local_path).unwrap();
  let expected = "# Database configuration
API_KEY=secret123 # Keep this secret!
DB_HOST=production.example.com
DB_PORT=5432 # Default postgres port

# New feature
NEW_VAR=default # Feature flag
";

  assert_eq!(synced_content, expected);
}
