use std::time::Duration;

use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use anyhow::Result;
use structopt::*;
// use tokio::time::sleep;
use tokio::time::delay_for as sleep;

mod client;
mod server;

use client::{Disconnect, WsClient};
use server::MyWebSocket;

/// do websocket handshake and start `MyWebSocket` actor
async fn ws_index(r: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(MyWebSocket::new(), &r, stream)
}

#[derive(StructOpt)]
#[structopt(name = "load", about = "A load generator to test actix Websockets")]
struct CLIArgs {
    #[structopt(short = "n", long = "number", default_value = "200")]
    /// Number of clients to generate per round
    number_clients: usize,
    #[structopt(short = "r", long = "rounds", default_value = "10")]
    /// Number of rounds
    number_rounds: usize,
}

#[actix_web::main]
async fn main() -> Result<()> {
    let opt = CLIArgs::from_args();
    pretty_env_logger::init_timed();

    actix_web::rt::spawn(async move {
        HttpServer::new(|| {
            App::new()
                // enable logger
                .wrap(middleware::Logger::default())
                // websocket route
                .service(web::resource("/ws/").route(web::get().to(ws_index)))
        })
        // start http server on 127.0.0.1:8080
        .bind("127.0.0.1:8080")
        .unwrap()
        .run()
        .await
        .unwrap();
    });
    sleep(Duration::from_secs(1)).await;

    let n = opt.number_clients;
    log::info!("Starting stress test with {} clients per round", n);

    for round in 0..opt.number_rounds {
        log::info!("Round {}: Connecting clients", round);
        do_round(n).await?;
        log::info!("Finished, resting for 5s");
        sleep(Duration::from_secs(5)).await;
    }

    log::info!("Finished all rounds, Ctrl-C to exit");
    tokio::signal::ctrl_c().await?;
    log::info!("Ctrl-C received, shutting down");
    actix::System::current().stop();
    Ok(())
}

async fn do_round(n: usize) -> Result<()> {
    let mut clients = Vec::with_capacity(n);

    for _ in 0..n {
        clients.push(WsClient::new().await?);
    }

    log::info!("Disconnecting");
    for addr in clients.into_iter() {
        addr.do_send(Disconnect);
    }
    Ok(())
}
