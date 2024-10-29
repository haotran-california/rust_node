const options = {
  hostname: 'localhost',
  port: 8000,
  path: '/upload',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
};

const req = request(options, (res) => {
  console.log('STATUS: ' + res.statusCode);
  console.log('HEADERS:' + JSON.stringify(res.headers));
  console.log("hello");
});

req.end();
