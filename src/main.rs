use serde::{Serialize, Deserialize};
use std::time::Duration;
use sunny_db::timeseries_db::SunnyDB;
use tokio::sync::RwLock;
use tokio::signal;
use std::sync::Arc;
use tokio::time::interval;
use reqwest;
use anyhow::{self, Context};
use axum;
use clap::Parser;


#[derive(Parser, Debug)]
struct Args {
    // Granularity in seconds at which PowerData is fetched
    #[arg(short, long)]
    granularity: u64,

    // Address to which the server is bound
    #[arg(short, long, default_value_t = String::from("0.0.0.0:3000"))]
    bind: String,

    // Server address from which to fetch /status/powerflow
    #[arg(long)]
    url: String,

    // Path to database directory
    #[arg(long)]
    db_path: String,

    // Time series segment size
    #[arg(long, default_value_t = 100)]
    segment_size: usize,

    // Time series loss threshold: during graceful shutdown, data in memory is persisted
    // if there's more values than set via the threshold; this is to avoid cluttering the DB
    // with small segments; set to 0 to always store any data
    #[arg(long, default_value_t = 10)]
    loss_threshold: usize
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
struct PowerValues {
    power_pv: f64,
    power_grid: f64,
    power_used: f64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let sunny_db = SunnyDB::<PowerValues>::new(args.segment_size, &(&args.db_path), 2, args.loss_threshold);

    // create an RW lock that locks the entire DB during writes;
    // writes should be pretty fast so that should be fine as we can have multiple readers
    let db_write_lock = Arc::new(RwLock::new(sunny_db));
    let db_shutdown_lock = Arc::clone(&db_write_lock);
    let db_read_lock = Arc::clone(&db_write_lock);

    println!("Spawning database writer...");
    let granularity = Duration::from_secs(args.granularity);
    tokio::spawn(async move {
        fetch_and_write_values_to_db(&db_write_lock, granularity, args.url).await;
    });

    // launch the server

    // initialize tracing
    println!("Initializing server...");
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = axum::Router::new()
        // `GET /` goes to `root`
        .route("/", axum::routing::get(move || landing_page(db_read_lock)));
        // .with_state(db_read_lock);

    // run our app with hyper, listening globally on port
    // very useful: https://github.com/tokio-rs/axum/tree/main/examples
    let listener = tokio::net::TcpListener::bind(&(args.bind)).await.unwrap();
    println!("Listening on http://{}", args.bind);
    println!("Starting now! Everything looks fantastic! Enjoy!");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(db_shutdown_lock))
        .await.unwrap();
}

async fn fetch_and_write_values_to_db(
    db_lock: &RwLock<SunnyDB<PowerValues>>,
    granularity: Duration,
    url: String
) {
    let mut pause = interval(granularity);

    let full_url = format!("http://{}/status/powerflow", url.strip_suffix("/").unwrap_or(&url));
    loop {
        pause.tick().await;
        let values = fetch_power_values(&full_url).await;
        match values {
            Ok(v) => {
                let mut sunny_db = db_lock.write().await;
                sunny_db.insert_value_at_current_time(v);
            },
            Err(e) => println!("Error encountered while trying to fetch latest data: {}", e)
        }
    }
}

async fn fetch_power_values(url: &str) -> anyhow::Result<PowerValues> {
    let current_values = reqwest::get(url)
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    let site_data = &current_values["site"];

    let power_values = PowerValues {
        power_pv: site_data["P_PV"].as_f64().context("Couldn't obtain PV power from response")?,
        power_grid: site_data["P_Grid"].as_f64().context("Couldn't obtain grid power from response")?,
        power_used: site_data["P_Load"].as_f64().context("Couldn't obtain used power from response")?
    };

    Ok(power_values)
}

async fn landing_page(db_read_lock: Arc<RwLock<SunnyDB<PowerValues>>>) -> String {
    let mut header = "Hello from Sunny! The values currently being collected are shown below (refresh for update):\n\n".to_owned();

    let reader = db_read_lock.read().await;
    let current_values = reader.time_series.get_current_values();
    
    header.push_str(&(serde_json::to_string_pretty(&current_values).unwrap_or("".to_string())));
    header
}

async fn shutdown_signal(db_shutdown_lock: Arc<RwLock<SunnyDB<PowerValues>>>) {
    // from https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs <3

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    // flush the database
    let mut write_lock = db_shutdown_lock.write().await;
    write_lock.lossy_persist();
}
