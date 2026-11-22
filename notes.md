# Request lifecycle

- server listener receives request
- send to handler
- match against registered routes
    - if matched, call handler
        - if Ok, end connection, kill thread
        - if Err, call error handler
- return 404