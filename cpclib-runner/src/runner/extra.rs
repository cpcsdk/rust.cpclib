
///! Rely on the installation of another tool, to use this one.
///! WIP, does not stil lwork

use std::{fmt::Debug, hash::Hash};

use crate::delegated::{DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication, StaticInformation};
use bon::Builder;

/// An extra tool is already installed with another one
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[derive(Builder)]
pub struct ExtraTool<BaseTool> 
where 
	BaseTool: Clone + Debug + PartialEq + Eq + Hash
{
	tool: BaseTool,
	target_os_exec_fname: Option<&'static str>,
	target_os_folder: Option<&'static str>
}

impl<BaseTool> ExtraTool<BaseTool>
where 
	BaseTool: Clone + Debug + PartialEq + Eq + Hash + Default {

	pub fn owner(&self) -> &BaseTool {
		&self.tool
	}
}
/*
impl<BaseTool> Default for ExtraTool<BaseTool>
where 
	BaseTool: Clone + Debug + PartialEq + Eq + Hash + Default {

		fn default() -> Self {
			Self{tool: Default::default()}
		}
}
*/


impl<BaseTool: StaticInformation>  StaticInformation for ExtraTool<BaseTool> where 
BaseTool: Clone + Debug + PartialEq + Eq + Hash{
	fn static_download_urls(&self) -> &'static crate::delegated::MutiplatformUrls {
		self.tool.static_download_urls()		
	}

	fn target_os_url(&self) -> Option<&'static str> {
		self.tool.target_os_url()
	}

	fn target_os_url_generator(&self) -> crate::delegated::UrlGenerator {
		self.tool.target_os_url_generator()
	}
}

impl<BaseTool: DownloadableInformation>  DownloadableInformation for ExtraTool<BaseTool> where 
BaseTool: Clone + Debug + PartialEq + Eq + Hash {
	fn target_os_archive_format(&self) -> crate::delegated::ArchiveFormat {
		self.tool.target_os_archive_format()
	}

	fn target_os_postinstall<E: cpclib_common::event::EventObserver>(&self) -> Option<crate::delegated::PostInstall<E>> where 
	BaseTool: Clone + Debug + PartialEq + Eq + Hash{
		self.tool.target_os_postinstall()
	}
}


impl<BaseTool: ExecutableInformation>  ExecutableInformation for ExtraTool<BaseTool> where 
BaseTool: Clone + Debug + PartialEq + Eq + Hash {
	fn target_os_exec_fname(&self) -> &'static str {
		if let Some(fname) = &self.target_os_exec_fname {
			fname
		} else {
			self.tool.target_os_exec_fname()
		}
	}

	fn target_os_folder(&self) -> &'static str {
		if let Some(folder) = &self.target_os_folder {
			folder
		} else {
			self.tool.target_os_folder()
		}
	}

	fn target_os_run_in_dir(&self) -> super::runner::RunInDir {
		self.tool.target_os_run_in_dir()
	}
}

impl<BaseTool:InternetStaticCompiledApplication> InternetStaticCompiledApplication for ExtraTool<BaseTool> where 
BaseTool: Clone + Debug + PartialEq + Eq + Hash {

}