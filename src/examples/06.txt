const server = createServer((req, res) => {
  console.log('Received a request!');
  console.log(req.method());
  console.log(req.url());

  res.statusCode(200);
  //res.setHeader('Content-Type', 'application/json');
  //res.end(JSON.stringify({ message: 'Success!' }));
  res.setHeader('custom', 'Bearer-Token');
  res.setHeader('Content-Type', 'text/plain');
  res.end('Hello World\n');

});

server.listen(8000, '127.0.0.1');