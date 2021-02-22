use actix::io::SinkWrite;
use actix::prelude::*;
use actix_codec::Framed;
use anyhow::{anyhow, Result};
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::{BoxedSocket, Client};
use futures::stream::{SplitSink, StreamExt};

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
