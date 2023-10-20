use std::cmp::Reverse;
use std::fmt::Display;
use std::hash::Hash;
use std::io::Write;

use anyhow::Result;
use bitflags::bitflags;
use colored::Colorize;
use itertools::{iterate, Itertools};
use rustc_hash::FxHashMap;
use serde::Serialize;

use pyrogen_checker::checker::FixTable;
use pyrogen_checker::fs::relativize_path;
use pyrogen_checker::logging::LogLevel;
use pyrogen_checker::message::{Emitter, TextEmitter};
use pyrogen_checker::notify_user;
use pyrogen_checker::registry::{AsRule, Rule};
use pyrogen_checker::settings::flags;
use pyrogen_checker::settings::types::SerializationFormat;

use crate::diagnostics::Diagnostics;

bitflags! {
    #[derive(Default, Debug, Copy, Clone)]
    pub(crate) struct Flags: u8 {
        /// Whether to show violations when emitting diagnostics.
        const SHOW_VIOLATIONS = 0b0000_0001;
        /// Whether to show the source code when emitting diagnostics.
        const SHOW_SOURCE = 0b000_0010;
    }
}

#[derive(Serialize)]
struct ExpandedStatistics<'a> {
    code: SerializeRuleAsCode,
    message: &'a str,
    count: usize,
}

struct SerializeRuleAsCode(Rule);

impl Serialize for SerializeRuleAsCode {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.code().to_string())
    }
}

impl Display for SerializeRuleAsCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.code())
    }
}

impl From<Rule> for SerializeRuleAsCode {
    fn from(rule: Rule) -> Self {
        Self(rule)
    }
}

pub(crate) struct Printer {
    format: SerializationFormat,
    log_level: LogLevel,
    flags: Flags,
}

impl Printer {
    pub(crate) const fn new(
        format: SerializationFormat,
        log_level: LogLevel,
        flags: Flags,
    ) -> Self {
        Self {
            format,
            log_level,
            flags,
        }
    }

    pub(crate) fn write_to_user(&self, message: &str) {
        if self.log_level >= LogLevel::Default {
            notify_user!("{}", message);
        }
    }

