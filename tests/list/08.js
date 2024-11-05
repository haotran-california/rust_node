const options = {
  hostname: 'localhost',
  port: 8000,
  path: '/upload',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
};

const req = http.request(options, (res) => {
  console.log("hello");
});

req.end();
