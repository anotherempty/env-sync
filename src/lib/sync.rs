use std::path::{Path, PathBuf};

#[cfg(feature = "tracing")]
use tracing::{debug, info, trace};

use crate::parse::{EnvEntry, EnvFile, ParseError};

const DEFAULT_LOCAL_FILENAME: &str = ".env";

pub struct EnvSync;

impl EnvSync {
  pub fn sync_with_options(options: EnvSyncOptions) -> Result<(), EnvSyncError> {
    #[cfg(feature = "tracing")]
    info!("Starting env sync");

    let EnvSyncOptions {
      local_file,
      template_file,
    } = options;

    let local_path = local_file.unwrap_or_else(|| {
      std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DEFAULT_LOCAL_FILENAME)
    });

    #[cfg(feature = "tracing")]
    debug!(?local_path, ?template_file, "Resolved file paths");

    // Create local file if it doesn't exist
    if !local_path.exists() {
      #[cfg(feature = "tracing")]
      debug!("Creating local file: {:?}", local_path);
      std::fs::write(&local_path, "").map_err(EnvSyncError::CreateLocal)?;
    }

    // Create template file if it doesn't exist
    if !template_file.exists() {
      #[cfg(feature = "tracing")]
      debug!("Creating template file: {:?}", template_file);
      std::fs::write(&template_file, "").map_err(EnvSyncError::CreateTemplate)?;
    }

    let local_str = std::fs::read_to_string(&local_path).map_err(EnvSyncError::LocalIo)?;
    let template_str = std::fs::read_to_string(&template_file).map_err(EnvSyncError::TemplateIo)?;

    let local_content = local_str
      .as_str()
      .try_into()
      .map_err(EnvSyncError::LocalParse)?;

    let template_content = template_str
      .as_str()
      .try_into()
      .map_err(EnvSyncError::TemplateParse)?;

    let synced = Self::sync(local_content, template_content)?;

    Self::update_local(synced, local_path)
  }

  fn sync<'a>(local: EnvFile<'a>, mut template: EnvFile<'a>) -> Result<EnvFile<'a>, EnvSyncError> {
    #[cfg(feature = "tracing")]
    debug!(
      "Starting sync of {} template entries",
      template.entries.len()
    );

    for entry in &mut template.entries {
      if let EnvEntry::Variable(template_var) = entry
        && let Some(local_var) = local.get(&template_var.key)
      {
        #[cfg(feature = "tracing")]
        trace!("Processing variable: {}", template_var.key);

        // Copy value if template is empty
        if template_var.value.is_empty() && !local_var.value.is_empty() {
          #[cfg(feature = "tracing")]
          trace!(
            "Copying local value for {}: {}",
            template_var.key, local_var.value
          );
          template_var.value = local_var.value.clone();
        }

        // Copy inline comment if template doesn't have one
        if template_var.inline_comment.is_none() && local_var.inline_comment.is_some() {
          #[cfg(feature = "tracing")]
          trace!("Copying inline comment for {}", template_var.key);
          template_var.inline_comment = local_var.inline_comment.clone();
        }

        // Copy preceding comments if template doesn't have any
        if template_var.preceding_comments.is_empty() && !local_var.preceding_comments.is_empty() {
          #[cfg(feature = "tracing")]
          trace!(
            "Copying {} preceding comments for {}",
            local_var.preceding_comments.len(),
            template_var.key
          );
          template_var.preceding_comments = local_var.preceding_comments.clone();
        }
      }
    }

    Ok(template)
  }

  fn update_local<P: AsRef<Path>>(local: EnvFile, local_path: P) -> Result<(), EnvSyncError> {
    #[cfg(feature = "tracing")]
    debug!("Writing synced content to {:?}", local_path.as_ref());

    let content = local.to_string();
    std::fs::write(local_path, content).map_err(EnvSyncError::Write)?;

    #[cfg(feature = "tracing")]
    info!("Sync completed successfully");

    Ok(())
  }
}

#[derive(Debug, thiserror::Error)]
pub enum EnvSyncError {
  #[error("Local file IO error: {0}")]
  LocalIo(std::io::Error),
  #[error("Local file parse error: {0}")]
  LocalParse(ParseError),
  #[error("Template file IO error: {0}")]
  TemplateIo(std::io::Error),
  #[error("Template file parse error: {0}")]
  TemplateParse(ParseError),
  #[error("Write error: {0}")]
  Write(std::io::Error),
  #[error("Failed to create local file: {0}")]
  CreateLocal(std::io::Error),
  #[error("Failed to create template file: {0}")]
  CreateTemplate(std::io::Error),
}

pub struct EnvSyncOptions {
  pub local_file: Option<PathBuf>,
  pub template_file: PathBuf,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sync() {
    let local_content = "# Comment for KEY1\nKEY1=value1\nKEY2=value2 # inline comment\nKEY3=";
    let template_content = "KEY1=\nKEY2=template_value\nKEY3=template_value3\nKEY4=new_key";

    let local: EnvFile = local_content.try_into().unwrap();
    let template: EnvFile = template_content.try_into().unwrap();

    let synced = EnvSync::sync(local, template).unwrap();

    let key1 = synced.get("KEY1").unwrap();
    assert_eq!(key1.value, "value1");
    assert_eq!(key1.preceding_comments.len(), 1);

    let key2 = synced.get("KEY2").unwrap();
    assert_eq!(key2.value, "template_value");
    assert_eq!(
      key2.inline_comment.as_ref().unwrap().to_string(),
      "# inline comment"
    );

    assert_eq!(synced.get("KEY3").unwrap().value, "template_value3");
    assert_eq!(synced.get("KEY4").unwrap().value, "new_key");
  }
}
