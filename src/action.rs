use chrono::NaiveDateTime;
use clap::ArgEnum;

use crate::error::{ParseError, ParseErrorKind};

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
    type Error = ParseError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match s {
            "user place" => Ok(ActionKind::Place),
            "user undo" => Ok(ActionKind::Undo),
            "mod overwrite" => Ok(ActionKind::Overwrite),
            "rollback" => Ok(ActionKind::Rollback),
            "rollback undo" => Ok(ActionKind::RollbackUndo),
            "console nuke" => Ok(ActionKind::Nuke),
            _ => Err(ParseError::new(ParseErrorKind::BadToken(s.to_string()))),
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
pub enum Identifier<'a> {
    Hash(&'a str),
    Username(&'a str),
}

impl<'a> From<&'a str> for Identifier<'a> {
    fn from(s: &'a str) -> Self {
        if s.len() == 512 {
            Identifier::Hash(s)
        } else {
            Identifier::Username(s)
        }
    }
}

impl<'a> ToString for Identifier<'a> {
    fn to_string(&self) -> String {
        match self {
            Identifier::Hash(s) => s.to_string(),
            Identifier::Username(s) => s.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActionRef<'a> {
    pub time: NaiveDateTime,
    pub user: Identifier<'a>,
    pub x: u32,
    pub y: u32,
    pub index: usize,
    pub kind: ActionKind,
}

// Todo: Remove
impl<'a> TryFrom<&'a str> for ActionRef<'a> {
    type Error = ParseError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let mut iter = s.split_terminator(|c| c == '\t');

        Ok(ActionRef {
            time: NaiveDateTime::parse_from_str(
                iter.next()
                    .ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?,
                "%Y-%m-%d %H:%M:%S,%3f",
            )?,
            user: Identifier::from(
                iter.next()
                    .ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?,
            ),
            x: iter
                .next()
                .ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?
                .parse()?,
            y: iter
                .next()
                .ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?
                .parse()?,
            index: iter
                .next()
                .ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?
                .parse()?,
            kind: ActionKind::try_from(
                iter.next()
                    .ok_or(ParseError::new(ParseErrorKind::UnexpectedEof))?,
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