    fn write_summary_text(&self, writer: &mut dyn Write, diagnostics: &Diagnostics) -> Result<()> {
        if self.log_level >= LogLevel::Default {
            if self.flags.intersects(Flags::SHOW_VIOLATIONS) {
                let fixed = diagnostics
                    .fixed
                    .values()
                    .flat_map(std::collections::HashMap::values)
                    .sum::<usize>();
                let remaining = diagnostics.messages.len();
                let total = fixed + remaining;
                if fixed > 0 {
                    let s = if total == 1 { "" } else { "s" };
                    writeln!(
                        writer,
                        "Found {total} error{s} ({fixed} fixed, {remaining} remaining)."
                    )?;
                } else if remaining > 0 {
                    let s = if remaining == 1 { "" } else { "s" };
                    writeln!(writer, "Found {remaining} error{s}.")?;
                }
            } else {
                let fixed = diagnostics
                    .fixed
                    .values()
                    .flat_map(std::collections::HashMap::values)
                    .sum::<usize>();
                if fixed > 0 {
                    let s = if fixed == 1 { "" } else { "s" };
                    if self.autofix_level.is_apply() {
                        writeln!(writer, "Fixed {fixed} error{s}.")?;
                    } else {
                        writeln!(writer, "Would fix {fixed} error{s}.")?;
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn write_once(
        &self,
        diagnostics: &Diagnostics,
        writer: &mut dyn Write,
    ) -> Result<()> {
        if matches!(self.log_level, LogLevel::Silent) {
            return Ok(());
        }

        if !self.flags.intersects(Flags::SHOW_VIOLATIONS) {
            if matches!(
                self.format,
                SerializationFormat::Text | SerializationFormat::Grouped
            ) {
                self.write_summary_text(writer, diagnostics)?;
            }
            return Ok(());
        }

        match self.format {
            SerializationFormat::Text => {
                TextEmitter::default()
                    .with_show_source(self.flags.intersects(Flags::SHOW_SOURCE))
                    .emit(writer, &diagnostics.messages)?;

                self.write_summary_text(writer, diagnostics)?;
            }
        }

        writer.flush()?;

        Ok(())
    }

    pub(crate) fn write_statistics(
        &self,
        diagnostics: &Diagnostics,
        writer: &mut dyn Write,
    ) -> Result<()> {
        let statistics: Vec<ExpandedStatistics> = diagnostics
            .messages
            .iter()
            .map(|message| (message.kind.rule(), &message.kind.body))
            .sorted()
            .fold(vec![], |mut acc, (rule, body)| {
                if let Some((prev_rule, _, _, count)) = acc.last_mut() {
                    if *prev_rule == rule {
                        *count += 1;
                        return acc;
                    }
                }
                acc.push((rule, body, 1));
                acc
            })
            .iter()
            .map(|(rule, message, fixable, count)| ExpandedStatistics {
                code: (*rule).into(),
                count: *count,
                message,
            })
            .sorted_by_key(|statistic| Reverse(statistic.count))
            .collect();

        if statistics.is_empty() {
            return Ok(());
        }

        match self.format {
            SerializationFormat::Text => {
                // Compute the maximum number of digits in the count and code, for all messages,
                // to enable pretty-printing.
                let count_width = num_digits(
                    statistics
                        .iter()
                        .map(|statistic| statistic.count)
                        .max()
                        .unwrap(),
                );
                let code_width = statistics
                    .iter()
                    .map(|statistic| statistic.code.to_string().len())
                    .max()
                    .unwrap();

                // By default, we mimic Flake8's `--statistics` format.
                for statistic in statistics {
                    writeln!(
                        writer,
                        "{:>count_width$}\t{:<code_width$}\t{}{}",
                        statistic.count.to_string().bold(),
                        statistic.code.to_string().red().bold(),
                        "",
                        statistic.message,
                    )?;
                }
                return Ok(());
            }
            // SerializationFormat::Json => {
            //     writeln!(writer, "{}", serde_json::to_string_pretty(&statistics)?)?;
            // }
            _ => {
                anyhow::bail!(
                    "Unsupported serialization format for statistics: {:?}",
                    self.format
                )
            }
        }

        writer.flush()?;

        Ok(())
    }

    pub(crate) fn write_continuously(
        &self,
        writer: &mut dyn Write,
        diagnostics: &Diagnostics,
    ) -> Result<()> {
        if matches!(self.log_level, LogLevel::Silent) {
            return Ok(());
        }

        if self.log_level >= LogLevel::Default {
            let s = if diagnostics.messages.len() == 1 {
                ""
            } else {
                "s"
            };
            notify_user!(
                "Found {} error{s}. Watching for file changes.",
                diagnostics.messages.len()
            );
        }

        if !diagnostics.messages.is_empty() {
            if self.log_level >= LogLevel::Default {
                writeln!(writer)?;
            }

            TextEmitter::default()
                .with_show_source(self.flags.intersects(Flags::SHOW_SOURCE))
                .emit(writer, &diagnostics.messages)?;
        }
        writer.flush()?;

        Ok(())
    }

    pub(crate) fn clear_screen() -> Result<()> {
        #[cfg(not(target_family = "wasm"))]
        clearscreen::clear()?;
        Ok(())
    }
}

fn num_digits(n: usize) -> usize {
    iterate(n, |&n| n / 10)
        .take_while(|&n| n > 0)
        .count()
        .max(1)
}

fn print_fix_summary(writer: &mut dyn Write, fixed: &FxHashMap<String, FixTable>) -> Result<()> {
    let total = fixed
        .values()
        .map(|table| table.values().sum::<usize>())
        .sum::<usize>();
    assert!(total > 0);
    let num_digits = num_digits(
        *fixed
            .values()
            .filter_map(|table| table.values().max())
            .max()
            .unwrap(),
    );

    let s = if total == 1 { "" } else { "s" };
    let label = format!("Fixed {total} error{s}:");
    writeln!(writer, "{}", label.bold().green())?;

    for (filename, table) in fixed
        .iter()
        .sorted_by_key(|(filename, ..)| filename.as_str())
    {
        writeln!(
            writer,
            "{} {}{}",
            "-".cyan(),
            relativize_path(filename).bold(),
            ":".cyan()
        )?;
        for (rule, count) in table.iter().sorted_by_key(|(.., count)| Reverse(*count)) {
            writeln!(
                writer,
                "    {count:>num_digits$} × {} ({})",
                rule.code().to_string().red().bold(),
                rule.as_ref(),
            )?;
        }
    }
    Ok(())
}
