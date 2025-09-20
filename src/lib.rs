use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EnvFile<'a> {
  pub entries: Vec<EnvEntry<'a>>,
}

impl<'a> fmt::Display for EnvFile<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for entry in &self.entries {
      write!(f, "{}", entry)?;
    }
    Ok(())
  }
}

impl<'a> Serialize for EnvFile<'a> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

impl<'de: 'a, 'a> Deserialize<'de> for EnvFile<'a> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = <&'de str>::deserialize(deserializer)?;
    s.parse().map_err(de::Error::custom)
  }
}

impl<'a> FromStr for EnvFile<'a> {
  type Err = ParseError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut entries = Vec::new();
    let mut pending_comments = Vec::new();

    for line in s.lines() {
      let trimmed = line.trim();

      if trimmed.is_empty() {
        if !pending_comments.is_empty() {
          for comment in pending_comments.drain(..) {
            entries.push(EnvEntry::OrphanComment(comment));
          }
        }
        entries.push(EnvEntry::EmptyLine);
        continue;
      }

      if trimmed.starts_with('#') {
        pending_comments.push(Cow::Owned(trimmed.to_string()));
        continue;
      }

      if let Some(eq_pos) = line.find('=') {
        let key = line[..eq_pos].trim();
        let value_part = &line[eq_pos + 1..];

        let (value, inline_comment) = if let Some(hash_pos) = value_part.find('#') {
          let value = value_part[..hash_pos].trim();
          let comment = value_part[hash_pos..].trim();
          (value, Some(Cow::Owned(comment.to_string())))
        } else {
          (value_part.trim(), None)
        };

        let variable = EnvVariable {
          key: Cow::Owned(key.to_string()),
          value: Cow::Owned(value.to_string()),
          preceding_comments: std::mem::take(&mut pending_comments),
          inline_comment,
        };

        entries.push(EnvEntry::Variable(variable));
      } else {
        return Err(ParseError::InvalidLine(line.to_string()));
      }
    }

    for comment in pending_comments {
      entries.push(EnvEntry::OrphanComment(comment));
    }

    Ok(Self { entries })
  }
}

impl<'a> EnvFile<'a> {
  pub fn set(&mut self, key: &str, value: String) -> Option<Cow<'a, str>> {
    for entry in &mut self.entries {
      if let EnvEntry::Variable(var) = entry
        && var.key == key
      {
        let old_value = var.value.clone();
        var.value = Cow::Owned(value);
        return Some(old_value);
      }
    }

    self.entries.push(EnvEntry::Variable(EnvVariable {
      key: Cow::Owned(key.to_string()),
      value: Cow::Owned(value),
      preceding_comments: Vec::new(),
      inline_comment: None,
    }));

    None
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnvEntry<'a> {
  Variable(#[serde(borrow)] EnvVariable<'a>),
  OrphanComment(#[serde(borrow)] Cow<'a, str>),
  EmptyLine,
}

impl<'a> fmt::Display for EnvEntry<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      EnvEntry::Variable(var) => {
        for comment in &var.preceding_comments {
          writeln!(f, "{}", comment)?;
        }
        write!(f, "{}={}", var.key, var.value)?;
        if let Some(comment) = &var.inline_comment {
          write!(f, " {}", comment)?;
        }
        writeln!(f)
      }
      EnvEntry::OrphanComment(comment) => {
        writeln!(f, "{}", comment)
      }
      EnvEntry::EmptyLine => {
        writeln!(f)
      }
    }
  }
}

impl<'a> FromStr for EnvEntry<'a> {
  type Err = ParseError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let trimmed = s.trim();

