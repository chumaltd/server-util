use log::{LevelFilter, Log, Metadata, Record};
use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, logs::SdkLoggerProvider};
use server_conf::{OtelConf, OtelLogLevel};

struct OtlpLogger {
    console: env_logger::Logger,
    otlp: Box<dyn Log>,
    otlp_level: LevelFilter,
}

impl Log for OtlpLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.otlp_level || self.console.enabled(metadata)
    }

    fn log(&self, record: &Record<'_>) {
        self.console.log(record);
        if record.level() <= self.otlp_level {
            self.otlp.log(record);
        }
    }

    fn flush(&self) {
        self.console.flush();
        self.otlp.flush();
    }
}

pub(super) fn init(service_name: &'static str, config: Option<&OtelConf>) -> Option<SdkLoggerProvider> {
    let mut console = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    );
    let Some(config) = config else {
        console.init();
        return None;
    };

    let exporter = match LogExporter::builder()
        .with_tonic()
        .with_endpoint(config.endpoint.clone())
        .build()
    {
        Ok(exporter) => exporter,
        Err(error) => {
            eprintln!("failed to build OTLP log exporter: {error}");
            console.init();
            return None;
        }
    };
    let provider = SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_service_name(service_name).build())
        .build();
    let console = console.build();
    let otlp_level = match config.log_level {
        OtelLogLevel::Error => LevelFilter::Error,
        OtelLogLevel::Warn => LevelFilter::Warn,
        OtelLogLevel::Info => LevelFilter::Info,
        OtelLogLevel::Debug => LevelFilter::Debug,
        OtelLogLevel::Trace => LevelFilter::Trace,
    };
    let max_level = console.filter().max(otlp_level);
    log::set_boxed_logger(Box::new(OtlpLogger {
        console,
        otlp: Box::new(OpenTelemetryLogBridge::new(&provider)),
        otlp_level,
    }))
    .expect("failed to initialize logger");
    log::set_max_level(max_level);
    Some(provider)
}

pub(super) async fn shutdown(provider: SdkLoggerProvider) {
    let result = tokio::task::spawn_blocking(move || provider.shutdown()).await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => eprintln!("failed to shut down OpenTelemetry logger provider: {error}"),
        Err(error) => eprintln!("OpenTelemetry logger shutdown task failed: {error}"),
    }
}
