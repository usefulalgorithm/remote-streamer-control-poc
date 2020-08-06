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
    std::{env, time::Duration},
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

fn main() {
    env::set_var("RUST_LOG", "actix_web=info,info");
    env_logger::init();
    let server_url = env::args().nth(1).unwrap_or("http://127.0.0.1:8080".to_string());

    let sys = System::new("rscpoc-streamer");
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
        let addr = Streamer::create(|ctx| {
            Streamer::add_stream(stream, ctx);
            Streamer(SinkWrite::new(sink, ctx))
        });
        addr.do_send(StreamerCmd::new(StreamerCmdTypes::ID));
    });
    sys.run().unwrap();
}

struct Streamer(SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>);

#[derive(Debug)]
enum StreamerCmdTypes {
    ID,
}

#[derive(Message)]
#[rtype(result = "()")]
struct StreamerCmd(String);

impl StreamerCmd {
    fn new(cmd_type: StreamerCmdTypes) -> Self {
        StreamerCmd(format!("/{:?}", cmd_type))
    }
}

impl Actor for Streamer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.hb(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        info!("Disconnected");
        System::current().stop();
    }
}

impl Streamer {
    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_later(HEARTBEAT_INTERVAL, |act, ctx| {
            act.0.write(Message::Ping(Bytes::from_static(b""))).unwrap();
            act.hb(ctx);
        });
    }
}

impl Handler<StreamerCmd> for Streamer {
    type Result = ();

    fn handle(&mut self, msg: StreamerCmd, _: &mut Context<Self>) {
        self.0.write(Message::Text(msg.0)).unwrap();
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for Streamer {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, _: &mut Context<Self>) {
        if let Ok(Frame::Text(txt)) = msg {
            info!("From Server: {}", std::str::from_utf8(&txt).unwrap())
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

impl actix::io::WriteHandler<WsProtocolError> for Streamer {}
