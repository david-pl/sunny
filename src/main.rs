use anyhow::{self, Context};
use axum::{
    self,
    extract::Path,
    http::Method,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bitcode::{Decode, Encode};
use clap::Parser;
use reqwest;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Sub};
use std::sync::Arc;
use std::time::Duration;
use sunny_db::statistics::*;
use sunny_db::timeseries::TimeSeries;
use sunny_db::timeseries_db::SunnyDB;
use tokio::signal;
use tokio::sync::RwLock;
use tokio::time::interval;
use tower_http::{cors::{Any, CorsLayer}, services::ServeDir};
use tower_http::services::ServeFile;

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
    sunny_home: String,

    // Time series segment size
    #[arg(long, default_value_t = 100)]
    segment_size: usize,

    // Time series loss threshold: during graceful shutdown, data in memory is persisted
    // if there's more values than set via the threshold; this is to avoid cluttering the DB
    // with small segments; set to 0 to always store any data
    #[arg(long, default_value_t = 10)]
    loss_threshold: usize,
}

#[derive(Copy, Clone, Encode, Decode, PartialEq, Serialize, Deserialize, Debug)]
struct PowerValues {
    power_pv: f64,
    power_to_grid: f64,
    power_from_grid: f64,
    power_used: f64,
}

// traits required to do statistics
impl Add for PowerValues {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            power_pv: self.power_pv + other.power_pv,
            power_to_grid: self.power_to_grid + other.power_to_grid,
            power_from_grid: self.power_from_grid + other.power_from_grid,
            power_used: self.power_used + other.power_used,
        }
    }
}

impl Sub for PowerValues {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            power_pv: self.power_pv - other.power_pv,
            power_to_grid: self.power_to_grid - other.power_to_grid,
            power_from_grid: self.power_from_grid - other.power_from_grid,
            power_used: self.power_used - other.power_used,
        }
    }
}

impl Mul<f64> for PowerValues {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self {
            power_pv: self.power_pv * rhs,
            power_to_grid: self.power_to_grid * rhs,
            power_from_grid: self.power_from_grid * rhs,
            power_used: self.power_used * rhs,
        }
    }
}

impl Div<f64> for PowerValues {
    type Output = Self;

    fn div(self, rhs: f64) -> Self {
        Self {
            power_pv: self.power_pv / rhs,
            power_to_grid: self.power_to_grid / rhs,
            power_from_grid: self.power_from_grid / rhs,
            power_used: self.power_used / rhs,
        }
    }
}

/// Simple wrapper around Arc<RwLock> to make it read-only
/// see also: https://stackoverflow.com/questions/70470631/getting-a-read-only-version-of-an-arcrwlockfoo
#[derive(Clone)]
struct DatabaseReadLock {
    lock: Arc<RwLock<SunnyDB<PowerValues>>>,
}

impl DatabaseReadLock {
    fn new(lock: Arc<RwLock<SunnyDB<PowerValues>>>) -> Self {
        DatabaseReadLock { lock: lock }
    }

    async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, SunnyDB<PowerValues>> {
        self.lock.read().await
    }
}

// Error handling -- see https://github.com/tokio-rs/axum/blob/main/examples/anyhow-error-response/src/main.rs
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let sunny_home = args.sunny_home;
    let sunny_path = if sunny_home.ends_with("/") {
        sunny_home
    } else {
        sunny_home + "/"
    };
    let db_path = sunny_path.to_owned() + "db";
    let sunny_db =
        SunnyDB::<PowerValues>::new(args.segment_size, &db_path, 2, args.loss_threshold);

    // create an RW lock that locks the entire DB during writes;
    // writes should be pretty fast so that should be fine as we can have multiple readers
    let db_write_lock = Arc::new(RwLock::new(sunny_db));
    let db_shutdown_lock = Arc::clone(&db_write_lock);
    let db_read_lock_1 = DatabaseReadLock::new(Arc::clone(&db_write_lock));
    let db_read_lock_2 = db_read_lock_1.clone();
    let db_read_lock_3 = db_read_lock_1.clone();

    println!("Spawning database writer...");
    let granularity = Duration::from_secs(args.granularity);
    tokio::spawn(async move {
        fetch_and_write_values_to_db(&db_write_lock, granularity, args.url).await;
    });

    // launch the server

    // initialize tracing
    println!("Initializing server...");
    tracing_subscriber::fmt::init();

    // cors layer
    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(Any);

    let index_route = sunny_path.to_owned() + "index.html";
    let assets_route = sunny_path + "assets/";

    // build our application with a route
    let app = axum::Router::new()
        // `GET /` goes to `root`
        .route_service(
            "/",
            ServeFile::new(index_route),
        )
        .layer(cors.clone())
        .nest_service("/assets",
            ServeDir::new(assets_route)
        )
        .layer(cors.clone())
        .route(
            "/values/:start_time/:end_time",
            axum::routing::get(move |Path((start_time, end_time)): Path<(u64, u64)>| {
                get_values_in_time_range(db_read_lock_2, Path((start_time, end_time)))
            }),
        )
        .layer(cors.clone())
        .route(
            "/values-with-stats/:start_time/:end_time",
            axum::routing::get(move |Path((start_time, end_time)): Path<(u64, u64)>| {
                get_values_in_time_range_with_statistics(
                    db_read_lock_3,
                    Path((start_time, end_time)),
                )
            }),
        )
        .layer(cors.clone());

    // run our app with hyper, listening globally on port
    // very useful: https://github.com/tokio-rs/axum/tree/main/examples
    let listener = tokio::net::TcpListener::bind(&(args.bind)).await.unwrap();
    println!("Listening on http://{}", args.bind);
    println!("Starting now! Everything looks fantastic! Enjoy!");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(db_shutdown_lock))
        .await
        .unwrap();
}

