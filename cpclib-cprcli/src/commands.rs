use cpclib_common::itertools::Itertools;
use cpclib_cpr::{CartridgeBank, Cpr, CprInfo};
use colored::*;
const DATA_WIDTH: usize = 16;

pub enum Command {
	Info,
	Dump
}

fn mem_to_string(bank: &CartridgeBank, from: Option<usize>, amount: Option<usize>) -> String {
    let from = from.unwrap_or(0);
    let amount = amount.unwrap_or_else(|| bank.data().len() - from);


    (from..(from + amount))
        .map(move |addr| bank.data()[addr])
        .chunks(DATA_WIDTH)
        .into_iter()
        .enumerate()
        .map(|(i, bytes)| {
            let bytes = bytes.collect_vec();
            let hex = bytes.iter().map(|byte| format!("{:02X}", byte)).join(" ");

            let addr = DATA_WIDTH * i + (from) as usize;

            let chars = bytes
                .iter()
                .map(|byte| {
                    char::from_u32(*byte as u32)
                        .map(|c| {
                            if !(' '..='~').contains(&c) {
                                '.'
                            }
                            else {
                                c
                            }
                        })
                        .unwrap_or('.')
                })
                .collect::<String>();

            format!("{:04X}: {:48}|{:16}|", addr, hex, chars)
        })
        .join("\n")
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
        .dedup()
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
			Command::Info => self.handle_info(cpr, cpr2),
			Command::Dump => self.handle_dump(cpr, cpr2)
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

	fn handle_dump(&self, cpr: &mut Cpr, cpr2: Option<&mut Cpr>) {
		for bank in cpr.banks() {
			println!("Bank {}", bank.code().as_str());

			let mem = mem_to_string(bank, None, None);
			if let Some(cpr2) = cpr2.as_ref() {
				let bank2 = cpr2.bank_by_code(bank.code()).expect(&format!("Bank {} unavailable in cpr2", bank.code()));
				let mem2 = mem_to_string(bank2, None, None);

				let summary = diff_lines(&mem, &mem2);
				println!("{}", summary);

			} else {
				println!("{}", mem);
			}
		}

	}
}