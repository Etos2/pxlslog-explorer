use std::{
    io::{BufRead, BufReader, Read},
    str::FromStr,
};

use bitflags::bitflags;
use chrono::NaiveDateTime;
use nom::{
    bytes::complete::{take, take_while1},
    character::complete::{self, multispace1},
    combinator::map_res,
    IResult, Parser,
};

use super::{
    action::Index,
    actionkind::ActionKind,
    error::{ActionParseError, ActionParseErrorKind},
    identifier::Identifier,
    DATE_FMT,
};

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ActionsParseFlags: u8 {
        const TIME = 0b00000001;
        const USER = 0b00000010;
        const INDEX = 0b00000100;
        const KIND = 0b00001000;
    }
}

pub struct ActionsIterator<'a> {
    actions: &'a Actions,
    i: usize,
}

impl<'a> Iterator for ActionsIterator<'a> {
    type Item = ActionsView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.actions.get_view(self.i);
        self.i += 1;
        out
    }
}

pub struct ActionsView<'a> {
    pub time: &'a NaiveDateTime,
    pub user: Option<&'a Identifier>,
    pub coord: (u32, u32),
    pub index: Option<Index>,
    pub kind: Option<ActionKind>,
}

// todo: Change to i64 rather than NaiveDateTime?
#[derive(Clone, Debug)]
pub struct Actions {
    pub time: Vec<NaiveDateTime>,
    pub user: Option<Vec<Identifier>>,
    pub coord: Vec<(u32, u32)>,
    pub index: Option<Vec<Index>>,
    pub kind: Option<Vec<ActionKind>>,
    pub bounds: (u32, u32, u32, u32),
}

#[derive(Clone, Debug)]
pub struct ActionsParser {
    time: Vec<NaiveDateTime>,
    user: Vec<Identifier>,
    coord: Vec<(u32, u32)>,
    index: Vec<Index>,
    kind: Vec<ActionKind>,
    flag: ActionsParseFlags,
    bounds: (u32, u32, u32, u32),
}

impl Actions {
    pub fn get_view(&self, i: usize) -> Option<ActionsView<'_>> {
        Some(ActionsView {
            time: self.time.get(i)?,
            user: self.user.as_ref().map(|v| v.get(i)).unwrap_or_default(),
            coord: self.coord.get(i).cloned()?,
            index: self.index.as_ref().map(|v| v.get(i)).unwrap_or_default().cloned(),
            kind: self.kind.as_ref().map(|v| v.get(i)).unwrap_or_default().cloned(),
        })
    }

    pub fn iter(&self) -> ActionsIterator {
        ActionsIterator {
            actions: self,
            i: 0,
        }
    }
}

impl ActionsParser {
    pub fn new(flag: ActionsParseFlags) -> ActionsParser {
        ActionsParser {
            time: Vec::new(),
            user: Vec::new(),
            coord: Vec::with_capacity(100000),
            index: Vec::new(),
            kind: Vec::new(),
            flag,
            bounds: (u32::MAX, u32::MAX, u32::MIN, u32::MIN),
        }
    }

    // TODO: Change into read_line
    pub fn read(&mut self, src: impl Read) -> Result<(), ActionParseError> {
        let mut reader = BufReader::new(src);
        let mut buffer = String::new();
        let mut l = 0;
        let mut c = 0;

        while reader
            .read_line(&mut buffer)
            .map_err(|e| ActionParseError::from(e).with_position(l, c))?
            != 0
        {
            buffer.pop();
            let (input, _) = Self::parse_line(self, &buffer[..], self.flag).unwrap(); // TODO: No unwrap
            if !input.is_empty() {
                Err(
                    ActionParseError::from(ActionParseErrorKind::ExpectedEndOfLine)
                        .with_position(l, c),
                )?
            }

            l += 1;
            buffer.clear();
        }

        self.bounds.2 += 1;
        self.bounds.3 += 1;

        Ok(())
    }

    fn parse_line<'a>(&mut self, input: &'a str, flag: ActionsParseFlags) -> IResult<&'a str, ()> {
        let (input, time) = map_res(take(23usize), |t| {
            NaiveDateTime::parse_from_str(t, DATE_FMT)
        })(input)?;
        let (input, _) = multispace1(input)?;
        let (input, user) = Self::conditional_parse(input, Identifier::parse, || {
            flag.intersects(ActionsParseFlags::USER)
        })?;
        let (input, _) = multispace1(input)?;
        let (input, x) = complete::u32(input)?;
        let (input, _) = multispace1(input)?;
        let (input, y) = complete::u32(input)?;
        let (input, _) = multispace1(input)?;
        let (input, index) = Self::conditional_parse(
            input,
            map_res(take_while1(|c: char| !c.is_whitespace()), Index::from_str),
            || flag.intersects(ActionsParseFlags::INDEX),
        )?;
        let (input, _) = multispace1(input)?;
        let (input, kind) = Self::conditional_parse(input, ActionKind::parse, || {
            flag.intersects(ActionsParseFlags::KIND)
        })?;

        if let Some(user) = user {
            self.user.push(user);
        }
        if let Some(index) = index {
            self.index.push(index);
        }
        if let Some(kind) = kind {
            self.kind.push(kind);
        }

        self.time.push(time);
        self.coord.push((x, y));
        self.bounds.0 = self.bounds.0.min(x);
        self.bounds.1 = self.bounds.1.min(y);
        self.bounds.2 = self.bounds.2.max(x);
        self.bounds.3 = self.bounds.3.max(y);

        Ok((input, ()))
    }

    // TODO: Parses regardless of condition result... change to read until next whiteline or provide cheaper alt parser
    // TODO: Opposite of multispace1?
    fn conditional_parse<'a, T, F>(
        input: &'a str,
        mut parse: impl Parser<&'a str, T, nom::error::Error<&'a str>>,
        cond: F,
    ) -> IResult<&'a str, Option<T>>
    where
        F: FnOnce() -> bool,
    {
        let (input, user) = parse.parse(input)?;
        if cond() {
            Ok((input, Some(user)))
        } else {
            Ok((input, None))
        }
    }

    pub fn build(self) -> Actions {
        Actions {
            time: self.time,
            user: (!self.user.is_empty()).then_some(self.user),
            coord: self.coord,
            index: (!self.index.is_empty()).then_some(self.index),
            kind: (!self.kind.is_empty()).then_some(self.kind),
            bounds: self.bounds,
        }
    }
}
