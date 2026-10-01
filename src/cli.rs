use crowsi_service_telemetry::{
    DiagnosticEventV1, ExpectedRuntime, RuntimeConfig, Signal, TelemetryError, TelemetryRuntime,
    TelemetryStore, contract_check as check_contract, generate_runtime_token,
};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

pub fn run(args: &[String]) -> Result<(), TelemetryError> {
    let Some(command) = args.first().map(String::as_str) else {
        return usage();
    };
    match command {
        "initialize" => initialize(&args[1..]),
        "token" => token(&args[1..]),
        "record" => record(&args[1..]),
        "project" | "query" => project(&args[1..]),
        "contract-check" => contract_check(&args[1..]),
        "serve" => serve(&args[1..]),
        _ => usage(),
    }
}

fn contract_check(args: &[String]) -> Result<(), TelemetryError> {
    if !args.is_empty() {
        return usage();
    }
    println!("{}", serde_json::to_string(&check_contract()?)?);
    Ok(())
}

fn initialize(args: &[String]) -> Result<(), TelemetryError> {
    let service = option(args, "--service")?;
    let db = absolute(option(args, "--db")?)?;
    let expected_runtime = match option(args, "--expected-runtime")? {
        "continuous" => ExpectedRuntime::Continuous,
        "on-demand" => ExpectedRuntime::OnDemand,
        "external" => ExpectedRuntime::External,
        _ => {
            return Err(TelemetryError::InvalidInput(
                "invalid expected runtime".into(),
            ));
        }
    };
    TelemetryStore::initialize(&db, service, expected_runtime)?;
    println!("initialized {service}");
    Ok(())
}

fn token(args: &[String]) -> Result<(), TelemetryError> {
    if args.first().map(String::as_str) != Some("generate") {
        return usage();
    }
    let output = absolute(option(&args[1..], "--output")?)?;
    generate_runtime_token(&output)?;
    println!("generated {}", output.display());
    Ok(())
}

fn record(args: &[String]) -> Result<(), TelemetryError> {
    let db = absolute(option(args, "--db")?)?;
    let input = absolute(option(args, "--input")?)?;
    let bytes = std::fs::read(input)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let signal = if value.get("signal").is_some() {
        serde_json::from_value::<Signal>(value)?
    } else {
        Signal::Event(serde_json::from_value::<DiagnosticEventV1>(value)?.try_into()?)
    };
    let revision = TelemetryStore::open(&db)?.record(signal)?;
    println!("{{\"accepted\":true,\"revision\":{revision}}}");
    Ok(())
}

fn project(args: &[String]) -> Result<(), TelemetryError> {
    let db = absolute(option(args, "--db")?)?;
    println!(
        "{}",
        serde_json::to_string(&TelemetryStore::open(&db)?.projection()?)?
    );
    Ok(())
}

fn serve(args: &[String]) -> Result<(), TelemetryError> {
    let bind = option(args, "--bind")?
        .parse::<SocketAddr>()
        .map_err(|_| TelemetryError::InvalidInput("invalid bind address".into()))?;
    let read_token_file = absolute(option(args, "--read-token-file")?)?;
    let ingest_token_file = absolute(option(args, "--ingest-token-file")?)?;
    let bindings = repeated(args, "--service-db")
        .into_iter()
        .map(parse_binding)
        .collect::<Result<Vec<_>, _>>()?;
    let config = RuntimeConfig {
        bind,
        service_databases: bindings,
        read_token_file,
        ingest_token_file,
    };
    TelemetryRuntime::load(&config)?.serve()
}

fn option<'a>(args: &'a [String], name: &str) -> Result<&'a str, TelemetryError> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| TelemetryError::InvalidInput(format!("missing {name}")))
}

fn repeated<'a>(args: &'a [String], name: &str) -> Vec<&'a str> {
    args.windows(2)
        .filter(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .collect()
}

fn parse_binding(value: &str) -> Result<(String, PathBuf), TelemetryError> {
    let (id, path) = value.split_once('=').ok_or_else(|| {
        TelemetryError::InvalidInput("service binding must be ID=ABSOLUTE_PATH".into())
    })?;
    Ok((id.into(), absolute(path)?))
}

fn absolute(value: &str) -> Result<PathBuf, TelemetryError> {
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(TelemetryError::InvalidPath(format!(
            "{value} is not absolute"
        )));
    }
    Ok(path.into())
}

fn usage<T>() -> Result<T, TelemetryError> {
    Err(TelemetryError::InvalidInput(
        "usage: initialize|token generate|record|project|query|contract-check|serve (see README)"
            .into(),
    ))
}
