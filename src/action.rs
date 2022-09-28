use chrono::NaiveDateTime;
use clap::ArgEnum;

use crate::error::{RuntimeError, RuntimeErrorKind};

// TODO: Move ArgEnum into filter.rs?
#[derive(Debug, PartialEq, Copy, Clone, ArgEnum)]
pub enum ActionKind {
    Place,
    Undo,
    Overwrite,
    Rollback,
    RollbackUndo,
    Nuke,
}

impl<'a> TryFrom<&'a str> for ActionKind {
    type Error = RuntimeError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match s {
            "user place" => Ok(ActionKind::Place),
            "user undo" => Ok(ActionKind::Undo),
            "mod overwrite" => Ok(ActionKind::Overwrite),
            "rollback" => Ok(ActionKind::Rollback),
            "rollback undo" => Ok(ActionKind::RollbackUndo),
            "console nuke" => Ok(ActionKind::Nuke),
            _ => Err(RuntimeError::new(RuntimeErrorKind::BadToken(s.to_string()))),
        }
    }
}

impl ToString for ActionKind {
    fn to_string(&self) -> String {
        match self {
            ActionKind::Place => "user place",
            ActionKind::Undo => "user undo",
            ActionKind::Overwrite => "mod overwrite",
            ActionKind::Rollback => "rollback",
            ActionKind::RollbackUndo => "rollback undo",
            ActionKind::Nuke => "console nuke",
        }
        .to_string()
    }
}

#[derive(Debug, Clone)]
pub enum IdentifierRef<'a> {
    Hash(&'a str),
    Username(&'a str),
}

impl<'a> From<&'a str> for IdentifierRef<'a> {
    fn from(s: &'a str) -> Self {
        if s.len() == 64 {
            IdentifierRef::Hash(s)
        } else {
            IdentifierRef::Username(s)
        }
    }
}

impl<'a> ToString for IdentifierRef<'a> {
    fn to_string(&self) -> String {
        match self {
            IdentifierRef::Hash(s) => s.to_string(),
            IdentifierRef::Username(s) => s.to_string(),
        }
    }
}

impl<'a> IdentifierRef<'a> {
    pub fn is_hash(&self) -> bool {
        match self {
            IdentifierRef::Hash(_) => true,
            IdentifierRef::Username(_) => false,
        }
    }

    pub fn is_username(&self) -> bool {
        match self {
            IdentifierRef::Hash(_) => false,
            IdentifierRef::Username(_) => true,
        }
    }

    pub fn get(&self) -> &str {
        match self {
            IdentifierRef::Hash(s) => s,
            IdentifierRef::Username(s) => s,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Identifier {
    Hash(String),
    Username(String),
}

impl From<&str> for Identifier {
    fn from(s: &str) -> Self {
        if s.len() == 512 {
            Identifier::Hash(s.to_owned())
        } else {
            Identifier::Username(s.to_owned())
        }
    }
}

impl ToString for Identifier {
    fn to_string(&self) -> String {
        match self {
            Identifier::Hash(s) => s.to_string(),
            Identifier::Username(s) => s.to_string(),
        }
    }
}

impl Identifier {
    pub fn is_hash(&self) -> bool {
        match self {
            Identifier::Hash(_) => true,
            Identifier::Username(_) => false,
        }
    }

    pub fn is_username(&self) -> bool {
        match self {
            Identifier::Hash(_) => false,
            Identifier::Username(_) => true,
        }
    }

    pub fn get(&self) -> &str {
        match self {
            Identifier::Hash(s) => s,
            Identifier::Username(s) => s,
        }
    }

    pub fn as_ref(&self) -> IdentifierRef {
        match self {
            Identifier::Hash(s) => IdentifierRef::Hash(s),
            Identifier::Username(s) => IdentifierRef::Username(s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActionRef<'a> {
    pub time: NaiveDateTime,
    pub user: IdentifierRef<'a>,
    pub x: u32,
    pub y: u32,
    pub index: usize,
    pub kind: ActionKind,
}

// Todo: Remove
impl<'a> TryFrom<&'a str> for ActionRef<'a> {
    type Error = RuntimeError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let mut iter = s.split_terminator(|c| c == '\t');

        Ok(ActionRef {
            time: NaiveDateTime::parse_from_str(
                iter.next()
                    .ok_or(RuntimeError::new(RuntimeErrorKind::UnexpectedEof))?,
                "%Y-%m-%d %H:%M:%S,%3f",
            )?,
            user: IdentifierRef::from(
                iter.next()
                    .ok_or(RuntimeError::new(RuntimeErrorKind::UnexpectedEof))?,
            ),
            x: iter
                .next()
                .ok_or(RuntimeError::new(RuntimeErrorKind::UnexpectedEof))?
                .parse()?,
            y: iter
                .next()
                .ok_or(RuntimeError::new(RuntimeErrorKind::UnexpectedEof))?
                .parse()?,
            index: iter
                .next()
                .ok_or(RuntimeError::new(RuntimeErrorKind::UnexpectedEof))?
                .parse()?,
            kind: ActionKind::try_from(
                iter.next()
                    .ok_or(RuntimeError::new(RuntimeErrorKind::UnexpectedEof))?,
            )?,
        })
    }
}

impl<'a> ToString for ActionRef<'a> {
    fn to_string(&self) -> String {
        let mut out = self.time.format("%Y-%m-%d %H:%M:%S,%3f").to_string();
        out += "\t";
        out += &self.user.to_string();
        out += "\t";
        out += &self.x.to_string();
        out += "\t";
        out += &self.y.to_string();
        out += "\t";
        out += &self.index.to_string();
        out += "\t";
        out += &self.kind.to_string();
        out
    }
}
