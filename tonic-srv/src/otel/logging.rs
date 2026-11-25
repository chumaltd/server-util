use log::{Level, LevelFilter, Log, Metadata, Record};
use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, logs::SdkLoggerProvider};

struct ErrorExportLogger {
    console: env_logger::Logger,
    otlp: Box<dyn Log>,
}

impl Log for ErrorExportLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() == Level::Error || self.console.enabled(metadata)
    }

    fn log(&self, record: &Record<'_>) {
        self.console.log(record);
        if record.level() == Level::Error {
            self.otlp.log(record);
        }
    }

    fn flush(&self) {
        self.console.flush();
        self.otlp.flush();
    }
}

pub(super) fn init(service_name: &'static str, endpoint: Option<&str>) -> Option<SdkLoggerProvider> {
    let mut console = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    );
    let Some(endpoint) = endpoint else {
        console.init();
        return None;
    };

    let exporter = match LogExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.to_owned())
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
    let max_level = console.filter().max(LevelFilter::Error);
    log::set_boxed_logger(Box::new(ErrorExportLogger {
        console,
        otlp: Box::new(OpenTelemetryLogBridge::new(&provider)),
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
