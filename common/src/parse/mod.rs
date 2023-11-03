pub mod dynamic;
pub mod error;
pub mod pxlslog;

use std::{
    error::Error,
    io::{self, BufRead, Read},
};

use crate::data::{
    action::Action,
    actions::{Actions, ActionsBuilder},
};
use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ActionParseFlags: u8 {
        const TIME = 0b00000001;
        const USER = 0b00000010;
        const INDEX = 0b00000100;
        const KIND = 0b00001000;
    }
}

pub trait ActionParser {
    type Err: Error + Sync + Send;

    fn parse_line(&mut self, line: impl AsRef<str>) -> Result<Action, Self::Err>;

    fn parse_line_opt(
        &mut self,
        line: impl AsRef<str>,
        flags: ActionParseFlags,
    ) -> Result<Action, Self::Err>;
}

pub trait ActionsParser: ActionParser {
    fn parse(&mut self, mut reader: impl Read + BufRead) -> io::Result<Result<Actions, Self::Err>> {
        let mut actions = ActionsBuilder::new();
        let mut buffer = String::new();

        while reader.read_line(&mut buffer)? != 0 {
            let line = buffer
                .strip_suffix("\r\n")
                .or(buffer.strip_suffix('\n'))
                .unwrap_or(&buffer);
            match self.parse_line(line) {
                Ok(action) => {
                    actions.push(action);
                    buffer.clear();
                }
                Err(e) => return Ok(Err(e)),
            }
        }

        Ok(Ok(actions.build()))
    }

    fn parse_opt(
        &mut self,
        mut reader: impl Read + BufRead,
        flags: ActionParseFlags,
    ) -> io::Result<Result<Actions, Self::Err>> {
        let mut actions = ActionsBuilder::new();
        let mut buffer = String::new();

        while reader.read_line(&mut buffer)? != 0 {
            let line = buffer
                .strip_suffix("\r\n")
                .or(buffer.strip_suffix('\n'))
                .unwrap_or(&buffer);
            match self.parse_line_opt(line, flags) {
                Ok(action) => {
                    actions.push(action);
                    buffer.clear();
                }
                Err(e) => return Ok(Err(e)),
            }
        }

        Ok(Ok(actions.build()))
    }
}
