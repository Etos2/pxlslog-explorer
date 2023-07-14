use std::fmt::Display;

use nom::{
    branch::alt,
    bytes::complete::{take, take_while1},
    combinator::{map, verify},
    IResult,
};

use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseIdentifierError {
    Empty,
    InvalidLength(usize),
}

impl Display for ParseIdentifierError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseIdentifierError::Empty => write!(fmt, "unexpected end of string"),
            ParseIdentifierError::InvalidLength(n) => {
                write!(fmt, "invalid length for identifier: ({})", n)
            }
        }
    }
}

impl std::error::Error for ParseIdentifierError {}

#[derive(Debug, Clone, PartialEq)]
pub enum Identifier {
    Hash(String),
    Username(String),
}

impl Identifier {
    pub fn is_key(&self) -> bool {
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

    pub fn parse(input: &str) -> IResult<&str, Identifier, ErrorTree<&str>> {
        alt((
            map(
                verify(take_while1(|c: char| !c.is_whitespace()), |s: &str| {
                    s.chars().count() < 32
                }),
                |s: &str| Identifier::Username(s.into()),
            ),
            map(take(64usize), |s: &str| Identifier::Hash(s.into())),
        ))(input)
    }
}

impl<T> PartialEq<T> for Identifier where T: AsRef<str>  {
    fn eq(&self, other: &T) -> bool {
        self.get() == other.as_ref()
    }
}

impl TryFrom<&str> for Identifier {
    type Error = ErrorTree<Location>;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        final_parser(Self::parse)(input)
    }
}

impl ToString for Identifier {
    fn to_string(&self) -> String {
        self.get().to_string()
    }
}
