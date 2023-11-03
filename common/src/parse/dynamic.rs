use super::pxlslog::PxlsLogParser;

pub enum DynamicPixelParser {
    PxlsLog(PxlsLogParser),
}

// impl ActionParser for DynamicPixelParser {
//     type Err = Box<dyn Error>;

//     fn parse_line(
//         &mut self,
//         reader: impl Read,
//         flags: ActionParseFlags,
//     ) -> Result<Action, Self::Err> {
//         match self {
//             DynamicPixelParser::PxlsLog(p) => p.parse_line(reader, flags).map_err(|e| e.into()),
//         }
//     }
// }

// impl ActionsParser for DynamicPixelParser {
//     fn parse(
//         &mut self,
//         reader: impl Read + std::io::BufRead,
//         flags: ActionParseFlags,
//     ) -> ActionParseResult<Actions> {
//         match self {
//             DynamicPixelParser::PxlsLog(p) => p.parse(reader, flags),
//         }
//     }
// }
