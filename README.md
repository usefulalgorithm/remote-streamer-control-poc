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
cargo run --release --bin rscpoc-streamer http://remote-streamer-control-poc.herokuapp.com
```

You'll see stuff like this:

```
[2020-08-06T08:58:17Z INFO  rscpoc_streamer]
    ClientResponse HTTP/1.1 101 Switching Protocols
      headers:
        "date": "Thu, 06 Aug 2020 08:58:17 GMT"
        "via": "1.1 vegur"
        "upgrade": "websocket"
        "connection": "Upgrade"
        "sec-websocket-accept": // snipped
        "transfer-encoding": "chunked"

[2020-08-06T08:58:17Z INFO  rscpoc_streamer] Connected
[2020-08-06T08:58:44Z INFO  rscpoc_streamer] From Server: ID = 17998777093975623133
```

This means we've successfully registered a streamer to server, with `ID` = 17998777093975623133.

To send JSON commands to streamer, do this in another terminal / tmux window:

```
// List the IDs of registered streamers
curl http://remote-streamer-control-poc.herokuapp.com/list

// Send a command to a streamer
curl -X POST -H 'Content-Type: application/json' -d "{YOUR_COMMAND}" http://remote-streamer-control-poc.herokuapp.com/send/{STREAMER_ID}
```

Then observe at the terminal where you've launched the streamer program and you should see the command being successfully dispatched.

## TODO

1. Heroku native SSL support costs money, so currently only HTTP is supported. Could use Cloudflare to take care of this.
2. Need more sophisticated authentication method. Currently there's no way for the server to verify the validity of the client sending the command.
3. No syntax check is performed by the server on the incoming message. Without it wouldn't break anything but it's nice to have this check.
