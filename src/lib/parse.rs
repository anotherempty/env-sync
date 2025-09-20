use std::{borrow::Cow, convert::TryFrom, fmt};

#[cfg(feature = "tracing")]
use tracing::{debug, trace};

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

impl<'a> TryFrom<&'a str> for EnvFile<'a> {
  type Error = ParseError;

  fn try_from(s: &'a str) -> Result<Self, Self::Error> {
    #[cfg(feature = "tracing")]
    debug!("Parsing env file with {} lines", s.lines().count());

    let mut entries = Vec::new();
    let mut pending_comments = Vec::new();

    for line in s.lines() {
      #[cfg(feature = "tracing")]
      trace!("Parsing line: {:?}", line);

      let mut entry: EnvEntry = line.try_into()?;

      if let EnvEntry::Variable(ref mut var) = entry {
        #[cfg(feature = "tracing")]
        trace!(
          "Found variable: {} with {} pending comments",
          var.key,
          pending_comments.len()
        );

        var.preceding_comments = std::mem::take(&mut pending_comments);
      } else if let EnvEntry::OrphanComment(comment) = entry {
        #[cfg(feature = "tracing")]
        trace!("Found comment, adding to pending");

        pending_comments.push(comment);
        continue;
      } else if matches!(entry, EnvEntry::EmptyLine) && !pending_comments.is_empty() {
        #[cfg(feature = "tracing")]
        trace!(
          "Empty line with {} pending comments, flushing",
          pending_comments.len()
        );

        for comment in pending_comments.drain(..) {
          entries.push(EnvEntry::OrphanComment(comment));
        }
      }

      entries.push(entry);
    }

    for comment in pending_comments {
      entries.push(EnvEntry::OrphanComment(comment));
    }

    #[cfg(feature = "tracing")]
    debug!("Parsed {} entries", entries.len());

    Ok(Self { entries })
  }
}

impl<'a> EnvFile<'a> {
  pub fn get(&self, key: &str) -> Option<&EnvVariable<'a>> {
    self.entries.iter().find_map(|entry| {
      if let EnvEntry::Variable(var) = entry {
        if var.key == key { Some(var) } else { None }
      } else {
        None
      }
    })
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnvEntry<'a> {
  Variable(EnvVariable<'a>),
  OrphanComment(EnvComment<'a>),
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

impl<'a> TryFrom<&'a str> for EnvEntry<'a> {
  type Error = ParseError;

  fn try_from(s: &'a str) -> Result<Self, Self::Error> {
    let trimmed = s.trim();

    if trimmed.is_empty() {
      Ok(EnvEntry::EmptyLine)
    } else if trimmed.starts_with(COMMENT_PREFIX) {
      Ok(EnvEntry::OrphanComment(trimmed.try_into()?))
    } else {
      Ok(EnvEntry::Variable(trimmed.try_into()?))
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvVariable<'a> {
  pub key: Cow<'a, str>,
  pub value: Cow<'a, str>,
  pub preceding_comments: Vec<EnvComment<'a>>,
  pub inline_comment: Option<EnvComment<'a>>,
}

impl<'a> fmt::Display for EnvVariable<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for comment in &self.preceding_comments {
      writeln!(f, "{}", comment)?;
    }
    write!(f, "{}{}{}", self.key, ASSIGNMENT_OPERATOR, self.value)?;
    if let Some(comment) = &self.inline_comment {
      write!(f, " {}", comment)?;
    }
    Ok(())
  }
}

impl<'a> TryFrom<&'a str> for EnvVariable<'a> {
  type Error = ParseError;

  fn try_from(s: &'a str) -> Result<Self, Self::Error> {
    #[cfg(feature = "tracing")]
    trace!("Parsing variable from: {:?}", s);

    if let Some((key, value_part)) = s.split_once(ASSIGNMENT_OPERATOR) {
      let key = key.trim();

      let (value, inline_comment) =
        if let Some((value, comment)) = value_part.split_once(COMMENT_PREFIX) {
          (value.trim(), Some(EnvComment(Cow::Borrowed(comment))))
        } else {
          (value_part.trim(), None)
        };

      #[cfg(feature = "tracing")]
      trace!(
        "Parsed variable: key={}, value={}, has_inline_comment={}",
        key,
        value,
        inline_comment.is_some()
      );

      Ok(EnvVariable {
        key: Cow::Borrowed(key),
        value: Cow::Borrowed(value),
        preceding_comments: Vec::new(),
        inline_comment,
      })
    } else {
      Err(ParseError::InvalidLine(s.to_string()))
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvComment<'a>(Cow<'a, str>);

impl<'a> fmt::Display for EnvComment<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{}", COMMENT_PREFIX, self.0)
  }
}

