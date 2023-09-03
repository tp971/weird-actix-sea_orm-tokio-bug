# Weird actix / sea-orm / tokio bug

This is an attempt at reproducing a weird bug in at least one of actix web, sea-orm or tokio.
The program starts a background task that periodically inserts a row into a database,
then starts an `actix_web::HttpServer`,
and after the server stops, the background task is notified and quits.

For some reason, sometimes after terminating the server with SIGTERM or SIGINT (aka Ctrl+C),
the background task fails with one of the following behaviours, depending on the version of sea-orm and the Cargo profile:

```
sea-orm 0.11.3 (sqlx 0.6.3) / debug:
    [2023-09-03T01:33:31Z ERROR example] error doing stuff: Execution Error: error communicating with database: A Tokio 1.x context was found, but it is being shutdown.

sea-orm 0.11.3 (sqlx 0.6.3) / release:
    [2023-09-03T01:36:17Z ERROR example] error doing stuff: Execution Error: error communicating with database: A Tokio 1.x context was found, but it is being shutdown.

sea-orm 0.12.2 (sqlx 0.7.1) / debug:
    thread 'main' panicked at 'assertion failed: `(left == right)`
      left: `0`,
     right: `1`', /home/.../.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.32.0/src/runtime/io/scheduled_io.rs:220:9

sea-orm 0.12.2 (sqlx 0.7.1) / release:
    hangs on "inserting", must be killed via SIGKILL
```

## Reproducing

To reproduce:
1. install MySQL, MariaDB or PostgreSQL and create a user and database,
2. clone this repository,
3. set `DATABASE_URL` in `src/main.db`,
4. `cargo run` or `cargo run --release`,
5. execute an HTTP query, e.g. with `curl http://127.0.0.1:8080/test`,
6. stop the server (either SIGTERM via `kill` or SIGINT via Ctrl+C).

The bug does not seem to occur deterministically.
On my machine, it works best with the following timing:

1. start the server,
2. on the 2nd insert, send the HTTP request,
3. on the 4th insert, stop the server.
