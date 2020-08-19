use {
    actix::*,
    actix_web::{guard, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer},
    actix_web_actors::ws,
    bytes::Bytes,
    futures::channel::oneshot,
    log::{error, info, warn},
    messages::{
        ClientMessage, Connect, Disconnect, EncoderMessage, EncoderMessageType, GetSession, List,
    },
    serde_json::json,
    std::env,
    std::time::{Duration, Instant},
};

pub mod messages;
mod server;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const ENCODER_TIMEOUT: Duration = Duration::from_secs(10);

struct WsSession {
    /// unique session id
    id: usize,
    hb: Instant,
    addr: Addr<server::RemoteServer>,
    tx: Option<oneshot::Sender<i32>>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => {
                        info!("My id: {}", res);
                        act.id = res;
                    }
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify remote server
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<ClientMessage> for WsSession {
    type Result = ();
    fn handle(&mut self, msg: ClientMessage, ctx: &mut Self::Context) {
        self.tx = Some(msg.tx);
        let message = std::str::from_utf8(&msg.msg).unwrap().to_owned();
        ctx.text(serde_json::to_string(&EncoderMessage(EncoderMessageType::Cmd(message))).unwrap());
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    /// Handles websocket messages from Encoder
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Ok(msg) => msg,
            Err(_) => {
                ctx.stop();
                return;
            }
        };
        info!("WEBSOCKET MESSAGE: {:?}", msg);
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let msg: EncoderMessage = serde_json::from_str(text.as_str()).unwrap();
                let res = match msg.0 {
                    EncoderMessageType::ID(_) => Some(
                        serde_json::to_string(&EncoderMessage(EncoderMessageType::ID(Some(
                            self.id,
                        ))))
                        .unwrap(),
                    ),
                    EncoderMessageType::CmdRet(ret) => {
                        if let Some(tx) = self.tx.take() {
                            tx.send(ret).unwrap();
                        } else {
                            error!("No sender for this command");
                        }
                        // Don't send anything back to Encoder
                        None
                    }
                    _ => Some(text),
                };
                if let Some(res) = res {
                    ctx.text(res);
                }
            }
            ws::Message::Binary(bin) => ctx.binary(bin),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl WsSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > ENCODER_TIMEOUT {
                // timed out
                warn!("Encoder timed out!");
                act.addr.do_send(Disconnect { id: act.id });
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

async fn encoder_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::RemoteServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsSession {
            id: 0,
            hb: Instant::now(),
            addr: srv.get_ref().clone(),
            tx: None,
        },
        &req,
        stream,
    )
}

async fn list_route(srv: web::Data<Addr<server::RemoteServer>>) -> Result<HttpResponse, Error> {
    let addr = srv.get_ref().clone();
    let res = addr.send(List).await?;
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("{:?}\n", res)))
}

async fn send_route(
    target: web::Path<usize>,
    bytes: Bytes,
    srv: web::Data<Addr<server::RemoteServer>>,
) -> Result<HttpResponse, Error> {
    // we only check if content-type is application/json and don't care about the rest

    let mut results = None;
    let addr = srv.get_ref().clone();
    let (tx, rx) = oneshot::channel::<i32>();
    if let Some(session) = addr.send(GetSession(*target)).await? {
        session.send(ClientMessage { msg: bytes, tx: tx }).await?;
        results = Some(rx.await.unwrap());
    }
    match results {
        Some(res) => Ok(HttpResponse::Ok().content_type("text/plain").body(
            serde_json::to_string_pretty(&json!({
                "target": *target,
                "value": res,
            }))
            .unwrap(),
        )),
        None => Ok(HttpResponse::NotFound()
            .body(serde_json::to_string_pretty(&json!({"target": *target,})).unwrap())),
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=info,info");
    env_logger::init();
    let server = server::RemoteServer::default().start();
    // GetSession $PORT for HttpServer
    let port = env::var("PORT")
        .unwrap_or(String::from("8000"))
        .parse()
        .expect("PORT must be a number");

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Logger::new("%a %{User-Agent}i"))
            .data(server.clone())
            // encoders - websocket
            .service(web::resource("/ws/").to(encoder_route))
            // client - send json command
            .service(
                web::resource("/send/{target}").route(
                    web::post()
                        .guard(guard::Header("Content-Type", "application/json"))
                        .to(send_route),
                ),
            )
            // client - list encoders
            .service(web::resource("/list").route(web::get().to(list_route)))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
