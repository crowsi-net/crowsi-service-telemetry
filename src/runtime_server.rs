use crate::TelemetryRuntime;
use crate::error::Result;
use std::net::TcpListener;

pub fn serve(runtime: &TelemetryRuntime) -> Result<()> {
    let listener = TcpListener::bind(runtime.bind())?;
    for incoming in listener.incoming() {
        let mut stream = incoming?;
        let Some(permit) = runtime.connection_permit() else {
            crate::runtime_handler::reject_capacity(&mut stream);
            continue;
        };
        let worker = runtime.clone();
        std::thread::spawn(move || {
            let _permit = permit;
            let _ = worker.handle(&mut stream);
        });
    }
    Ok(())
}
