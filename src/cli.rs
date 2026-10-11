use crowsi_service_telemetry::{
    DiagnosticEventV1, ExpectedRuntime, RuntimeConfig, Signal, TelemetryError, TelemetryRuntime,
    TelemetryStore, contract_check as check_contract, generate_runtime_token,
};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

struct Options<'a> {
    values: BTreeMap<&'a str, Vec<&'a str>>,
}

impl<'a> Options<'a> {
    fn parse(args: &'a [String], allowed: &[&str]) -> Result<Self, TelemetryError> {
        let mut values = BTreeMap::<&str, Vec<&str>>::new();
        let mut pairs = args.chunks_exact(2);
        for pair in &mut pairs {
            let name = pair[0].as_str();
            if !allowed.contains(&name) || pair[1].starts_with("--") {
                return Err(TelemetryError::InvalidInput(
                    "invalid CLI option or missing value".into(),
                ));
            }
            let items = values.entry(name).or_default();
            if name != "--service-db" && !items.is_empty() {
                return Err(TelemetryError::InvalidInput(
                    "duplicate singleton CLI option".into(),
                ));
            }
            items.push(pair[1].as_str());
        }
        if !pairs.remainder().is_empty() || allowed.iter().any(|name| !values.contains_key(name)) {
            return Err(TelemetryError::InvalidInput(
                "missing CLI option or value".into(),
            ));
        }
        Ok(Self { values })
    }

    fn get(&self, name: &str) -> &str {
        self.values[name][0]
    }

    fn bindings(&self) -> Result<Vec<(String, PathBuf)>, TelemetryError> {
        let mut seen = std::collections::BTreeSet::new();
        self.values["--service-db"]
            .iter()
            .map(|value| {
                let binding = parse_binding(value)?;
                if !seen.insert(binding.0.clone()) {
                    return Err(TelemetryError::InvalidInput(
                        "duplicate service database binding".into(),
                    ));
                }
                Ok(binding)
            })
            .collect()
    }
}

pub fn run(args: &[String]) -> Result<(), TelemetryError> {
    let Some(command) = args.first().map(String::as_str) else {
        return usage();
    };
    match command {
        "initialize" => initialize(&Options::parse(
            &args[1..],
            &["--service", "--db", "--expected-runtime"],
        )?),
        "token" => token(&args[1..]),
        "record" => record(&Options::parse(&args[1..], &["--db", "--input"])?),
        "project" | "query" => project(&Options::parse(&args[1..], &["--db"])?),
        "contract-check" => contract_check(&args[1..]),
        "serve" => serve(&Options::parse(
            &args[1..],
            &[
                "--bind",
                "--read-token-file",
                "--ingest-token-file",
                "--service-db",
            ],
        )?),
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

fn initialize(args: &Options<'_>) -> Result<(), TelemetryError> {
    let service = args.get("--service");
    let db = absolute(args.get("--db"))?;
    let expected_runtime = match args.get("--expected-runtime") {
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
    let options = Options::parse(&args[1..], &["--output"])?;
    let output = absolute(options.get("--output"))?;
    generate_runtime_token(&output)?;
    println!("generated {}", output.display());
    Ok(())
}

fn record(args: &Options<'_>) -> Result<(), TelemetryError> {
    let db = absolute(args.get("--db"))?;
    let input = absolute(args.get("--input"))?;
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

fn project(args: &Options<'_>) -> Result<(), TelemetryError> {
    let db = absolute(args.get("--db"))?;
    println!(
        "{}",
        serde_json::to_string(&TelemetryStore::open(&db)?.projection()?)?
    );
    Ok(())
}

fn serve(args: &Options<'_>) -> Result<(), TelemetryError> {
    let bind = args
        .get("--bind")
        .parse::<SocketAddr>()
        .map_err(|_| TelemetryError::InvalidInput("invalid bind address".into()))?;
    let read_token_file = absolute(args.get("--read-token-file"))?;
    let ingest_token_file = absolute(args.get("--ingest-token-file"))?;
    let bindings = args.bindings()?;
    let config = RuntimeConfig {
        bind,
        service_databases: bindings,
        read_token_file,
        ingest_token_file,
    };
    TelemetryRuntime::load(&config)?.serve()
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
        return Err(TelemetryError::InvalidPath(
            "CLI path must be absolute".into(),
        ));
    }
    Ok(path.into())
}

fn usage<T>() -> Result<T, TelemetryError> {
    Err(TelemetryError::InvalidInput(
        "usage: initialize|token generate|record|project|query|contract-check|serve (see README)"
            .into(),
    ))
}
