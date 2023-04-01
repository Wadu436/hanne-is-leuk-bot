use std::{error::Error, fmt::Display};

use chrono::{Datelike, NaiveDate};
use once_cell::sync::Lazy;
use regex::Regex;

static EXAM_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:(\d{1,2}) (\w*)|(\d{1,2})[/-](\d{1,2}))[:- ]+(?:\d{1,2}[hu]\d{0,2})?[ ]*(.*)")
        .unwrap()
});

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ParseExam {
    pub day: NaiveDate,
    pub name: String,
}

impl Display for ParseExam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.day, self.name)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum ErrorMark {
    #[default]
    None,
    Squiggly {
        start: usize,
        end: usize,
    },
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum ErrorType {
    Warning,
    #[default]
    Error,
}

#[derive(Clone, Debug)]
pub struct ParseError {
    ty: ErrorType,
    line: usize,
    column: usize,
    message: String,
    part: String,
    mark: ErrorMark,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "**{} while parsing: {}**\n -> line {}, column {}",
            match self.ty {
                ErrorType::Warning => "warning",
                ErrorType::Error => "error",
            },
            self.message,
            self.line + 1,
            self.column + 1
        )?;

        let mut message = format!("{}", self.part);
        match self.mark {
            ErrorMark::Squiggly { start, end } => {
                if end > start {
                    message += "\n";
                    message += format!("{}{}", " ".repeat(start), "^".repeat(end - start)).as_str();
                }
            }
            _ => {}
        }

        if message.len() > 0 {
            write!(f, "```\n{}\n```", message)?;
        }

        Ok(())
    }
}

impl Error for ParseError {}

struct Token {
    text: String,
    line_nr: usize,
    column_nr: usize,
}

// Returns
pub fn parse(schedule: &str) -> Result<(Vec<ParseExam>, Vec<ParseError>), crate::Error> {
    let mut exams: Vec<_> = Vec::new();
    let mut warnings: Vec<_> = Vec::new();

    // Split the schedule into seperate exams
    let schedule: Vec<_> = if schedule.contains("\n") {
        // schedule has newlines, split on those
        schedule
            .split("\n")
            .enumerate()
            .map(|(i, s)| {
                let trimmed_clean = s.replace("```", "");
                let trimmed_start = trimmed_clean.trim_start();
                let trimmed = trimmed_start.trim_end();
                let trim_offset = s.len() - trimmed_start.len();
                Token {
                    text: trimmed.to_owned(),
                    line_nr: i,
                    column_nr: trim_offset,
                }
            })
            .filter(|token| token.text.len() > 0)
            .collect()
    } else {
        // schedule has commas, split on those
        // schedule.split(",").enumerate().map(|(i, s)| Token {text: s, line: i, column: 0})
        let mut column = 0;
        schedule
            .split(",")
            .map(|s| {
                // Remove backticks (they mess with our own messages)
                let trimmed_clean = s.replace("`", "");
                let trimmed_start = trimmed_clean.trim_start();
                let trimmed = trimmed_start.trim_end();
                let trim_offset = s.len() - trimmed_start.len();
                let tok = Token {
                    text: trimmed.to_owned(),
                    line_nr: 0,
                    column_nr: column + trim_offset,
                };
                column += s.len();
                tok
            })
            .filter(|token| token.text.len() > 0)
            .collect()
    };

    let now = chrono::offset::Utc::now().date_naive();
    let current_year = now.year();
    let current_month = now.month();
    let current_day = now.day();

    for exam in schedule {
        if let Some(captures) = EXAM_REGEX.captures(exam.text.as_str()) {
            println!("{:?}", captures);

            let day_capture = captures.get(1).or(captures.get(3)).unwrap();
            let day: u32 = day_capture.as_str().parse()?;

            let date_index_start = day_capture.start();

            let (month, date_index_end): (u32, _) = {
                if let Some(month_text) = captures.get(2) {
                    (
                        match month_text.as_str().to_lowercase().as_str() {
                            "jan" | "januari" | "january" => 1,
                            "feb" | "februari" | "february" => 2,
                            "mar" | "maart" | "march" => 3,
                            "apr" | "april" => 4,
                            "mei" | "may" => 5,
                            "jun" | "juni" | "june" => 6,
                            "jul" | "juli" | "july" => 7,
                            "aug" | "augustus" => 8,
                            "sep" | "september" => 9,
                            "oct" | "okt" | "october" | "oktober" => 10,
                            "nov" | "november" => 11,
                            "dec" | "december" => 12,
                            _ => {
                                warnings.push(ParseError {
                                    ty: ErrorType::Error,
                                    line: exam.line_nr,
                                    column: exam.column_nr + month_text.start(),
                                    message: "Could not parse month.".to_owned(),
                                    part: exam.text.to_owned(),
                                    mark: ErrorMark::Squiggly {
                                        start: month_text.start(),
                                        end: month_text.end(),
                                    },
                                });
                                continue;
                            }
                        },
                        month_text.end(),
                    )
                } else {
                    let month_capture = captures.get(4).unwrap();
                    let month = month_capture.as_str().parse()?;
                    if !(1..=12).contains(&month) {
                        warnings.push(ParseError {
                            ty: ErrorType::Error,
                            line: exam.line_nr,
                            column: exam.column_nr + month_capture.start(),
                            message: "Invalid month.".to_owned(),
                            part: exam.text.to_owned(),
                            mark: ErrorMark::Squiggly {
                                start: month_capture.start(),
                                end: month_capture.end(),
                            },
                        });
                        continue;
                    }
                    (month, month_capture.end())
                }
            };

            // Ensure the date is in the future :)
            let year = if month > current_month {
                current_year
            } else if month == current_month {
                if day >= current_day {
                    current_year
                } else {
                    current_year + 1
                }
            } else {
                current_year + 1
            };

            let exam_date = if let Some(exam_date) = NaiveDate::from_ymd_opt(year, month, day) {
                exam_date
            } else {
                warnings.push(ParseError {
                    ty: ErrorType::Error,
                    line: exam.line_nr,
                    column: exam.column_nr + date_index_start,
                    message: "Invalid date".to_owned(),
                    part: exam.text.to_owned(),
                    mark: ErrorMark::Squiggly {
                        start: date_index_start,
                        end: date_index_end,
                    },
                });
                continue;
            };
            let exam_name = captures.get(5).unwrap().as_str();

            exams.push(ParseExam {
                day: exam_date,
                name: exam_name.to_owned(),
            });
        } else {
            warnings.push(ParseError {
                ty: ErrorType::Warning,
                line: exam.line_nr,
                column: exam.column_nr,
                message: "Could not match an exam.".to_owned(),
                part: exam.text.replace('`', ""),
                mark: ErrorMark::None,
            });
        }
    }

    return Ok((exams, warnings));
}
