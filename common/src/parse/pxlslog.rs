use std::str::FromStr;

use chrono::{NaiveDateTime, ParseResult};
use nom::character::complete::{tab, u32};
use nom::{bytes::complete::take_while1, combinator::map_res, IResult, Parser};
use nom_supreme::error::ErrorTree;
use nom_supreme::final_parser::final_parser;
use nom_supreme::ParserExt;

use crate::data::actionkind::ActionKind;
use crate::data::identifier::Identifier;
use crate::data::DATE_FMT;

use crate::data::action::{Action, Index};

use super::{ActionParseFlags, ActionParser, ActionsParser};

pub struct PxlsLogParser;

impl ActionParser for PxlsLogParser {
    type Err = ErrorTree<String>;

    fn parse_line(&mut self, line: impl AsRef<str>) -> Result<Action, Self::Err> {
        parse_line_final(line.as_ref()).map_err(|e| e.map_locations(str::to_owned))
    }

    fn parse_line_opt(
        &mut self,
        line: impl AsRef<str>,
        flags: ActionParseFlags,
    ) -> Result<Action, Self::Err> {
        parse_line_opt_final(line.as_ref(), flags).map_err(|e| e.map_locations(str::to_owned))
    }
}

impl ActionsParser for PxlsLogParser {}

fn parse_line_final(input: &str) -> Result<Action, ErrorTree<&str>> {
    final_parser(parse_line)(input)
}

fn parse_line_opt_final(input: &str, flag: ActionParseFlags) -> Result<Action, ErrorTree<&str>> {
    final_parser(|i| parse_line_opt(i, flag))(input)
}

fn parse_line(input: &str) -> IResult<&str, Action, ErrorTree<&str>> {
    let (input, time) = parse_date(input)?;
    let (input, _) = tab(input)?;
    let (input, user) = Identifier::parse(input)?;
    let (input, _) = tab(input)?;
    let (input, x) = u32(input)?;
    let (input, _) = tab(input)?;
    let (input, y) = u32(input)?;
    let (input, _) = tab(input)?;
    let (input, index) = parse_index(input)?;
    let (input, _) = tab(input)?;
    let (input, kind) = ActionKind::parse(input)?;

    Ok((
        input,
        Action {
            time,
            user: Some(user),
            x,
            y,
            index: Some(index),
            kind: Some(kind),
        },
    ))
}

fn parse_line_opt(input: &str, flag: ActionParseFlags) -> IResult<&str, Action, ErrorTree<&str>> {
    let (input, time) = parse_date(input)?;
    let (input, _) = tab(input)?;
    let (input, user) = opt_parse(
        input,
        Identifier::parse,
        flag.intersects(ActionParseFlags::USER),
    )?;
    let (input, _) = tab(input)?;
    let (input, x) = u32(input)?;
    let (input, _) = tab(input)?;
    let (input, y) = u32(input)?;
    let (input, _) = tab(input)?;
    let (input, index) = opt_parse(input, parse_index, flag.intersects(ActionParseFlags::INDEX))?;
    let (input, _) = tab(input)?;
    let (input, kind) = opt_parse(
        input,
        ActionKind::parse,
        flag.intersects(ActionParseFlags::KIND),
    )?;

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

fn from_date(input: &str) -> ParseResult<i64> {
    NaiveDateTime::parse_from_str(input, DATE_FMT).map(|t| t.timestamp_millis())
}

fn parse_date(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    take_while1(|c| !(['\n', '\r', '\t'].contains(&c)))
        .map_res(from_date)
        .parse(input)
}

fn parse_index(input: &str) -> IResult<&str, Index, ErrorTree<&str>> {
    map_res(take_while1(|c: char| !c.is_whitespace()), Index::from_str)(input)
}

fn opt_parse<'a, T>(
    input: &'a str,
    mut parse: impl Parser<&'a str, T, ErrorTree<&'a str>>,
    cond: bool,
) -> IResult<&'a str, Option<T>, ErrorTree<&'a str>> {
    if cond {
        let (input, user) = parse.parse(input)?;
        Ok((input, Some(user)))
    } else {
        let (input, _) = take_while1(|c| !(['\n', '\r', '\t'].contains(&c)))(input)?;
        Ok((input, None))
    }
}
