use std::{sync::OnceLock};

use crate::delegated::{DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation};


pub const DISARK_CMD: &str = "disark";


#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum DisarkVersion {
	#[default]
	V1_0_0
}



impl StaticInformation for DisarkVersion {
	fn static_download_urls(&self) -> &'static crate::delegated::MutiplatformUrls {
		static URL: OnceLock<MutiplatformUrls> = OnceLock::new();

		URL.get_or_init(|| {
		MutiplatformUrls::builder()
			.linux("https://bitbucket.org/JulienNevo/disark/downloads/Disark-linux64-1.0.0.zip")
			.windows("https://bitbucket.org/JulienNevo/disark/downloads/Disark-windows-1.0.0.zip")
			.macos("https://bitbucket.org/JulienNevo/disark/downloads/Disark-macos-1.0.0.zip")
			.build()
		})
	}
}

impl DownloadableInformation for DisarkVersion {
	fn target_os_archive_format(&self) -> crate::delegated::ArchiveFormat {
		crate::delegated::ArchiveFormat::Zip
	}
}

impl ExecutableInformation for DisarkVersion {
	fn target_os_folder(&self) -> &'static str {
		"disark"
	}

	fn target_os_exec_fname(&self) -> &'static str {
		#[cfg(not(target_os="windows"))]
		return "disark";
		#[cfg(target_os="windows")]
		return "disark.exe"

	}
}

impl InternetStaticCompiledApplication for DisarkVersion {

}