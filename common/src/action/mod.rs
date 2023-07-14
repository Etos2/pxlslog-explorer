pub mod identifier;

use anyhow::{Context, Error, Result};
use chrono::NaiveDateTime;
use nom::{
    branch::alt,
    bytes::complete::take,
    character::complete::{self, multispace1},
    combinator::{map, map_res},
    IResult, Parser,
};
use nom_supreme::{error::ErrorTree, final_parser::Location, tag::complete::tag};
use nom_supreme::{final_parser::final_parser, ParserExt};

use self::identifier::Identifier;

pub const DATE_FMT: &str = "%Y-%m-%d %H:%M:%S,%3f";

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ActionKind {
    Place,
    Undo,
    Overwrite,
    Rollback,
    RollbackUndo,
    Nuke,
}

impl ActionKind {
    fn parse(input: &str) -> IResult<&str, ActionKind, ErrorTree<&str>> {
        alt((
            tag("user place").value(ActionKind::Place),
            tag("user undo").value(ActionKind::Undo),
            tag("mod overwrite").value(ActionKind::Overwrite),
            tag("rollback undo").value(ActionKind::RollbackUndo),
            tag("rollback").value(ActionKind::Rollback),
            tag("console nuke").value(ActionKind::Nuke),
        ))(input)
    }
}

impl TryFrom<&str> for ActionKind {
    type Error = Error;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        let result: Result<_, ErrorTree<Location>> = final_parser(Self::parse)(input);
        result
            .map_err(anyhow::Error::from)
            .context("Failed to parse action kind")
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

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub time: NaiveDateTime,
    pub user: Identifier,
    pub x: u32,
    pub y: u32,
    pub index: usize,
    pub kind: ActionKind,
}

impl Action {
    fn parse(input: &str) -> IResult<&str, Self, ErrorTree<&str>> {
        let (input, time) = map_res(take(23usize), |t| {
            NaiveDateTime::parse_from_str(t, DATE_FMT)
        })
        .context("date")
        .parse(input)?;

        let (input, _) = multispace1(input)?;
        let (input, user) = Identifier::parse(input)?;
        let (input, _) = multispace1(input)?;
        let (input, x) = complete::u32(input)?;
        let (input, _) = multispace1(input)?;
        let (input, y) = complete::u32(input)?;
        let (input, _) = multispace1(input)?;
        let (input, index) = map(complete::u32, |n| n as usize)(input)?;
        let (input, _) = multispace1(input)?;
        let (input, kind) = ActionKind::parse(input)?;

        Ok((
            input,
            Action {
                time,
                user,
                x,
                y,
                index,
                kind,
            },
        ))
    }
}

impl TryFrom<&str> for Action {
    type Error = ErrorTree<Location>;

    fn try_from(input: &str) -> Result<Self, ErrorTree<Location>> {
        final_parser(Self::parse)(input)
    }
}

impl ToString for Action {
    fn to_string(&self) -> String {
        let mut out = self.time.format(DATE_FMT).to_string();
        out += "\t";
        out += self.user.get();
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

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn identifier_try_from_err_empty() {
        assert!(Identifier::try_from("").is_err());
    }

    #[test]
    fn action_kind_to_string() {
        assert_eq!(ActionKind::Place.to_string(), "user place");
        assert_eq!(ActionKind::Undo.to_string(), "user undo");
        assert_eq!(ActionKind::Overwrite.to_string(), "mod overwrite");
        assert_eq!(ActionKind::Rollback.to_string(), "rollback");
        assert_eq!(ActionKind::RollbackUndo.to_string(), "rollback undo");
        assert_eq!(ActionKind::Nuke.to_string(), "console nuke");
    }

    #[test]
    fn action_kind_try_from() {
        assert_eq!(
            ActionKind::try_from("user place").unwrap(),
            ActionKind::Place
        );
        assert_eq!(ActionKind::try_from("user undo").unwrap(), ActionKind::Undo);
        assert_eq!(
            ActionKind::try_from("mod overwrite").unwrap(),
            ActionKind::Overwrite
        );
        assert_eq!(
            ActionKind::try_from("rollback").unwrap(),
            ActionKind::Rollback
        );
        assert_eq!(
            ActionKind::try_from("rollback undo").unwrap(),
            ActionKind::RollbackUndo
        );
        assert_eq!(
            ActionKind::try_from("console nuke").unwrap(),
            ActionKind::Nuke
        );
        assert!(ActionKind::try_from("other").is_err());
    }
}