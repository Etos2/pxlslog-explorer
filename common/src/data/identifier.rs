use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_while1},
    combinator::all_consuming,
    Finish, IResult, Parser,
};

use nom_locate::LocatedSpan;
use nom_supreme::error::ErrorTree;
use nom_supreme::parser_ext::ParserExt;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ParseIdentifierError {
    #[error("unexpected end of string")]
    Empty,
    #[error("invalid length for identifier: ({0})")]
    InvalidLength(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Identifier {
    Hash(String),
    Username(String),
    None,
}

impl Identifier {
    pub fn is_hashed(&self) -> bool {
        match self {
            Identifier::Hash(_) => true,
            Identifier::Username(_) => false,
            Identifier::None => false,
        }
    }

    pub fn is_username(&self) -> bool {
        match self {
            Identifier::Hash(_) => false,
            Identifier::Username(_) => true,
            Identifier::None => false,
        }
    }

    pub fn get(&self) -> Option<&str> {
        match self {
            Identifier::Hash(s) => Some(s),
            Identifier::Username(s) => Some(s),
            Identifier::None => None,
        }
    }

    pub(crate) fn parse(input: &str) -> IResult<&str, Identifier, ErrorTree<&str>> {
        alt((
            tag("dead").map(|_| Identifier::None),
            take_while1(|c: char| !c.is_whitespace())
                .verify(|s: &&str| s.chars().count() <= 32)
                .map(|s: &str| Identifier::Username(s.into())),
            take(64usize).map(|s: &str| Identifier::Hash(s.into())),
        ))(input)
    }
}

impl<T> PartialEq<T> for Identifier
where
    T: AsRef<str>,
{
    fn eq(&self, other: &T) -> bool {
        self.get().map(|id| id == other.as_ref()).unwrap_or(false)
    }
}

impl<'a> TryFrom<&'a str> for Identifier {
    type Error = ErrorTree<&'a str>;

    fn try_from(input: &'a str) -> Result<Self, Self::Error> {
        let span = LocatedSpan::new(input);
        let result = all_consuming(Self::parse)(&span).finish();
        match result {
            Ok((_, id)) => Ok(id),
            Err(e) => Err(e),
        }
    }
}

impl ToString for Identifier {
    fn to_string(&self) -> String {
        self.get().map(str::to_string).unwrap_or("null".to_string())
    }
}
