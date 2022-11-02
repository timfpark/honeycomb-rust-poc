use opentelemetry::sdk::trace as sdktrace;
use opentelemetry::trace::{TraceContextExt, TraceError, Tracer};
use opentelemetry::{global, Key};
use opentelemetry_otlp::WithExportConfig;
use std::env;
use tonic::metadata::MetadataMap;
use tonic::transport::ClientTlsConfig;
use tracing_subscriber::prelude::*;
use url::Url;

fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    let honeycomb_api_key = env::var("HONEYCOMB_API_KEY").expect("HONEYCOMB_API_KEY must be set");

    let mut metadata = MetadataMap::with_capacity(2);
    metadata.insert("x-honeycomb-team", honeycomb_api_key.parse().unwrap());
    metadata.insert("x-honeycomb-dataset", "my-api".parse().unwrap());

    let opentelemetry_endpoint =
        env::var("OTEL_ENDPOINT").unwrap_or_else(|_| "https://api.honeycomb.io".to_owned());

    let opentelemetry_endpoint =
        Url::parse(&opentelemetry_endpoint).expect("OTEL_ENDPOINT is not a valid url");

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(opentelemetry_endpoint.as_str())
                .with_metadata(metadata.clone())
                .with_tls_config(
                    ClientTlsConfig::new().domain_name(
                        opentelemetry_endpoint
                            .host_str()
                            .expect("OTEL_ENDPOINTshould have a valid host"),
                    ),
                ),
        )
        .install_batch(opentelemetry::runtime::Tokio)
}

#[tokio::main]
async fn main() {
    let tracer = init_tracer().expect("failed to instantiate opentelemetry tracing");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .expect("failed to register tracer with registry");

    const LEMONS_KEY: Key = Key::from_static_str("lemons");
    const ANOTHER_KEY: Key = Key::from_static_str("ex.com/another");

    let tracer = global::tracer("ex.com/basic");

    tracer.in_span("operation", |cx| {
        let span = cx.span();
        span.add_event(
            "Nice operation!".to_string(),
            vec![Key::new("bogons").i64(100)],
        );
        span.set_attribute(ANOTHER_KEY.string("yes"));

        tracer.in_span("Sub operation...", |cx| {
            let span = cx.span();
            span.set_attribute(LEMONS_KEY.string("five"));

            span.add_event("Sub span event", vec![]);
        });
    });

    loop {
        tracing::info!("just sleeping, press ctrl-c to exit");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