    if trimmed.is_empty() {
      Ok(EnvEntry::EmptyLine)
    } else if trimmed.starts_with('#') {
      Ok(EnvEntry::OrphanComment(Cow::Owned(trimmed.to_string())))
    } else if let Some(eq_pos) = s.find('=') {
      let key = s[..eq_pos].trim();
      let value_part = &s[eq_pos + 1..];

      let (value, inline_comment) = if let Some(hash_pos) = value_part.find('#') {
        let value = value_part[..hash_pos].trim();
        let comment = value_part[hash_pos..].trim();
        (value, Some(Cow::Owned(comment.to_string())))
      } else {
        (value_part.trim(), None)
      };

      Ok(EnvEntry::Variable(EnvVariable {
        key: Cow::Owned(key.to_string()),
        value: Cow::Owned(value.to_string()),
        preceding_comments: Vec::new(),
        inline_comment,
      }))
    } else {
      Err(ParseError::InvalidLine(s.to_string()))
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvVariable<'a> {
  #[serde(borrow)]
  pub key: Cow<'a, str>,
  #[serde(borrow)]
  pub value: Cow<'a, str>,
  #[serde(borrow)]
  pub preceding_comments: Vec<Cow<'a, str>>,
  #[serde(borrow)]
  pub inline_comment: Option<Cow<'a, str>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
  #[error("Invalid line: {0}")]
  InvalidLine(String),
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_simple() {
    let input = "KEY=value\nANOTHER=test";
    let env: EnvFile = input.parse().unwrap();

    assert_eq!(env.entries.len(), 2);
    match &env.entries[0] {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "value");
      }
      _ => panic!("Expected variable"),
    }
    match &env.entries[1] {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "ANOTHER");
        assert_eq!(var.value, "test");
      }
      _ => panic!("Expected variable"),
    }
  }

  #[test]
  fn test_parse_with_comments() {
    let input = "# This is a comment\nKEY=value\n# Another comment\n# Multi line\nTEST=123";
    let env: EnvFile = input.parse().unwrap();

    let mut iter = env.entries.iter();

    // First entry should be KEY variable with one preceding comment
    match iter.next().unwrap() {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "value");
        assert_eq!(var.preceding_comments.len(), 1);
        assert_eq!(var.preceding_comments[0], "# This is a comment");
      }
      _ => panic!("Expected variable"),
    }

    // Second entry should be TEST variable with two preceding comments
    match iter.next().unwrap() {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "TEST");
        assert_eq!(var.value, "123");
        assert_eq!(var.preceding_comments.len(), 2);
        assert_eq!(var.preceding_comments[0], "# Another comment");
        assert_eq!(var.preceding_comments[1], "# Multi line");
      }
      _ => panic!("Expected variable"),
    }

    assert!(iter.next().is_none());
  }

  #[test]
  fn test_parse_inline_comments() {
    let input = "KEY=value # This is inline\nTEST=123";
    let env: EnvFile = input.parse().unwrap();

    match &env.entries[0] {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "value");
        assert_eq!(var.inline_comment, Some(Cow::Borrowed("# This is inline")));
      }
      _ => panic!("Expected variable"),
    }
  }

  #[test]
  fn test_roundtrip() {
    let input = "# Comment\nKEY=value\n\n# Orphan\nTEST=123 # inline";
    let env: EnvFile = input.parse().unwrap();
    let output = env.to_string();

    // Parse the output again and compare
    let env2: EnvFile = output.parse().unwrap();
    assert_eq!(env, env2);
  }

  #[test]
  fn test_env_entry_from_str() {
    // Test empty line
    let entry: EnvEntry = "".parse().unwrap();
    assert_eq!(entry, EnvEntry::EmptyLine);

    // Test comment
    let entry: EnvEntry = "# This is a comment".parse().unwrap();
    match entry {
      EnvEntry::OrphanComment(comment) => assert_eq!(comment, "# This is a comment"),
      _ => panic!("Expected OrphanComment"),
    }

    // Test variable
    let entry: EnvEntry = "KEY=value".parse().unwrap();
    match entry {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "value");
        assert!(var.preceding_comments.is_empty());
        assert!(var.inline_comment.is_none());
      }
      _ => panic!("Expected Variable"),
    }

    // Test variable with inline comment
    let entry: EnvEntry = "KEY=value # comment".parse().unwrap();
    match entry {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "value");
        assert_eq!(
          var.inline_comment,
          Some(Cow::Owned("# comment".to_string()))
        );
      }
      _ => panic!("Expected Variable"),
    }

    // Test invalid line
    assert!("invalid line without equals".parse::<EnvEntry>().is_err());
  }
}
