use std::thread;
use std::time::Duration;

#[ctor::ctor]
fn init() {
	thread::spawn(detector);
}

fn detector() {
	loop {
		thread::sleep(Duration::from_secs(1));

		let deadlocks = parking_lot::deadlock::check_deadlock();
		if deadlocks.is_empty() {
			continue;
		}

		eprintln!("DEADLOCK DETECTED");

		for (i, threads) in deadlocks.iter().enumerate() {
			eprintln!("Deadlock #{}", i);
			for t in threads {
				eprintln!("Thread Id {:#?}", t.thread_id());
				eprintln!("{:#?}", t.backtrace());
			}
		}

		std::process::abort();
	}
}
