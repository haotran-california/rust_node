const server = http.createServer((req, res) => {
  console.log('Received a Request!');
  console.log('Display Request fields');
  console.log(req.method());
  console.log(req.url());
  //Console does not support printing objects
  //console.log(req.headers());

  res.statusCode(200);
  res.setHeader('custom', 'Bearer-Token');
  res.setHeader('Content-Type', 'text/plain');
  res.end('Hello World\n');

});

server.listen(8000, '127.0.0.1');