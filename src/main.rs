use {
    actix::*,
    actix_web::{guard, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer},
    actix_web_actors::ws,
    bytes::Bytes,
    log::{info, warn},
    messages::{ClientMessage, Connect, Disconnect, List, Message},
    std::env,
    std::time::{Duration, Instant},
};

pub mod messages;
mod server;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const ENCODER_TIMEOUT: Duration = Duration::from_secs(10);

struct WsStreamerSession {
    /// unique session id
    id: usize,
    hb: Instant,
    addr: Addr<server::RemoteServer>,
}

impl Actor for WsStreamerSession {
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

impl Handler<Message> for WsStreamerSession {
    type Result = ();
    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsStreamerSession {
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
            ws::Message::Text(text) => ctx.text(if let "/ID" = text.as_str() {
                format!("ID = {}", self.id.to_string())
            } else {
                text
            }),
            ws::Message::Binary(bin) => ctx.binary(bin),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl WsStreamerSession {
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
        WsStreamerSession {
            id: 0,
            hb: Instant::now(),
            addr: srv.get_ref().clone(),
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

    let addr = srv.get_ref().clone();
    let res = addr
        .send(ClientMessage {
            msg: bytes,
            target: *target,
        })
        .await?;
    match res {
        Some(target) => Ok(HttpResponse::Ok()
            .content_type("text/plain")
            .body(format!("Successfully sent to id={}\n", target))),
        None => {
            Ok(HttpResponse::NotFound().body(format!("Cannot find encoder with id={}\n", target)))
        }
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=info,info");
    env_logger::init();
    let server = server::RemoteServer::default().start();
    // Get $PORT for HttpServer
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