async fn fetch_and_write_values_to_db(
    db_lock: &RwLock<SunnyDB<PowerValues>>,
    granularity: Duration,
    url: String,
) {
    let mut pause = interval(granularity);

    let full_url = format!(
        "http://{}/status/powerflow",
        url.strip_suffix("/").unwrap_or(&url)
    );
    loop {
        pause.tick().await;
        let values = fetch_power_values(&full_url).await;
        match values {
            Ok(v) => {
                let mut sunny_db = db_lock.write().await;
                sunny_db.insert_value_at_current_time(v);
            }
            Err(e) => println!("Error encountered while trying to fetch latest data: {}", e),
        }
    }
}

async fn fetch_power_values(url: &str) -> anyhow::Result<PowerValues> {
    let current_values = reqwest::get(url).await?.json::<serde_json::Value>().await?;
    let site_data = &current_values["site"];

    // convert some power values from negative to all positive values
    // this is especially important for the grid values since they can be positive and negative,
    // which would be annoying to deal with when computing e.g. the total amount of energy used,
    // which would correspond to an integral over only the positive parts;
    // so, we're splitting the values in two here

    // the grid power value is negative if we're feeding into to the grid and positive if we're pulling from it
    let grid_power = site_data["P_Grid"]
        .as_f64()
        .context("Couldn't obtain grid power from response")?;
    let (power_to_grid, power_from_grid) = if grid_power < 0.0 {
        (-grid_power, 0.0)
    } else {
        (0.0, grid_power)
    };

    // power load can only be negative, but still, let's work with positives only
    let power_load = site_data["P_Load"]
        .as_f64()
        .context("Couldn't obtain used power from response")?;
    let power_used = -power_load;

    let power_values = PowerValues {
        power_pv: site_data["P_PV"]
            .as_f64()
            .context("Couldn't obtain PV power from response")?,
        power_from_grid: power_from_grid,
        power_to_grid: power_to_grid,
        power_used: power_used,
    };

    Ok(power_values)
}

async fn get_values_in_time_range(
    db_read_lock: DatabaseReadLock,
    Path((start_time, end_time)): Path<(u64, u64)>,
) -> Result<String, AppError> {
    let reader = db_read_lock.read().await;

    let read_timeseries = reader.get_values_in_range(start_time, end_time);
    match read_timeseries {
        Some(series) => Ok(serde_json::to_string_pretty(&series.get_current_values())?),
        None => Ok(String::from("{ }")),
    }
}

#[derive(Serialize)]
struct ValuesAndStats {
    values: Vec<(u64, PowerValues)>,
    average: Option<PowerValues>,
    maxes: Option<PowerValues>,
    energy_kwh: Option<PowerValues>,
}

async fn get_values_in_time_range_with_statistics(
    db_read_lock: DatabaseReadLock,
    Path((start_time, end_time)): Path<(u64, u64)>,
) -> Result<String, AppError> {
    let reader = db_read_lock.read().await;
    let read_timeseries = reader.get_values_in_range(start_time, end_time);

    if read_timeseries.is_none() {
        return Ok(String::from("{ }"));
    }

    let timeseries = read_timeseries.unwrap();

    // time is in ms so the integral over the series comes out in units of W*ms = mJ
    let integral = timeseries.integrate();
    let energy_joule = integral.map(|e| e * 1e-3);
    let energy_kwh = energy_joule.map(|e| e * 1e-3 / 3600.0);
    let avg = integral.map(|e| {
        e / (timeseries.get_end_time().unwrap() - timeseries.get_start_time().unwrap()) as f64
    });
    let maxes = get_max_powervalues_from_series(&timeseries);

    let response_data = ValuesAndStats {
        values: timeseries.get_current_values(),
        average: avg,
        maxes: maxes,
        energy_kwh: energy_kwh,
    };

    let json = serde_json::to_string(&response_data);
    Ok(json?)
}

fn get_max_powervalues_from_series(timeseries: &TimeSeries<PowerValues>) -> Option<PowerValues> {
    let pv_max = timeseries
        .max_by(|a, b| a.power_pv.partial_cmp(&b.power_pv).unwrap())?
        .power_pv;
    let grid_max = timeseries
        .max_by(|a, b| a.power_from_grid.partial_cmp(&b.power_from_grid).unwrap())?
        .power_from_grid;
    let into_grid_max = timeseries
        .max_by(|a, b| a.power_to_grid.partial_cmp(&b.power_to_grid).unwrap())?
        .power_to_grid;
    let used_max = timeseries
        .max_by(|a, b| a.power_used.partial_cmp(&b.power_used).unwrap())?
        .power_used;

    let pv = PowerValues {
        power_pv: pv_max,
        power_to_grid: into_grid_max,
        power_from_grid: grid_max,
        power_used: used_max,
    };
    Some(pv)
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
