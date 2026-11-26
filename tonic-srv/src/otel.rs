//! OTLP log and trace export for gRPC servers.

use opentelemetry::{
    propagation::{Extractor, TextMapPropagator},
    trace::TracerProvider as _,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    logs::SdkLoggerProvider,
    propagation::TraceContextPropagator,
    trace::{Sampler, SdkTracerProvider},
};
use server_conf::OtelConf;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{EnvFilter, layer::Layer, layer::SubscriberExt};

mod logging;

const GRPC_REQUEST_TARGET: &str = "grpc.request";

/// Owns the providers that must be flushed after the server drains.
pub struct Telemetry {
    logger: Option<SdkLoggerProvider>,
    tracer: Option<SdkTracerProvider>,
}

impl Telemetry {
    /// Flush and shut down the configured providers.
    pub async fn shutdown(self) {
        if let Some(provider) = self.logger {
            logging::shutdown(provider).await;
        }
        if let Some(provider) = self.tracer {
            shutdown_tracing(provider).await;
        }
    }
}

/// Initialize console logging and the OTLP signals enabled by the configuration.
/// Call once, before serving requests, from within a Tokio runtime.
pub fn init(service_name: &'static str, config: Option<&OtelConf>) -> Telemetry {
    init_with_request_target(service_name, config, GRPC_REQUEST_TARGET)
}

/// Initialize console logging and the configured OTLP signals for a custom request span target.
///
/// When trace export is enabled, add an INFO directive for `request_target` to the trace filter.
/// Call once, before serving requests, from within a Tokio runtime.
pub fn init_with_request_target(
    service_name: &'static str,
    config: Option<&OtelConf>,
    request_target: &'static str,
) -> Telemetry {
    let config = config.filter(|conf| !conf.endpoint.is_empty());
    let logger = logging::init(service_name, config.filter(|conf| conf.logs));
    let tracer = config.filter(|conf| conf.traces).and_then(|conf| {
        match init_tracing(service_name, &conf.endpoint, request_target) {
            Ok(provider) => Some(provider),
            Err(error) => {
                eprintln!("failed to initialize OpenTelemetry tracing: {error}");
                None
            }
        }
    });
    Telemetry { logger, tracer }
}

fn init_tracing(
    service_name: &'static str,
    endpoint: &str,
    request_target: &str,
) -> Result<SdkTracerProvider, Box<dyn std::error::Error + Send + Sync>> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.to_owned())
        .build()?;
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_sampler(Sampler::AlwaysOn)
        .with_resource(Resource::builder().with_service_name(service_name).build())
        .build();
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive(format!("{request_target}=info").parse()?);
    let layer = tracing_opentelemetry::layer()
        .with_tracer(provider.tracer(service_name))
        .with_filter(filter);
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer))?;
    Ok(provider)
}

/// Flush pending spans after the server drains.
async fn shutdown_tracing(provider: SdkTracerProvider) {
    match tokio::task::spawn_blocking(move || provider.shutdown()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => eprintln!("failed to shut down OpenTelemetry tracer provider: {error}"),
        Err(error) => eprintln!("OpenTelemetry tracer shutdown task failed: {error}"),
    }
}

/// Create the request span used by tonic's server builder.
pub fn request_span<B>(request: &tonic::codegen::http::Request<B>) -> tracing::Span {
    let span = tracing::info_span!(
        target: GRPC_REQUEST_TARGET,
        "grpc request",
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
    );

    if request.headers().contains_key("traceparent") {
        let propagator = TraceContextPropagator::new();
        let parent = propagator.extract(&HeaderExtractor(request.headers()));
        let _ = span.set_parent(parent);
    }

    span
}

struct HeaderExtractor<'a>(&'a tonic::codegen::http::HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|key| key.as_str()).collect()
    }
}
