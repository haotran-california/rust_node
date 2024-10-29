# Project Summary

This project is a tiny JavaScript runtime, directly inspired by Node.js. It is built in Rust and uses V8 Engine. 


# API

### `console.log(arg)`
### Parameters:
- `arg` (String|Number) The string to be printed to the console.

### `setTimeout(callback, delay)`
### `setInterval(callback, delay)`
### Parameters:
- `callback` (Function) The function to be executed repeatedly at each interval.
- `interval` (Number) The time, in milliseconds, between successive executions of the callback.

## `FS`
### `fs.readFile(path, callback)`
### `fs.writeFile(path, data, callback)`
### Parameters:
- `path` (String): The file path to be read.
- `data` (String): The content to write to the file.
- `callback` (Function): A function that will be executed once the file is read. The callback takes two arguments:
  - `error` (String|Null): If an error occurs, this will contain the error message. Otherwise, it will be `null`.
  - `data` (String): The content of the file, returned as a string.

## `HTTP`
### `http.createServer()`
  Returns (Object): `Server`

### `http.get(url, callback)`
### `http.request(options, callback)`
### Parameters:
- `url` (String): The address to which the HTTP request is directed
- `options` (Object): The configuration for an HTTP request containing the following properties: 
  - `hostname` (String): The server's domain or IP address 
  - `port` (Number): The port number to connect to on the server 
  - `path` (String): The endpoint on the server for the request 
  - `method` (String): The HTTP method to use 
  - `headers` (Object): Optional, an object of request headers, with each key as a header name 

### `SERVER`
### `server.listen(port, hostname)`
### Parameters:
  - `port` (Number): The port number to connect to on the server 
  - `hostname` (String): The server's domain or IP address 

### `REQUEST` 
### `req.headers()`
  Returns (HashMap<String, String>): 
### `req.method()`
  Returns (String): each entry consists of a header key pair value where the name of the header is the key and the value is the corresponding value
### `req.url()`
  Returns (String): the HTP method used for the request 
### `req.end()`

### `RESPONSE` 
### Methods: 
### `req.setHeader()`
### `req.statusCode()`
### `req.end()`

# Resources  
Deno
- [Deno Internals Book](https://choubey.gitbook.io/internals-of-deno)
- [Roll your own runtime](https://deno.com/blog/roll-your-own-javascript-runtime) 

YouTube
- [Create Your Own JavaScript Runtime, Eric Wendal](https://www.youtube.com/watch?v=ynNDmp7hBdo&t=1s)
- [How to use v8 isolates in Rust using rusty_v8](https://www.youtube.com/watch?v=ZzbmcQv-VJc&t=637s) 

Github
- [dune](https://github.com/aalykiot/dune)
- [runtime](https://github.com/novel-js/runtime)
- [dudeJs](https://github.com/ghost8395/dudeJS) 
- [learning-v8](https://github.com/danbev/learning-v8)

Docs
- [rusty_v8](https://docs.rs/rusty_v8/latest/rusty_v8/)

Personal
- YouTube Series 
- Medium
- Blog Posts
