use cpclib_common::itertools::Itertools;
use cpclib_cpr::{Cpr, CprInfo};
use colored::*;

pub enum Command {
	Info
}

fn diff_lines(first: &str, second: &str) -> String {
    first
        .lines()
        .zip(second.lines())
        .map(|(line1, line2)| {
            if line1 != line2 {
                format!("{}\t{}", line1, line2)
            }
            else {
                "...".to_string()
            }
        })
        .unique()
        .join("\n")
}

fn compare_lines(first: &str, second: &str) -> String {
    first
        .lines()
        .zip(second.lines())
        .map(|(line1, line2)| {
            if line1 != line2 {
                format!("NOK\t{}\t{}", line1, line2).red()
            }
            else {
                format!(" OK\t{}\t{}", line1, line2).blue()
            }
        })
        .join("\n")
}


impl Command {
	pub fn handle(&self, cpr: &mut Cpr, cpr2: Option<&mut Cpr>) {
		match self {
			Command::Info => self.handle_info(cpr, cpr2)
		}
	}

	fn handle_info(&self, cpr: &mut Cpr, cpr2: Option<&mut Cpr>) {
		let info = CprInfo::from(cpr as &Cpr);

		if let Some(cpr2) = cpr2 {
			let info2 = CprInfo::from(cpr2 as &Cpr);

			let info1 = info.to_string();
			let info2 = info2.to_string();
			let summary = compare_lines(&info1, &info2);
			println!("{}", summary);

		} else {
			println!("{}", info);
		}
	}
}