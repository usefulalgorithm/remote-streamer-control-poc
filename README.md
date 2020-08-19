# remote-streamer-control-poc
A simple webserver / webclient proof-of-concept for remote streamer control

## Prerequisite

Make sure you have Rust installed, please visit:

https://www.rust-lang.org/tools/install

## Usage

The server is hosted on Heroku:

http://remote-streamer-control-poc.herokuapp.com/

To build, do:

```
cd {REPO_PATH}/
cargo run --release --bin rscpoc-encoder http://remote-streamer-control-poc.herokuapp.com
```

You'll see stuff like this:

```
[2020-08-06T08:58:17Z INFO  rscpoc_encoder]
    ClientResponse HTTP/1.1 101 Switching Protocols
      headers:
        "date": "Thu, 06 Aug 2020 08:58:17 GMT"
        "via": "1.1 vegur"
        "upgrade": "websocket"
        "connection": "Upgrade"
        "sec-websocket-accept": // snipped
        "transfer-encoding": "chunked"

[2020-08-06T08:58:17Z INFO  rscpoc_encoder] Connected
[2020-08-06T08:58:44Z INFO  rscpoc_encoder] From Server: ID = 17998777093975623133
```

This means we've successfully registered an encoder to server, with `ID` = 17998777093975623133.

To send JSON commands to encoder, do this in another terminal / tmux window:

```
// List the IDs of registered encoders
curl http://remote-streamer-control-poc.herokuapp.com/list

// Send a command to an encoder
curl -X POST -i -H 'Content-Type: application/json' -d "{YOUR_COMMAND}" http://remote-streamer-control-poc.herokuapp.com/send/{STREAMER_ID}
```

At the terminal where you fired up `curl`, you should see:
* If the specified encoder doesn't exist in server's database:
```
HTTP/1.1 404 Not Found
Server: Cowboy
Connection: keep-alive
Content-Length: 36
Date: Wed, 19 Aug 2020 07:22:59 GMT
Via: 1.1 vegur

{
  "target": 15147167111768808175
}
```
* If the encoder is registered:
```
HTTP/1.1 200 OK
Server: Cowboy
Connection: keep-alive
Content-Length: 50
Content-Type: text/plain
Date: Wed, 19 Aug 2020 07:33:24 GMT
Via: 1.1 vegur

{
  "target": 17998777093975623133,
  "value": ${LENGTH_OF_YOUR_COMMAND}
}
```

At the terminal where you've launched `rscpoc-encoder`, you should see:
```
[2020-08-19T07:33:24Z INFO  rscpoc_encoder] Received command: ${YOUR_COMMAND}
```

## TODO

1. Heroku native SSL support costs money, so currently only HTTP is supported. Could use Cloudflare to take care of this.
2. Need more sophisticated authentication method. Currently there's no way for the server to verify the validity of the client sending the command.
3. No syntax check is performed by the server on the incoming message. Without it wouldn't break anything but it's nice to have this check.
