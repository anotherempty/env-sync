use std::{borrow::Cow, fmt, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

const COMMENT_PREFIX: &str = "#";
const ASSIGNMENT_OPERATOR: &str = "=";

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

#[cfg(feature = "serde")]
impl<'a> Serialize for EnvFile<'a> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

#[cfg(feature = "serde")]
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
      let mut entry: EnvEntry = line.parse()?;

      if let EnvEntry::Variable(ref mut var) = entry {
        var.preceding_comments = std::mem::take(&mut pending_comments);
      } else if let EnvEntry::OrphanComment(comment) = entry {
        pending_comments.push(comment);
        continue;
      } else if matches!(entry, EnvEntry::EmptyLine) && !pending_comments.is_empty() {
        for comment in pending_comments.drain(..) {
          entries.push(EnvEntry::OrphanComment(comment));
        }
      }

      entries.push(entry);
    }

    for comment in pending_comments {
      entries.push(EnvEntry::OrphanComment(comment));
    }

    Ok(Self { entries })
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnvEntry<'a> {
  Variable(EnvVariable<'a>),
  OrphanComment(Cow<'a, str>),
  EmptyLine,
}

impl<'a> fmt::Display for EnvEntry<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      EnvEntry::Variable(var) => {
        write!(f, "{}", var)?;
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
    } else if trimmed.starts_with(COMMENT_PREFIX) {
      Ok(EnvEntry::OrphanComment(Cow::Owned(trimmed.to_string())))
    } else {
      Ok(EnvEntry::Variable(trimmed.parse()?))
    }
  }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for EnvEntry<'a> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

#[cfg(feature = "serde")]
impl<'de: 'a, 'a> Deserialize<'de> for EnvEntry<'a> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = <&'de str>::deserialize(deserializer)?;
    s.parse().map_err(de::Error::custom)
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvVariable<'a> {
  pub key: Cow<'a, str>,
  pub value: Cow<'a, str>,
  pub preceding_comments: Vec<Cow<'a, str>>,
  pub inline_comment: Option<Cow<'a, str>>,
}

impl<'a> fmt::Display for EnvVariable<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for comment in &self.preceding_comments {
      writeln!(f, "{}", comment)?;
    }
    write!(f, "{}{}{}", self.key, ASSIGNMENT_OPERATOR, self.value)?;
    if let Some(comment) = &self.inline_comment {
      write!(f, " {COMMENT_PREFIX}{}", comment)?;
    }
    Ok(())
  }
}

impl<'a> FromStr for EnvVariable<'a> {
  type Err = ParseError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if let Some((key, value_part)) = s.split_once(ASSIGNMENT_OPERATOR) {
      let key = key.trim();

      let (value, inline_comment) =
        if let Some((value, comment)) = value_part.split_once(COMMENT_PREFIX) {
          (value.trim(), Some(Cow::Owned(comment.to_string())))
        } else {
          (value_part.trim(), None)
        };

      Ok(EnvVariable {
        key: Cow::Owned(key.to_string()),
        value: Cow::Owned(value.to_string()),
        preceding_comments: Vec::new(),
        inline_comment,
      })
    } else {
      Err(ParseError::InvalidLine(s.to_string()))
    }
  }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for EnvVariable<'a> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

#[cfg(feature = "serde")]
impl<'de: 'a, 'a> Deserialize<'de> for EnvVariable<'a> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = <&'de str>::deserialize(deserializer)?;
    s.parse().map_err(de::Error::custom)
  }
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
        assert_eq!(var.inline_comment, Some(Cow::Borrowed(" This is inline")));
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
        assert_eq!(var.inline_comment, Some(Cow::Owned(" comment".to_string())));
      }
      _ => panic!("Expected Variable"),
    }

    // Test invalid line
    assert!("invalid line without equals".parse::<EnvEntry>().is_err());
  }
}
