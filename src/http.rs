use rusty_v8 as v8;
use tokio::net::TcpStream;
use std::io::{Write, Read};
use std::str;

pub fn http_server_listen(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_value: v8::ReturnValue,
) {
    // Extract the port number from the first argument
    let port = 3000;
    if args.length() > 0 && args.get(0).is_number() {
        let port = args.get(0)
        .integer_value(scope)
        .unwrap() as u16
    } 

    // Extract the host from the second arguement
    let host = "127.0.0.1".to_string();
    if args.length() > 1 && args.get(1).is_string() {
        host = args.get(0)
        .to_rust_string_lossy(scope)
    }

    let tx_http = retrieve_tx_http(scope).unwrap(); // Assuming this function returns the channel sender

    tokio::task::spawn_local(async move {
        // Bind to the specified host and port
        let listener = match tokio::net::TcpListener::bind((host.as_str(), port)).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind to {}:{}", host, port);
                return;
            }
        };

        println!("Server is listening on port {}", port);


        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    if let Err(e) = tx_http.send(socket) {
                        eprintln!("Failed to send socket to main event loop: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                }
            }
        }
    });
}

Some(socket) = rx_http.recv() => {
    pending = true;
    let mut buf = [0; 1024];
    let n = socket.read(&mut buf).await.unwrap();
    let request_data = String::from_utf8_lossy(&buf[..n]).to_string();

    // Call the JS callback
    let response = call_js_request_callback(persistent_callback.clone(), request_data);

    // Send the response
    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        response.len(),
        response
    );
    socket.write_all(http_response.as_bytes()).await.unwrap();
}
