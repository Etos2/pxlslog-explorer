use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
};

use clap::{ArgEnum, Args};
use rayon::{prelude::ParallelIterator, str::ParallelString};
use sha2::{Digest, Sha256};

use crate::{
    action::{ActionKind, ActionRef, Identifier, IdentifierRef},
    error::{ConfigError, ConfigResult, RuntimeError, RuntimeResult},
    palette::PaletteParser,
};

use super::{Command, CommandInput};

#[derive(Args)]
#[clap(
    about = "Render individual frames or output raw frame data to STDOUT.",
    long_about = "Render individual frames or output raw frame data to STDOUT.
Guaranted to produce 2 frames per render, where the first frame is the background and the last frame is the complete contents of the log.
To output only the final result, use the \"--screenshot\" arg or manually skip the first frame \"--skip\"."
)]
pub struct StatisticInput {
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of input log file")]
    #[clap(display_order = 0)]
    src: String,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of output data")]
    #[clap(long_help = "Filepath of output data [defaults to STDOUT]")]
    #[clap(display_order = 0)]
    dst: Option<String>,
    #[clap(short, long, arg_enum)]
    #[clap(value_name("ENUM"))]
    #[clap(help = "Type of data to generate")]
    mode: Option<Mode>,
    #[clap(short, long)]
    #[clap(value_name("ENUM"))]
    #[clap(help = "How to present the data")]
    plot: bool,
    #[clap(short, long)]
    #[clap(value_name("PATH"))]
    #[clap(help = "Filepath of palette")]
    #[clap(long_help = "Filepath of palette [possible types: .json, .txt, .gpl, .aco, .csv]")]
    #[clap(display_order = 0)]
    palette: Option<String>,
    #[clap(long)]
    #[clap(multiple_values(true))]
    #[clap(value_name("STRING"))]
    #[clap(help = "Only include entries that belong to this username/ hash")]
    user: Vec<String>,
}

#[derive(Debug, Copy, Clone, ArgEnum)]
enum Mode {
    All,
    Personal,
    Color,
    Canvas,
    Leaderboard,
}

enum Format {
    Terminal,
    CSV,
}

pub struct StatisticData {
    src: String,
    dst: Option<String>,
    mode: Mode,
    plot: bool,
    format: Format,
    palette: Vec<[u8; 4]>,
    users: Vec<Identifier>,
}

impl CommandInput<StatisticData> for StatisticInput {
    fn validate(&self) -> ConfigResult<StatisticData> {
        let palette = match &self.palette {
            Some(path) => PaletteParser::try_parse(&path)
                .map_err(|e| ConfigError::new("palette", &e.to_string()))?,
            None => super::render::DEFAULT_PALETTE.to_vec(),
        };

        let format = match &self.dst {
            Some(p) => {
                let path = PathBuf::from(p);
                match path.extension().map(|s| s.to_string_lossy()).as_deref() {
                    Some("csv") => Format::CSV,
                    Some(e) => Err(ConfigError::new(
                        "dst",
                        &format!("unsupported extension \'{}\'", e),
                    ))?,
                    None => Err(ConfigError::new("dst", "unsupported extension"))?,
                }
            }
            None => Format::Terminal,
        };

        let users: Vec<Identifier> = self
            .user
            .iter()
            .map(|u| {
                if u.len() == 512 {
                    Identifier::Hash(u.to_owned())
                } else {
                    Identifier::Username(u.to_owned())
                }
            })
            .collect();

        // Fail if missing essential info
        let mode = self.mode.unwrap_or(Mode::All);
        match mode {
            Mode::Personal => {
                if users.is_empty() {
                    Err(ConfigError::new(
                        "user",
                        "username or hash required for personal statistics",
                    ))?
                }
            }
            Mode::Leaderboard => {
                if users.iter().any(Identifier::is_username) {
                    Err(ConfigError::new(
                        "user",
                        "username required for leadboard statistics",
                    ))?
                }
            }
            _ => (),
        }

        Ok(StatisticData {
            src: self.src.to_owned(),
            dst: self.dst.to_owned(),
            mode,
            plot: self.plot,
            format,
            palette,
            users,
        })
    }
}

