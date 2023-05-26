use std::error::Error;
use std::io::prelude::*;
use csv::{ReaderBuilder};
//use serde::ser::StdError;
use serde::Deserialize;
use std::io::Cursor;

use std::fs::File;
//use std::path::Path;
use reqwest;
use tokio::task::JoinHandle;

use gtfs_structures::Error as GtfsError;

use std::io::{Read, Write};
//use std::net::TcpStream;
use std::fs::copy;

#[feature(async_await)]
use futures::future::{join_all};

#[derive(Debug, Deserialize, Clone)]
struct Agency {
    agency: String,
    url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("./gtfs_schedules.csv");
    let mut contents = String::new();
    file.expect("read zip file failed!").read_to_string(&mut contents);

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(contents.as_bytes());

    let mut agencies = Vec::new();
    for result in reader.deserialize() {
        let record: Agency = result?;
        agencies.push(record);
    }

    // Iterate over the paths.
    let mut tasks: Vec<JoinHandle<Result<(), ()>>>= vec![];

    let firstagencies = agencies.clone();

    for agency in firstagencies {
        // Copy each path into a new string
    // that can be consumed/captured by the task closure
    let path = agency.url.clone();

    // Create a Tokio task for each path
    tasks.push(tokio::spawn(async move {
        match reqwest::get(&path).await {
            Ok(resp) => {
                match resp.bytes().await {
                    Ok(text) => {
                        println!("RESPONSE: {} KB from {}", text.len()/1000, path);

                        //create folder if not exists 
                        std::fs::create_dir_all("./gtfs_schedules").expect("create folder failed!");

                        //save to file

                        let mut file = File::create(format!("./gtfs_schedules/{}.zip", agency.agency)).expect("create file failed!");

                         // Copy the response body into the file
                         let mut content =  Cursor::new(text);
                        std::io::copy(&mut content, &mut file);

                        println!("save to file: {}", format!("./gtfs_schedules/{}.zip", agency.agency));
                    }
                    Err(_) => println!("ERROR reading {}", path),
                }
            }
            Err(_) => println!("ERROR downloading {}", path),
        }
        Ok(())
    }));
    }

    // Wait for them all to finish
    println!("Started {} tasks. Waiting...", tasks.len());
    join_all(tasks).await;

    for agency in agencies {
        println!("v2 agency: {}, url: {}", agency.agency, agency.url);

        let gtfs = gtfs_structures::Gtfs::from_path(format!("./gtfs_schedules/{}.zip", agency.agency))?;

        println!("Read duration read_duration: {:?}", gtfs.read_duration);

        println!("there are {} stops in the gtfs", gtfs.stops.len());

        println!("there are {} routes in the gtfs", gtfs.routes.len());
    }

    Ok(())
}