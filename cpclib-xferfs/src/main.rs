pub mod xferfs;


struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        println!("{}: {}: {}", record.target(), record.level(), record.args());
    }

    fn flush(&self) {}
}

static LOGGER: ConsoleLogger = ConsoleLogger;

fn main()  {
	log::set_logger(&LOGGER).unwrap();
	log::set_max_level(log::LevelFilter::Debug);
	
	let hostname = "192.168.1.26";
	let mountpoint = "/tmp/xfer".to_owned();

	let fs = xferfs::XferFs::new(hostname);
	fuse_mt::mount(
		fuse_mt::FuseMT::new(fs, 1),
		&mountpoint, 
		&[]
	).unwrap();
}