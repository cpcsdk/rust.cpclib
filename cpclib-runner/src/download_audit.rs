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

#[allow(unused_imports)]
use crate::delegated::{
    GithubCompilableApplication, GithubCompiledApplication, InternetDynamicCompiledApplication,
    InternetStaticCompiledApplication
};
use crate::runner::assembler::uz80::Uz80Version;
use crate::runner::assembler::{RasmVersion, SjasmplusVersion, VasmVersion};
use crate::runner::ay::ayt::AytVersion;
use crate::runner::ay::fap::FAPVersion;
use crate::runner::ay::minimiser::MinimiserVersion;
use crate::runner::convgeneric::ConvGenericVersion;
use crate::runner::disassembler::disark::DisarkVersion;
use crate::runner::emulator::cadence::CadenceVersion;
use crate::runner::emulator::caprice_forever::CapriceForeverVersion;
use crate::runner::emulator::cpcemu::CpcEmuVersion;
use crate::runner::emulator::cpcemupower::CpcEmuPowerVersion;
use crate::runner::emulator::emulator1984::Emulator1984Version;
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

macro_rules! audit {
    ($($name: literal => $version: expr;)*) => {
        $(
            {
                let description = $version.configuration::<()>();
                match (description.download_fn_url)() {
                    Ok(url) => println!("{}\t{url}", $name),
                    Err(e) => println!("{}\tERROR {e}", $name)
                }
            }
        )*
    };
}

#[test]
#[ignore = "needs the network"]
fn download_audit() {
    audit! {
        "ace" => AceVersion::default();
        "amspirit" => AmspiritVersion::default();
        "cpcec" => CpcecVersion::default();
        "winape" => WinapeVersion::default();
        "cpcemu" => CpcEmuVersion::default();
        "retrovm" => RetroVmVersion::default();
        "sugarbox" => SugarBoxV2Version::default();
        "rasm" => RasmVersion::default();
        "sjasmplus" => SjasmplusVersion::default();
        "vasm" => VasmVersion::default();
        "uz80" => Uz80Version::default();
        "disark" => DisarkVersion::default();
        "at3" => At3Version::default();
        "chipnsfx" => ChipnsfxVersion::default();
        "ayt" => AytVersion::default();
        "minimiser" => MinimiserVersion::default();
        "fap" => FAPVersion::default();
        "amspiritlite" => AmspiritLiteVersion::default();
        "cadence" => CadenceVersion::default();
        "capriceforever" => CapriceForeverVersion::default();
        "cpcemupower" => CpcEmuPowerVersion::default();
        "emulator1984" => Emulator1984Version::default();
        "grafx2" => Grafx2Version::default();
        "hspcompiler" => HspCompilerVersion::default();
        "martine" => MartineVersion::default();
        "impdsk" => ImpDskVersion::default();
        "2cdt" => TwoCdtVersion::default();
        "convgeneric" => ConvGenericVersion::default();
    }
}
