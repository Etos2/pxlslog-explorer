use nom::{branch::alt, bytes::complete::tag, Finish, IResult, combinator::{all_consuming, value}};

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
    pub(crate) fn parse(input: &str) -> IResult<&str, ActionKind> {
        alt((
            value(ActionKind::Place, tag("user place")),
            value(ActionKind::Undo, tag("user undo")),
            value(ActionKind::Overwrite, tag("mod overwrite")),
            value(ActionKind::RollbackUndo, tag("rollback undo")),
            value(ActionKind::Rollback, tag("rollback")),
            value(ActionKind::Nuke, tag("console nuke")),
        ))(input)
    }
}

impl<'a> TryFrom<&'a str> for ActionKind {
    type Error = nom::error::Error<&'a str>;

    fn try_from(input: &'a str) -> Result<Self, Self::Error> {
        let result = all_consuming(Self::parse)(input).finish();
        match result {
            Ok((_, kind)) => Ok(kind),
            Err(e) => Err(e),
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
