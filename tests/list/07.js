http.get('http://localhost:8000/upload', (res) => {
    console.log("Node callback function has started")

    res.on('data', (chunk) => {
        console.log('Received chunk:', chunk);
    });

    res.on('end', () => {
        console.log('Response ended');
    });

})

// Note: res.end() cannot be called here
// res is of type http.IncomingMessage  