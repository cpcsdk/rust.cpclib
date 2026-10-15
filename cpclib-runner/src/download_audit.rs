//! Which tools can still be downloaded? Every one of them is fetched from a
//! third party's site, and those change: this resolves the download address of
//! every tool the way a first run would (a web page to read, a GitHub release
//! to look up, a fixed address) and prints it, or why there is none.
//!
//! `cargo test -p cpclib-runner --lib download_audit -- --ignored --nocapture`
//! prints one `name<TAB>address` line per tool; whether the address really
//! answers is for the caller to check (`curl -I`). Ignored by default: it needs
//! the network.

#![cfg(test)]

use cpclib_common::network;

#[allow(unused_imports)]
use crate::delegated::{
    GithubCompilableApplication, GithubCompiledApplication, InternetDynamicCompiledApplication,
    InternetStaticCompiledApplication
};
use crate::runner::assembler::uz80::Uz80Version;
use crate::runner::assembler::{FantamsVersion, RasmVersion, SjasmplusVersion, VasmVersion};
use crate::runner::ay::ayt::AytVersion;
use crate::runner::ay::fap::FAPVersion;
use crate::runner::ay::minimiser::MinimiserVersion;
use crate::runner::convgeneric::ConvGenericVersion;
use crate::runner::disassembler::disark::DisarkVersion;
use crate::runner::emulator::cadence::CadenceVersion;
use crate::runner::emulator::caprice_forever::CapriceForeverVersion;
use crate::runner::emulator::cpcemu::CpcEmuVersion;
use crate::runner::emulator::cpcemupower::CpcEmuPowerVersion;
use crate::runner::emulator::cpcsyntaxerror::CpcSyntaxErrorVersion;
use crate::runner::emulator::emulator1984::Emulator1984Version;
use crate::runner::emulator::koncepcja::KoncepcjaVersion;
use crate::runner::emulator::retrovm::RetroVmVersion;
use crate::runner::emulator::{
    AceVersion, AmspiritLiteVersion, AmspiritVersion, CpcecVersion, SugarBoxV2Version,
    WinapeVersion
};
use crate::runner::grafx2::Grafx2Version;
use crate::runner::hspcompiler::HspCompilerVersion;
use crate::runner::impdisc::ImpDskVersion;
use crate::runner::martine::MartineVersion;
use crate::runner::tracker::at3::At3Version;
use crate::runner::tracker::chipnsfx::ChipnsfxVersion;
use crate::runner::twocdt::TwoCdtVersion;
use crate::runner::vlink::Vlink;

/// Tools that are not published for Linux, on purpose.
const NOT_FOR_THIS_OS: &[&str] = &["cpcemu"];

/// `(name, address or why there is none)` of every tool, the current version.
fn resolved_downloads() -> Vec<(&'static str, Result<String, String>)> {
    let mut all = Vec::new();
    macro_rules! audit {
        ($($name: literal => $version: expr;)*) => {
            $(
                {
                    let description = $version.configuration::<()>();
                    all.push(($name, (description.download_fn_url)()));
                }
            )*
        };
    }

    audit! {
        "ace" => AceVersion::default();
        "amspirit" => AmspiritVersion::default();
        "amspiritlite" => AmspiritLiteVersion::default();
        "cpcec" => CpcecVersion::default();
        "winape" => WinapeVersion::default();
        "cpcemu" => CpcEmuVersion::default();
        "cpcemupower" => CpcEmuPowerVersion::default();
        "capriceforever" => CapriceForeverVersion::default();
        "cadence" => CadenceVersion::default();
        "cpcsyntaxerror" => CpcSyntaxErrorVersion::default();
        "koncepcja" => KoncepcjaVersion::default();
        "retrovm" => RetroVmVersion::default();
        "sugarbox" => SugarBoxV2Version::default();
        "emulator1984" => Emulator1984Version::default();
        "fantams" => FantamsVersion::default();
        "rasm" => RasmVersion::default();
        "sjasmplus" => SjasmplusVersion::default();
        "vasm" => VasmVersion::default();
        "vlink" => Vlink;
        "uz80" => Uz80Version::default();
        "disark" => DisarkVersion::default();
        "at3" => At3Version::default();
        "chipnsfx" => ChipnsfxVersion::default();
        "ayt" => AytVersion::default();
        "minimiser" => MinimiserVersion::default();
        "fap" => FAPVersion::default();
        "grafx2" => Grafx2Version::default();
        "hspcompiler" => HspCompilerVersion::default();
        "martine" => MartineVersion::default();
        "impdsk" => ImpDskVersion::default();
        "2cdt" => TwoCdtVersion::default();
        "convgeneric" => ConvGenericVersion::default();
    }
    all
}

/// Resolves every tool's download address, then asks each address for its
/// headers (no body is read): fails, listing them, if one cannot be resolved
/// or does not answer. Run it when a tool stops installing, and regularly
/// (`.github/workflows/download-audit.yml` does) to learn of it before users do.
#[test]
#[ignore = "needs the network"]
fn download_audit() {
    let mut failures = Vec::new();
    for (name, url) in resolved_downloads() {
        match url {
            Ok(url) => {
                match network::download_response(&url) {
                    Ok(_) => println!("{name}\t{url}"),
                    Err(e) => {
                        println!("{name}\tDEAD {url} ({e})");
                        failures.push(format!("{name}: {url} - {e}"));
                    }
                }
            },
            Err(e) if NOT_FOR_THIS_OS.contains(&name) => println!("{name}\t(none: {e})"),
            Err(e) => {
                println!("{name}\tERROR {e}");
                failures.push(format!("{name}: no address - {e}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} tool(s) cannot be downloaded:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