impl<'a> TryFrom<&'a str> for EnvComment<'a> {
  type Error = ParseError;

  fn try_from(s: &'a str) -> Result<Self, Self::Error> {
    #[cfg(feature = "tracing")]
    trace!("Parsing comment from: {:?}", s);

    let trimmed = s.trim();
    if let Some(content) = trimmed.strip_prefix(COMMENT_PREFIX) {
      #[cfg(feature = "tracing")]
      trace!("Parsed comment content: {:?}", content);

      Ok(EnvComment(Cow::Borrowed(content)))
    } else {
      Err(ParseError::InvalidLine(s.to_string()))
    }
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
    let env: EnvFile = input.try_into().unwrap();

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
    let env: EnvFile = input.try_into().unwrap();

    let mut iter = env.entries.iter();

    // First entry should be KEY variable with one preceding comment
    match iter.next().unwrap() {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "value");
        assert_eq!(var.preceding_comments.len(), 1);
        assert_eq!(var.preceding_comments[0].to_string(), "# This is a comment");
      }
      _ => panic!("Expected variable"),
    }

    // Second entry should be TEST variable with two preceding comments
    match iter.next().unwrap() {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "TEST");
        assert_eq!(var.value, "123");
        assert_eq!(var.preceding_comments.len(), 2);
        assert_eq!(var.preceding_comments[0].to_string(), "# Another comment");
        assert_eq!(var.preceding_comments[1].to_string(), "# Multi line");
      }
      _ => panic!("Expected variable"),
    }

    assert!(iter.next().is_none());
  }

  #[test]
  fn test_parse_inline_comments() {
    let input = "KEY=value # This is inline\nTEST=123";
    let env: EnvFile = input.try_into().unwrap();

    match &env.entries[0] {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "value");
        assert_eq!(
          var.inline_comment,
          Some(EnvComment(Cow::Owned(" This is inline".to_string())))
        );
      }
      _ => panic!("Expected variable"),
    }
  }

  #[test]
  fn test_roundtrip() {
    let input = "# Comment\nKEY=value\n\n# Orphan\nTEST=123 # inline";
    let env: EnvFile = input.try_into().unwrap();
    let output = env.to_string();

    // Parse the output again and compare
    let env2: EnvFile = output.as_str().try_into().unwrap();
    assert_eq!(env, env2);
  }

  #[test]
  fn test_env_entry_from_str() {
    // Test empty line
    let entry: EnvEntry = "".try_into().unwrap();
    assert_eq!(entry, EnvEntry::EmptyLine);

    // Test comment
    let entry: EnvEntry = "# This is a comment".try_into().unwrap();
    match entry {
      EnvEntry::OrphanComment(comment) => assert_eq!(
        comment,
        EnvComment(Cow::Owned(" This is a comment".to_string()))
      ),
      _ => panic!("Expected OrphanComment"),
    }

    // Test variable
    let entry: EnvEntry = "KEY=value".try_into().unwrap();
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
    let entry: EnvEntry = "KEY=value # comment".try_into().unwrap();
    match entry {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "value");
        assert_eq!(
          var.inline_comment,
          Some(EnvComment(Cow::Owned(" comment".to_string())))
        );
      }
      _ => panic!("Expected Variable"),
    }

    // Test invalid line
    assert!(EnvEntry::try_from("invalid line without equals").is_err());
  }

  #[test]
  fn test_key_without_value() {
    // Test key with equals but no value
    let entry: EnvEntry = "KEY=".try_into().unwrap();
    match entry {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "");
        assert!(var.inline_comment.is_none());
      }
      _ => panic!("Expected Variable"),
    }

    // Test key with equals and whitespace
    let entry: EnvEntry = "KEY=   ".try_into().unwrap();
    match entry {
      EnvEntry::Variable(var) => {
        assert_eq!(var.key, "KEY");
        assert_eq!(var.value, "");
        assert!(var.inline_comment.is_none());
      }
      _ => panic!("Expected Variable"),
    }
  }
}
