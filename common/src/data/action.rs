use std::str::FromStr;

use chrono::NaiveDateTime;
use super::{actionkind::ActionKind, identifier::Identifier, DATE_FMT};

#[derive(Clone, Debug, PartialEq)]
pub enum Index {
    Color(usize),
    Transparent,
}

impl FromStr for Index {
    type Err = <usize as FromStr>::Err;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s == "-1" {
            Ok(Index::Transparent)
        } else {
            Ok(Index::Color(s.parse()?))
        }
    }
}

impl ToString for Index {
    fn to_string(&self) -> String {
        match self {
            Index::Color(n) => n.to_string(),
            Index::Transparent => "-1".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub time: i64,
    pub user: Option<Identifier>,
    pub x: u32,
    pub y: u32,
    pub index: Option<Index>,
    pub kind: Option<ActionKind>,
}

// impl Action {
//     fn parse(input: &str) -> IResult<&str, Self, ErrorTree<&str>> {
//         let (input, time) = map_res(take(23usize), |t| {
//             NaiveDateTime::parse_from_str(t, DATE_FMT).map(|t| t.timestamp_millis())
//         })
//         .context("date")
//         .parse(input)?;

//         let (input, _) = multispace1(input)?;
//         let (input, user) = Identifier::parse(input).unwrap();
//         let (input, _) = multispace1(input)?;
//         let (input, x) = complete::u32(input)?;
//         let (input, _) = multispace1(input)?;
//         let (input, y) = complete::u32(input)?;
//         let (input, _) = multispace1(input)?;
//         let (input, index) = map_res(take_while1(|c: char| !c.is_whitespace()), Index::from_str)
//             .context("index")
//             .parse(input)?;
//         let (input, _) = multispace1(input)?;
//         let (input, kind) = ActionKind::parse(input).unwrap();

//         Ok((
//             input,
//             Action {
//                 time,
//                 user,
//                 x,
//                 y,
//                 index,
//                 kind,
//             },
//         ))
//     }
// }

impl ToString for Action {
    fn to_string(&self) -> String {
        let mut out = NaiveDateTime::from_timestamp_millis(self.time)
            .unwrap() // Safety: Fails in the year 262000, not my problem
            .format(DATE_FMT)
            .to_string();
        out += "\t";
        out += self
            .user
            .as_ref()
            .unwrap_or(&Identifier::Username("Null".to_string()))
            .get();
        out += "\t";
        out += &self.x.to_string();
        out += "\t";
        out += &self.y.to_string();
        out += "\t";
        out += &self
            .index
            .as_ref()
            .unwrap_or(&Index::Transparent)
            .to_string();
        out += "\t";
        out += &self.kind.unwrap_or(ActionKind::Place).to_string();
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
