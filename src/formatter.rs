use crate::database::DbExam;
use once_cell::sync::Lazy;
use regex::Regex;

// $() => Only when exam name is empty
// #() => Only when exam name is not empty

// $name => exam name (can only be used inside #())

// Not even gonna bother with escapes lol

pub const DEFAULT_FORMAT: &'static str = "$(Good luck with your exam!)#(Good luck with $name!)";

static NO_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\(([^)]*)\)").unwrap());
static NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\#\(([^)]*)\)").unwrap());
static NAME_REPLACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$name").unwrap());
static NEWLINE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\\n").unwrap());

pub fn format_exam<S: AsRef<str>>(format: S, exam: DbExam) -> String {
    let filtered_format = if exam.exam_name.is_empty() {
        let f = NO_NAME_REGEX.replace_all(format.as_ref(), "$1");
        NAME_REGEX.replace_all(f.as_ref(), "").to_string()
    } else {
        let f = NO_NAME_REGEX.replace_all(format.as_ref(), "");
        let f = NAME_REGEX.replace_all(f.as_ref(), "$1");
        NAME_REPLACE_REGEX
            .replace_all(&f, exam.exam_name)
            .to_string()
    };
    let filtered_format = NEWLINE
        .replace_all(filtered_format.as_ref(), "\n")
        .to_string();

    filtered_format
}
