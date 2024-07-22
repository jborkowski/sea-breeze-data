use std::sync::{Arc, Mutex};

use chrono::Local;
use chrono_tz::Europe::Madrid;
use clap::{Parser, Subcommand};
use std::time::Duration;

use tokio::time::interval;
use warp::{http::StatusCode, Filter};
use windfinder::{WindData, WindFinder};

use crate::windfinder::Data;

mod windfinder;

#[derive(Debug, Parser)]
#[command(name = "sea")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Now,
    Server,
}

type SharedWindData = Arc<Mutex<Option<WindData>>>;

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    let wf = WindFinder::new();

    let url = "https://www.windfinder.com/forecast/els_poblets_valencia_spain";

    let data: Arc<Mutex<Option<WindData>>> = Arc::new(Mutex::new(None));

    let data_clone = data.clone();

    tokio::spawn(async move {
        let mut interval = interval(Duration::new(3600, 0));

        loop {
            interval.tick().await;

            let scraped_data = wf.scrape(url).unwrap();

            let mut data_lock = data_clone.lock().unwrap();

            println!("Scrapped {:?}", scraped_data);

            *data_lock = Some(scraped_data);
        }
    });

    match args.command {
        Commands::Now => {
            let now = Local::now().with_timezone(&Madrid);
            let data = data.lock().unwrap().clone();
            let result = data.map(|data| data.for_date(now).expect("There is no data").clone());
            println!("{:?}", result);
        }
        Commands::Server => {
            let data = data.clone();

            println!("Listening on 0.0.0.0:3000");

            let handler = warp::path::end()
                .and(with_data(data))
                .and_then(handle_request);

            warp::serve(handler).run(([0, 0, 0, 0], 3000)).await;
        }
    }
}

async fn handle_request(data: SharedWindData) -> Result<impl warp::Reply, warp::Rejection> {
    let data_lock = data.lock().unwrap();
    let now = Local::now().with_timezone(&Madrid);

    use warp::reply::{json, with_status};
    match &*data_lock {
        Some(data) => match data.for_date(now) {
            Some(data) => Ok(with_status(json(&data), StatusCode::OK)),
            None => Ok(with_status(
                json(&Option::None::<Data>),
                StatusCode::NO_CONTENT,
            )),
        },

        None => {
            println!("OO Tak");
            Ok(with_status(
                json(&Option::None::<Data>),
                StatusCode::NOT_FOUND,
            ))
        }
    }
}

fn with_data(
    data: SharedWindData,
) -> impl Filter<Extract = (SharedWindData,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || data.clone())
}
