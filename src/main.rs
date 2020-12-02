use std::fmt::Formatter;
use std::fs::File;
use std::io::{self, BufRead};
use std::num::ParseIntError;
use std::path::Path;
use std::{env, error, fmt, num, result};

use regex::{self, Regex};

#[derive(fmt::Debug)]
struct Error {
    message: String,
}

impl Error {
    fn new(message: String) -> Error {
        Error { message }
    }
}

type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::new(format!("io error:{}", e))
    }
}

impl From<num::ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Error::new(format!("parse int error:{}", e))
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Self {
        Error::new(format!("regex error: {}", e))
    }
}

fn read_lines<P: AsRef<Path>>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>> {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

struct Parser {
    re: Regex,
}

impl Parser {
    fn new() -> Result<Parser> {
        let re = Regex::new(r"^(?P<from>\d+)-(?P<to>\d+)\s(?P<letter>\w):\s(?P<password>.+)$")?;
        Ok(Parser { re })
    }

    fn parse(&self, line: &str) -> Result<Record> {
        if let Some(caps) = self.re.captures(line) {
            let from = caps["from"].parse::<u64>()?;
            let to = caps["to"].parse::<u64>()?;
            let letter = caps["letter"].chars().collect::<Vec<char>>()[0];
            let password = caps["password"].to_string();
            Ok(Record {
                policy: Policy { from, to, letter },
                password,
            })
        } else {
            Err(Error::new("Invalid record".to_string()))
        }
    }
}

struct Policy {
    from: u64,
    to: u64,
    letter: char,
}

struct Record {
    policy: Policy,
    password: String,
}

impl Record {
    fn validate_old(&self) -> bool {
        let chars = self.password.chars();
        let count = chars.filter(|c| *c == self.policy.letter).count() as u64;
        count >= self.policy.from && count <= self.policy.to
    }

    fn validate_new(&self) -> bool {
        let char_vec: Vec<char> = self.password.chars().collect();
        (char_vec.len() as u64 >= self.policy.from
            && char_vec[(self.policy.from - 1) as usize] == self.policy.letter)
            != (char_vec.len() as u64 >= self.policy.to
                && char_vec[(self.policy.to - 1) as usize] == self.policy.letter)
    }
}

fn lines_to_records(lines: impl Iterator<Item=io::Result<String>>) -> Result<Vec<Record>> {
    let parser = Parser::new()?;
    Ok(lines
        .filter(|res| res.is_ok())
        .map(|res| res.unwrap()) // OK to unwrap here
        .map(|line| parser.parse(&line))
        .filter(|res| res.is_ok())
        .map(|res| res.unwrap()) // OK to unwrap here
        .collect())
}

fn count_old(recs: &Vec<Record>) -> u64 {
    recs.iter().filter(|rec| rec.validate_old()).count() as u64
}

fn count_new(recs: &Vec<Record>) -> u64 {
    recs.iter().filter(|rec| rec.validate_new()).count() as u64
}

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<String>>();
    if args.len() > 1 {
        let lines = read_lines(&args[1])?;
        let recs = lines_to_records(lines)?;

        println!(
            "The number of valid records by the old method is {}",
            count_old(&recs)
        );
        println!(
            "The number of valid records by the new method is {}",
            count_new(&recs)
        );
        Ok(())
    } else {
        Err(Error::new("filename argument required".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_db_record() -> result::Result<(), Error> {
        let record = Parser::new()?.parse("3-11 z: zzzzzdzzzzlzz")?;
        assert_eq!(3, record.policy.from);
        assert_eq!(11, record.policy.to);
        assert_eq!('z', record.policy.letter);
        assert_eq!("zzzzzdzzzzlzz", record.password);
        Ok(())
    }

    #[test]
    fn validates_a_valid_password() -> result::Result<(), Error> {
        let record = Parser::new()?.parse("1-3 a: abc")?;
        assert!(record.validate());
        Ok(())
    }

    #[test]
    fn does_not_validate_an_invalid_password() -> result::Result<(), Error> {
        let record = Parser::new()?.parse("1-3 a: aaaa")?;
        assert!(!record.validate());
        Ok(())
    }
}
