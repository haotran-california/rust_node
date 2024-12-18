// use rusty_v8 as v8;
// use tokio;
// use std::collections::HashMap;
// use std::ffi::c_void;
// use tokio::net::TcpStream;
// use tokio::io::AsyncWriteExt;
// use tokio::io::AsyncReadExt;

// extern crate httparse;

// //Import V8 Callback Functions


// //Channel Object Types
// use crate::interface::Operations; 
// use crate::interface::HttpOperation; 



// pub async fn send_response(stream: &mut TcpStream, response: &mut Response) {
//     println!("Within send response");
//     // Write the status line
//     let status_line = format!("HTTP/1.1 {} OK\r\n", response.status_code);
//     stream.write_all(status_line.as_bytes()).await.unwrap();

//     // Write an empty line to separate headers and body
//     stream.write_all(b"\r\n").await.unwrap();

//     // Write the body in chunks (like Node.js res.write())
//     stream.write_all(response.body.as_bytes()).await.unwrap();

//     // Close the connection (like Node.js res.end())
//     stream.flush().await.unwrap();
//     stream.shutdown().await.unwrap();
// }

// pub fn parse_http_request(raw_data: &[u8]) -> Option<Request> {
//     let mut headers = [httparse::EMPTY_HEADER; 16]; // A fixed size buffer for headers
//     let mut req = httparse::Request::new(&mut headers);

//     match req.parse(raw_data) {
//         Ok(status) => {
//             if status.is_partial() {
//                 println!("Partial HTTP request. Waiting for more data.");
//                 return None;
//             }

//             // Extract the method, path, and headers
//             let method = req.method.unwrap_or("").to_string();
//             let url = req.path.unwrap_or("").to_string();

//             let headers = req.headers.iter().fold(HashMap::new(), |mut map, header| {
//                 map.insert(header.name.to_string(), std::str::from_utf8(header.value).unwrap_or("").to_string());
//                 map
//             });

//             // If there is a Content-Length header, use it to read the body
//             let body_offset = status.unwrap();
//             let body = if let Some(content_length) = headers.get("Content-Length") {
//                 let length: usize = content_length.parse().unwrap_or(0);
//                 if body_offset + length <= raw_data.len() {
//                     Some(std::str::from_utf8(&raw_data[body_offset..(body_offset + length)]).unwrap_or("").to_string())
//                 } else {
//                     None // Incomplete body
//                 }
//             } else {
//                 None // No body
//             };

//             Some(Request{
//                 method,
//                 url,
//                 headers,
//                 body: body.unwrap_or("".to_string()),
//             })
//         }
//         Err(e) => {
//             println!("Error parsing HTTP request: {}", e);
//             None
//         }
//     }
// }

// pub async fn handle_http_request(mut socket: tokio::net::TcpStream) {
//     let mut buf = [0; 1024];  // A buffer for reading data in chunks
//     let mut request_data = String::new();  // A string to store the full request data

//     // Read data from the socket in a loop
//     loop {
//         let n = socket.read(&mut buf).await.unwrap();
        
//         if n == 0 {
//             // If no more data is read, break out of the loop
//             break;
//         }

//         // Append the chunk of data to the request_data string
//         request_data.push_str(&String::from_utf8_lossy(&buf[..n]));

//         // Check if the request has been fully received (indicated by two CRLFs)
//         if request_data.contains("\r\n\r\n") {
//             break;
//         }
//     }

//     // At this point, request_data contains the full request headers + body (if present)
//     println!("Received request:\n{}", request_data);

//     // Parse the request (you can write a parser or use a library for this)
//     let request = parse_http_request(request_data.as_bytes());

//     // Now we can create a Response
//     let mut headers = HashMap::new();
//     headers.insert("Content-Type".to_string(), "text/plain".to_string());

//     let res = Response {
//         status_code: 200,
//         headers,
//         body: "Hello from Rust HTTP server!".to_string()
//     };

//     // Send the response back to the client
//     //send_response(socket, res).await;
// }