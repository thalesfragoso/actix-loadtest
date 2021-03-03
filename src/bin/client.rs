use std::time::Duration;

use actix::io::SinkWrite;
use actix::prelude::*;
use actix_codec::Framed;
use anyhow::{anyhow, Result};
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::{BoxedSocket, Client};
use futures::stream::{SplitSink, StreamExt};
use structopt::*;
// use tokio::time::sleep;
use tokio::time::delay_for as sleep;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Disconnect;

pub struct WsClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl WsClient {
    pub async fn new() -> Result<Addr<Self>> {
        let (_response, framed) = Client::new()
            .ws("http://127.0.0.1:8080/ws/")
            .connect()
            .await
            .map_err(|e| anyhow!("Failed to connect to Server: {}", e))?;

        let (sink, stream) = framed.split();
        Ok(WsClient::create(move |ctx| {
            ctx.add_stream(stream);
            WsClient {
                writer: SinkWrite::new(sink, ctx),
            }
        }))
    }
}

impl Actor for WsClient {
    type Context = Context<Self>;
}

impl StreamHandler<Result<Frame, WsProtocolError>> for WsClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(Frame::Ping(msg)) => {
                self.writer.write(Message::Pong(msg));
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("Error while parsing WebSocket message: {}", e);
                ctx.stop();
            }
        }
    }
}

impl Handler<Disconnect> for WsClient {
    type Result = ();

    fn handle(&mut self, _msg: Disconnect, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

impl actix::io::WriteHandler<WsProtocolError> for WsClient {}

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
