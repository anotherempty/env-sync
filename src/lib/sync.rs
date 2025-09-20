use std::path::{Path, PathBuf};

use crate::parse::{EnvEntry, EnvFile, ParseError};

const DEFAULT_LOCAL_FILENAME: &str = ".env";
const DEFAULT_TEMPLATE_FILENAME: &str = ".env.template";

pub struct EnvSync;

impl EnvSync {
  pub fn sync_with_options(options: EnvSyncOptions) -> Result<(), EnvSyncError> {
    let EnvSyncOptions {
      local_file,
      template_file,
    } = options;

    let local_path = local_file.unwrap_or_else(|| {
      std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DEFAULT_LOCAL_FILENAME)
    });
    let template_path = template_file.unwrap_or_else(|| {
      std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DEFAULT_TEMPLATE_FILENAME)
    });

    let local_str = std::fs::read_to_string(&local_path).map_err(EnvSyncError::LocalIo)?;
    let template_str = std::fs::read_to_string(&template_path).map_err(EnvSyncError::TemplateIo)?;

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
    for entry in &mut template.entries {
      if let EnvEntry::Variable(template_var) = entry
        && let Some(local_var) = local.get(&template_var.key)
        && template_var.value.is_empty()
        && !local_var.value.is_empty()
      {
        template_var.value = local_var.value.clone();
      }
    }

    Ok(template)
  }

  fn update_local<P: AsRef<Path>>(local: EnvFile, local_path: P) -> Result<(), EnvSyncError> {
    let content = local.to_string();
    std::fs::write(local_path, content).map_err(EnvSyncError::Write)?;
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
}

pub struct EnvSyncOptions {
  pub local_file: Option<PathBuf>,
  pub template_file: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sync() {
    let local_content = "KEY1=value1\nKEY2=value2\nKEY3=";
    let template_content = "KEY1=\nKEY2=template_value\nKEY3=template_value3\nKEY4=new_key";

    let local: EnvFile = local_content.try_into().unwrap();
    let template: EnvFile = template_content.try_into().unwrap();

    let synced = EnvSync::sync(local, template).unwrap();

    assert_eq!(synced.get("KEY1").unwrap().value, "value1");
    assert_eq!(synced.get("KEY2").unwrap().value, "template_value");
    assert_eq!(synced.get("KEY3").unwrap().value, "template_value3");
    assert_eq!(synced.get("KEY4").unwrap().value, "new_key");
  }
}
