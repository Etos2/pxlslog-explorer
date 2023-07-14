use anyhow::{Error, Context};
use nom::{branch::alt, bytes::complete::tag, IResult};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{Location, final_parser},
    ParserExt,
};

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
    pub(crate) fn parse(input: &str) -> IResult<&str, ActionKind, ErrorTree<&str>> {
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