impl Command for StatisticData {
    fn run(&self, settings: &crate::Cli) -> RuntimeResult<()> {
        let data = std::fs::read_to_string(&self.src)
            .map_err(|e| RuntimeError::from_err(e, &self.src, 0))?;
        let actions: Vec<ActionRef> = data
            .as_parallel_string()
            .par_lines()
            .filter_map(|s| match ActionRef::try_from(s) {
                Ok(a) => Some(a),
                Err(_) => None, // TODO
            })
            .collect();

        let mut out: Box<dyn Write> = match &self.dst {
            Some(path) => Box::new(
                OpenOptions::new()
                    .create_new(settings.noclobber)
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(path)?,
            ),
            None => Box::new(std::io::stdout().lock()),
        };

        match self.mode {
            Mode::All => {
                for user in &self.users {
                    self.get_personal(&mut out, &actions, user.as_ref())?;
                    writeln!(out)?;
                }
                self.get_color(&mut out, &actions)?;
                writeln!(out)?;
                self.get_canvas(&mut out, &actions)?;
                writeln!(out)?;
                self.get_leaderboard(&mut out, &actions)?;
            }
            Mode::Personal => {
                for user in &self.users {
                    self.get_personal(&mut out, &actions, user.as_ref())?;
                    writeln!(out)?;
                }
            }
            Mode::Color => self.get_color(&mut out, &actions)?,
            Mode::Canvas => self.get_canvas(&mut out, &actions)?,
            Mode::Leaderboard => self.get_leaderboard(&mut out, &actions)?,
        };

        Ok(())
    }
}

impl StatisticData {
    fn get_personal(
        &self,
        out: &mut impl Write,
        actions: &[ActionRef],
        user: IdentifierRef,
    ) -> RuntimeResult<()> {
        let mut total = 0;
        let mut placed = 0;
        let mut survived = 0;
        let mut replaced = 0;
        let mut replaced_self = 0;
        let mut replaced_mod = 0;
        let mut restored_mod = 0;
        let mut undo = 0;

        let mut pixel_cache = HashSet::new();

        for action in actions {
            let is_equal = {
                match (&user, &action.user) {
                    (IdentifierRef::Hash(user_hash), IdentifierRef::Hash(random_hash)) => {
                        let time = action.time.format("%Y-%m-%d %H:%M:%S,%3f").to_string();
                        let mut hasher = Sha256::new();
                        hasher.update(time.as_bytes());
                        hasher.update(",");
                        hasher.update(action.x.to_string().as_bytes());
                        hasher.update(",");
                        hasher.update(action.y.to_string().as_bytes());
                        hasher.update(",");
                        hasher.update(action.index.to_string().as_bytes());
                        hasher.update(",");
                        hasher.update(user_hash.as_bytes());
                        let digest = hex::encode(hasher.finalize());
                        &digest[..] == *random_hash
                    }
                    (IdentifierRef::Username(user), IdentifierRef::Username(other)) => user == other,
                    _ => false,
                }
            };

            if is_equal {
                total += 1;
                match action.kind {
                    ActionKind::Place => {
                        placed += 1;
                        survived += 1;
                        if !pixel_cache.insert((action.x, action.y)) {
                            replaced_self += 1;
                        }
                    }
                    ActionKind::Undo => {
                        pixel_cache.remove(&(action.x, action.y));
                        undo += 1;
                    }
                    ActionKind::Overwrite => todo!(),
                    ActionKind::Rollback => todo!(),
                    ActionKind::RollbackUndo => todo!(),
                    ActionKind::Nuke => todo!(),
                }
            } else {
                match action.kind {
                    ActionKind::Place => {
                        if pixel_cache.remove(&(action.x, action.y)) {
                            replaced += 1;
                            survived -= 1;
                        }
                    }
                    ActionKind::Overwrite => {
                        if pixel_cache.get(&(action.x, action.y)).is_some() {
                            replaced_mod += 1;
                            survived -= 1;
                        }
                    }
                    _ => (),
                }
            }
        }

        let total_coverage = 100.0;
        let placed_coverage = placed as f64 / total as f64 * 100.0;
        let survived_coverage = survived as f64 / total as f64 * 100.0;
        let replaced_coverage = replaced as f64 / total as f64 * 100.0;
        let replaced_self_coverage = replaced_self as f64 / total as f64 * 100.0;
        let replaced_mod_coverage = replaced_mod as f64 / total as f64 * 100.0;
        let restored_mod_coverage = restored_mod as f64 / total as f64 * 100.0;
        let undo_coverage = undo as f64 / total as f64 * 100.0;

        #[rustfmt::skip]
        writeln!(out, "Total:            {:<6} ({:4.2}%)", total, total_coverage)?;
        #[rustfmt::skip]
        writeln!(out, "Placed:           {:<6} ({:4.2}%)", placed, placed_coverage)?;
        #[rustfmt::skip]
        writeln!(out, "Survived:         {:<6} ({:4.2}%)", survived, survived_coverage)?;
        #[rustfmt::skip]
        writeln!(out, "Replaced:         {:<6} ({:4.2}%)", replaced, replaced_coverage)?;
        #[rustfmt::skip]
        writeln!(out, "Replaced by self: {:<6} ({:4.2}%)", replaced_self, replaced_self_coverage)?;
        #[rustfmt::skip]
        writeln!(out, "Replaced by mods: {:<6} ({:4.2}%)", replaced_mod, replaced_mod_coverage)?;
        #[rustfmt::skip]
        writeln!(out, "Restored by mods: {:<6} ({:4.2}%)", restored_mod, restored_mod_coverage)?;
        #[rustfmt::skip]
        writeln!(out, "Undone:           {:<6} ({:4.2}%)", undo, undo_coverage)?;

        Ok(())
    }

