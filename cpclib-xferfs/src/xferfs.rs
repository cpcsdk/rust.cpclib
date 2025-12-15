use cpclib_xfer::CpcXfer;

use fuse_mt::*;
use std::path::Path;


pub struct XferFs {
	xfer: CpcXfer
}


impl XferFs {
	pub fn new<S:AsRef<str>>(hostname: S) -> Self {
		Self {
			xfer: CpcXfer::new(hostname)
		}
	}
}


impl fuse_mt::FilesystemMT for XferFs {

	fn opendir(&self, _req: RequestInfo, path: &Path, _flags: u32) -> ResultOpen {
		dbg!(path);
		unimplemented!()
	}


	fn readdir(&self, _req: RequestInfo, _path: &Path, _fh: u64) -> ResultReaddir {

		unimplemented!()
	}

}