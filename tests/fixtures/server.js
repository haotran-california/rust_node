const http = require("http");

const host = 'localhost';
const port = 8000;

const books = JSON.stringify([
    { title: "The Alchemist", author: "Paulo Coelho", year: 1988 },
    { title: "The Prophet", author: "Kahlil Gibran", year: 1923 }
]);

const authors = JSON.stringify([
    { name: "Paulo Coelho", countryOfBirth: "Brazil", yearOfBirth: 1947 },
    { name: "Kahlil Gibran", countryOfBirth: "Lebanon", yearOfBirth: 1883 }
]);

const requestListener = function (req, res) {
    res.setHeader("Content-Type", "application/json");
    switch (req.url) {
        case "/test": 
            for(let i = 0; i < 3; i++){
                let msg = `This is chunk number ${i}`
                res.write(msg)
            }
            break
        case "/books":
            res.writeHead(200);
            res.end(books);
            break
        case "/authors":
            res.writeHead(200);
            res.end(authors);
            break
        case "/upload":
            if (req.method === 'POST') {
                let body = '';

                // Collect data chunks
                req.on('data', chunk => {
                    body += chunk.toString();
                });

                // When all data is received
                req.on('end', () => {
                    console.log("Received data:", body);

                    // Respond to the client
                    res.writeHead(200);
                    res.end(JSON.stringify({ message: "Data received successfully!", receivedData: body }));
                });
            } else {
                res.writeHead(405); // Method Not Allowed if not a POST request
                res.end(JSON.stringify({ message: "Only POST requests are allowed on this endpoint." }));
            }
            break;

        default:
            res.writeHead(404); // Not Found for any other routes
            res.end(JSON.stringify({ message: "Route not found." }));
            break;
    }
}

const server = http.createServer(requestListener);
server.listen(port, host, () => {
    console.log(`Server is running on http://${host}:${port}`);
});