    fn get_color(&self, out: &mut impl Write, actions: &[ActionRef]) -> RuntimeResult<()> {
        let mut used_colors = 0;
        let mut color_map = HashMap::<usize, usize>::new();

        for action in actions {
            match color_map.get_mut(&action.index) {
                Some(i) => *i += 1,
                None => {
                    color_map.insert(action.index, 1);
                    used_colors += 1;
                }
            };
        }

        let mut colors: Vec<(usize, usize)> = color_map.into_iter().map(|v| (v.1, v.0)).collect();
        colors.sort_by(|a, b| b.cmp(a));

        writeln!(out, "Total:  {}", used_colors)?;
        for (amount, index) in colors {
            let rgba = match self.palette.get(index) {
                Some(p) => p,
                None => &[0, 0, 0, 0],
            };
            writeln!(
                out,
                "Amount: {:<8} #{:0<2X}{:0<2X}{:0<2X}{:0<2X}  {}",
                amount, rgba[0], rgba[1], rgba[2], rgba[3], index
            )?;
        }

        Ok(())
    }

    fn get_canvas(&self, out: &mut impl Write, actions: &[ActionRef]) -> RuntimeResult<()> {
        let mut total_actions = 0;

        let mut total_place = 0;
        let mut total_undo = 0;
        let mut total_overwrite = 0;
        let mut total_rollback = 0;
        let mut total_rollback_undo = 0;
        let mut total_nuke = 0;

        for action in actions {
            total_actions += 1;

            match action.kind {
                crate::action::ActionKind::Place => total_place += 1,
                crate::action::ActionKind::Undo => total_undo += 1,
                crate::action::ActionKind::Overwrite => total_overwrite += 1,
                crate::action::ActionKind::Rollback => total_rollback += 1,
                crate::action::ActionKind::RollbackUndo => total_rollback_undo += 1,
                crate::action::ActionKind::Nuke => total_nuke += 1,
            }
        }

        let coverage_place = total_place as f64 / total_actions as f64 * 100.0;
        let coverage_undo = total_undo as f64 / total_actions as f64 * 100.0;
        let coverage_overwrite = total_overwrite as f64 / total_actions as f64 * 100.0;
        let coverage_rollback = total_rollback as f64 / total_actions as f64 * 100.0;
        let coverage_rollback_undo = total_rollback_undo as f64 / total_actions as f64 * 100.0;
        let coverage_nuke = total_nuke as f64 / total_actions as f64 * 100.0;

        writeln!(out, "Total actions:        {:<8}", total_actions)?;
        #[rustfmt::skip]
        writeln!(out, "Total placed:         {:<8} ({:4.2}%)", total_place, coverage_place)?;
        #[rustfmt::skip]
        writeln!(out, "Total undos:          {:<8} ({:4.2}%)", total_undo, coverage_undo)?;
        #[rustfmt::skip]
        writeln!(out, "Total overwritten:    {:<8} ({:4.2}%)", total_overwrite, coverage_overwrite)?;
        #[rustfmt::skip]
        writeln!(out, "Total rollback:       {:<8} ({:4.2}%)", total_rollback, coverage_rollback)?;
        #[rustfmt::skip]
        writeln!(out, "Total rollback undos: {:<8} ({:4.2}%)", total_rollback_undo, coverage_rollback_undo)?;
        #[rustfmt::skip]
        writeln!(out, "Total nuked:          {:<8} ({:4.2}%)", total_nuke, coverage_nuke)?;

        Ok(())
    }

    fn get_leaderboard(&self, out: &mut impl Write, actions: &[ActionRef]) -> RuntimeResult<()> {
        let mut users = HashMap::new();
        for action in actions {
            if let IdentifierRef::Username(user) = action.user {
                match users.get_mut(user) {
                    Some(i) => *i += 1,
                    None => {
                        users.insert(user, 1);
                    }
                };
            }
        }

        let mut pixel_counts: Vec<(&str, usize)> = users.into_iter().collect();
        pixel_counts.sort_by(|&a, &b| b.1.cmp(&a.1));

        writeln!(out, "Total users: {}", pixel_counts.len())?;
        for (i, (user, count)) in pixel_counts.into_iter().enumerate() {
            writeln!(out, "{:>4}: {:<8} {}", i, count, user)?;
        }

        Ok(())
    }
}
