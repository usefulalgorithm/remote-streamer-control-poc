use {
    actix::{io::SinkWrite, *},
    actix_codec::Framed,
    awc::{
        error::WsProtocolError,
        ws::{Codec, Frame, Message},
        BoxedSocket, Client,
    },
    bytes::Bytes,
    futures::stream::{SplitSink, StreamExt},
    log::{info, warn},
    messages::{EncoderMessage, EncoderMessageType},
    std::{env, time::Duration},
};

pub mod messages;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

fn main() {
    env::set_var("RUST_LOG", "actix_web=info,info");
    env_logger::init();
    let server_url = env::args()
        .nth(1)
        .unwrap_or("http://127.0.0.1:8080".to_string());

    let sys = System::new("rscpoc-encoder");
    Arbiter::spawn(async move {
        let (response, framed) = Client::new()
            .ws(format!("{}/ws/", server_url))
            .connect()
            .await
            .map_err(|e| {
                warn!("Error: {}", e);
            })
            .unwrap();
        info!("{:?}", response);
        let (sink, stream) = framed.split();
        let addr = Encoder::create(|ctx| {
            Encoder::add_stream(stream, ctx);
            Encoder(SinkWrite::new(sink, ctx))
        });
        addr.do_send(EncoderMessage(EncoderMessageType::ID(None)));
    });
    sys.run().unwrap();
}

struct Encoder(SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>);

impl Actor for Encoder {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.hb(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        info!("Disconnected");
        System::current().stop();
    }
}

impl Encoder {
    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_later(HEARTBEAT_INTERVAL, |act, ctx| {
            act.0.write(Message::Ping(Bytes::from_static(b""))).unwrap();
            act.hb(ctx);
        });
    }
}

impl Handler<EncoderMessage> for Encoder {
    type Result = ();

    fn handle(&mut self, msg: EncoderMessage, _: &mut Context<Self>) {
        self.0
            .write(Message::Text(serde_json::to_string(&msg).unwrap()))
            .unwrap();
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for Encoder {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Context<Self>) {
        if let Ok(Frame::Text(txt)) = msg {
            let msg = std::str::from_utf8(&txt).unwrap();
            if let Ok(EncoderMessage(msg_type)) = serde_json::from_str(&msg) {
                match msg_type {
                    EncoderMessageType::ID(Some(id)) => info!("My ID: {}", id),

                    EncoderMessageType::Cmd(cmd) => {
                        info!("Received command: {}", cmd);
                        // XXX Do stuff with cmd?
                        // FIXME this will do for now
                        // XXX What if the command never returns?
                        let return_value = cmd.len();
                        let addr = ctx.address();
                        let _ =
                            addr.do_send(EncoderMessage(EncoderMessageType::CmdRet(return_value)));
                    }
                    _ => warn!("Invalid EncoderMessageType: {:?}", msg_type),
                };
            } else {
                warn!("Unknown message: {:?}", msg);
            }
        }
    }

    fn started(&mut self, _: &mut Context<Self>) {
        info!("Connected");
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        info!("Server disconnected");
        ctx.stop()
    }
}

impl actix::io::WriteHandler<WsProtocolError> for Encoder {}
